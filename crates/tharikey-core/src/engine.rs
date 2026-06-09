//! The engine.
//!
//! Two paths, one model:
//! - [`transliterate`] / [`reverse`] — one-shot `text` schemes (Malé Latin), used by the
//!   transliteration feature and radheef Latin search. Drives the **abugida processor**, which
//!   references abstract roles (`vowel_carrier`, `default_coda`) from the scheme — never hardcoded
//!   Thaana literals.
//! - [`Session`] — a stateful `keys` scheme (Phonetic, Typewriter) for the IME: 1:1 per key, plus
//!   optional toggleable sequence rules (e.g. `aa → aabaafili`), with unit-wise backspace.

use crate::matcher::Matcher;
use crate::scheme::{Atom, Emit, Layer, RewriteRule, Scheme, TextScheme, Transform};
use crate::util::{invert, is_thaana, pop_chars, starts_with};
use std::collections::HashSet;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Text path: abugida composition + reverse
// ---------------------------------------------------------------------------

/// Transliterate Latin `input` to Thaana using a `text` scheme.
///
/// The loop is a plain dispatch: classify each token (consonant / vowel / literal) and hand it to the
/// `Composer`, which owns *all* coda + word-final logic. Word-boundary handling (gemination,
/// the final-glottal letter, glide codas) lives in one place — see `Composer::flush`.
pub fn transliterate(scheme: &Scheme, input: &str) -> String {
    // Composition only applies to text schemes; a keys scheme has no romanization to compose.
    let Scheme::Text(t) = scheme else {
        return input.to_string();
    };
    let ab = &t.abugida;
    let prenasal: Vec<char> = ab.prenasal_marker.chars().collect();
    let matcher = Matcher::new(t.text_tokens());
    // Romanizations are case-insensitive (case is orthographic, not phonemic) — fold the input so
    // sentence-capitalised text works (e.g. Áletinu "Hurihá" / "Emíhun", capital Ð/Þ → ð/þ).
    let lowered = input.to_lowercase();
    let chars: Vec<char> = lowered.chars().collect();

    // Precompute each coda rule's token as chars ONCE per call — apply_coda runs at every input
    // position, so collecting `r.tok.chars()` inside it would re-allocate per rule per position.
    let coda_toks: Vec<Vec<char>> = t
        .coda
        .rules
        .iter()
        .map(|r| r.tok.chars().collect())
        .collect();

    let mut p = Composer::new(ab);
    let mut i = 0;
    while i < chars.len() {
        // prenasalisation: the marker (e.g. ') drops the pending consonant's coda -> hus-noonu (ނޑ)
        if !prenasal.is_empty() && starts_with(&chars[i..], &prenasal) {
            p.prenasal_drop();
            i += prenasal.len();
            continue;
        }
        // coda rule pass: the ordered, first-match-wins context rules (the former glide / final-vowel /
        // final-glottal / medial-glottal branches, now data — see scheme::CodaRule). Runs before the
        // abugida dispatch so a rule token (`iy`, a word-final/medial `h`) is claimed in context rather
        // than read as its bare vowel/consonant.
        if let Some(consumed) = p.apply_coda(&chars[i..], t, &matcher, &coda_toks) {
            i += consumed;
            continue;
        }
        if let Some((tok, len)) = matcher.longest(&chars[i..]) {
            if let Some(base) = t.consonants.get(&tok) {
                p.push_consonant(base);
            } else if let Some(fili) = t.vowel_output(&tok) {
                p.push_vowel(&tok, fili);
            } else if let Some(lit) = t.literals.get(&tok) {
                p.push_literal(lit);
            }
            i += len;
        } else {
            p.push_passthrough(chars[i]); // unknown char = boundary
            i += 1;
        }
    }
    p.flush(); // end of input is a boundary
    apply_rewrites(p.out, &t.rewrites) // optional post-composition rewrite layer (`rewrites`)
}

/// Apply the `rewrites` post-composition layer: each rule, in declared order, does one left-to-right
/// pass over the composed Thaana, replacing `match` with `out` wherever the `pre`/`post` context holds
/// (contexts are matched, not consumed). Each rule sees the previous rule's output — an FST-inspired
/// correction cascade (run as rewrite passes, not a compiled state machine). Mirrors the offline miner's
/// `apply_all`/`apply_one` exactly, so the engine reproduces its measured accuracy. No-op (and
/// allocation-free) when a scheme carries no rules — the hand-curated baseline.
fn apply_rewrites(input: String, rules: &[RewriteRule]) -> String {
    if rules.is_empty() {
        return input;
    }
    let mut chars: Vec<char> = input.chars().collect();
    for rule in rules {
        let m: Vec<char> = rule.match_.chars().collect();
        if m.is_empty() {
            continue;
        }
        let pre: Vec<char> = rule.pre.chars().collect();
        let post: Vec<char> = rule.post.chars().collect();
        let out: Vec<char> = rule.out.chars().collect();
        let mut res: Vec<char> = Vec::with_capacity(chars.len());
        let mut i = 0;
        while i < chars.len() {
            let hit = i + m.len() <= chars.len()
                && chars[i..i + m.len()] == m[..]
                && i >= pre.len()
                && chars[i - pre.len()..i] == pre[..]
                && i + m.len() + post.len() <= chars.len()
                && chars[i + m.len()..i + m.len() + post.len()] == post[..];
            if hit {
                res.extend_from_slice(&out);
                i += m.len();
            } else {
                res.push(chars[i]);
                i += 1;
            }
        }
        chars = res;
    }
    chars.into_iter().collect()
}

/// Abugida composition state + the role config it references. Centralising it here means the
/// `transliterate` loop is a flat dispatch and every coda / word-final decision (gemination, the
/// final-glottal letter, glide codas) has exactly one home.
struct Composer<'a> {
    ab: &'a crate::scheme::Abugida,
    out: String,
    awaiting_coda: bool, // a consonant is pending with no following vowel yet
    prev_base: Option<String>, // the pending consonant's output base (for gemination)
    last_vowel: Option<String>, // most-recent vowel token (selects the glottal letter / pre-context)
}

impl<'a> Composer<'a> {
    fn new(ab: &'a crate::scheme::Abugida) -> Self {
        Self {
            ab,
            out: String::new(),
            awaiting_coda: false,
            prev_base: None,
            last_vowel: None,
        }
    }

    /// Emit a consonant `base`: a doubled consonant becomes the geminate marker; otherwise any pending
    /// consonant first takes its coda (sukun). `last_vowel` is preserved so a following coda rule can
    /// still see the nucleus vowel (e.g. `rah` -> the `a` of `ra`).
    fn push_consonant(&mut self, base: &str) {
        let geminating = self.awaiting_coda
            && !self.ab.geminate.is_empty()
            && self.prev_base.as_deref() == Some(base);
        if geminating {
            pop_chars(&mut self.out, base.chars().count()); // drop the pending first copy for the geminate marker
            let nasal = self.ab.nasals.iter().any(|n| n == base);
            let marker = if nasal && !self.ab.geminate_nasal.is_empty() {
                &self.ab.geminate_nasal
            } else {
                &self.ab.geminate
            };
            self.out.push_str(marker);
        } else if self.awaiting_coda {
            self.out.push_str(&self.ab.default_coda); // pending consonant takes its pure-killer (sukun)
        }
        self.out.push_str(base);
        self.awaiting_coda = true;
        self.prev_base = Some(base.to_string());
    }

    /// Emit a dependent vowel `fili` (from `tok`): attaches to a pending consonant, else a standalone
    /// vowel = carrier + fili. Records `tok` as the nucleus for a possible coda rule.
    fn push_vowel(&mut self, tok: &str, fili: &str) {
        if !self.awaiting_coda {
            self.out.push_str(&self.ab.vowel_carrier); // standalone vowel = carrier + fili
        }
        self.out.push_str(fili);
        self.awaiting_coda = false;
        self.prev_base = None;
        self.last_vowel = Some(tok.to_string());
    }

    /// Flush a pending consonant to the literal `lit` boundary (punctuation, glottal stop, …).
    fn push_literal(&mut self, lit: &str) {
        self.flush();
        self.out.push_str(lit);
        self.reset_pending();
    }

    /// Pass an unknown char through verbatim — itself a word boundary, so flush first.
    fn push_passthrough(&mut self, ch: char) {
        self.flush();
        self.out.push(ch);
        self.reset_pending();
    }

    /// Prenasalisation: drop the pending consonant's coda (hus-noonu) without emitting it. `last_vowel`
    /// is preserved — the marker sits mid-cluster, so the nucleus still belongs to a later glottal.
    fn prenasal_drop(&mut self) {
        self.awaiting_coda = false;
        self.prev_base = None;
    }

    /// Give a pending word-final consonant its coda (sukun). The glottal/glide/marker swaps are coda
    /// rules now (applied in the main pass before the abugida dispatch), so flush is just the default
    /// coda. Idempotent when nothing is pending, so it is safe to call at every boundary.
    fn flush(&mut self) {
        if self.awaiting_coda {
            self.out.push_str(&self.ab.default_coda);
            self.awaiting_coda = false;
        }
    }

    /// Clear pending-consonant + nucleus state after a boundary, glide, or literal.
    fn reset_pending(&mut self) {
        self.prev_base = None;
        self.last_vowel = None;
    }

    /// Ordered coda-rule pass: the first rule whose token is a literal prefix here and whose `pre`/`suf`
    /// context holds fires and is emitted. Returns the chars consumed (the matched token's length), or
    /// `None` if no rule applies (the caller then runs the normal abugida dispatch).
    fn apply_coda(
        &mut self,
        rest: &[char],
        t: &TextScheme,
        matcher: &Matcher,
        coda_toks: &[Vec<char>],
    ) -> Option<usize> {
        for (r, tok) in t.coda.rules.iter().zip(coda_toks) {
            if tok.is_empty() || !starts_with(rest, tok) {
                continue;
            }
            let after = &rest[tok.len()..];
            if !self.pre_ok(&r.pre) {
                continue;
            }
            if !self.suf_ok(&r.suf, after, t, matcher) {
                continue;
            }
            self.emit_coda(&r.emit, t);
            return Some(tok.len());
        }
        None
    }

    /// Left context (composer state), checked against the first atom (coda is single-position; empty =
    /// no constraint). `vowel` = a preceding vowel nucleus; `consonant` = a consonant is pending; a
    /// literal/set = the preceding vowel token is that / in that set. `boundary` is not meaningful here.
    fn pre_ok(&self, atoms: &[Atom]) -> bool {
        let Some(atom) = atoms.first() else {
            return true;
        };
        match atom {
            Atom::Word(w) if w == "vowel" => !self.awaiting_coda && self.last_vowel.is_some(),
            Atom::Word(w) if w == "consonant" => self.awaiting_coda,
            Atom::Word(w) if w == "boundary" => false,
            Atom::Word(lit) => {
                !self.awaiting_coda && self.last_vowel.as_deref() == Some(lit.as_str())
            }
            Atom::Set(set) => {
                !self.awaiting_coda && self.last_vowel.as_ref().is_some_and(|v| set.contains(v))
            }
        }
    }

    /// Right context: lookahead at the next token. `boundary` = end of input / a non-letter; `vowel` /
    /// `consonant` resolve against the scheme's `[vowels]` / `[consonants]` keys; a literal/set = the next
    /// token is that / in that set. Empty = no constraint.
    fn suf_ok(&self, atoms: &[Atom], after: &[char], t: &TextScheme, matcher: &Matcher) -> bool {
        let Some(atom) = atoms.first() else {
            return true;
        };
        match atom {
            Atom::Word(w) if w == "boundary" => after.first().is_none_or(|ch| !ch.is_alphabetic()),
            Atom::Word(w) if w == "vowel" => {
                matches!(matcher.longest(after), Some((tok, _)) if t.vowels.contains_key(&tok))
            }
            Atom::Word(w) if w == "consonant" => {
                matches!(matcher.longest(after), Some((tok, _)) if t.consonants.contains_key(&tok))
            }
            Atom::Word(lit) => matches!(matcher.longest(after), Some((tok, _)) if &tok == lit),
            Atom::Set(set) => {
                matches!(matcher.longest(after), Some((tok, _)) if set.contains(&tok))
            }
        }
    }

    /// Emit a matched coda rule, referencing abugida roles (Lipika-style):
    /// - `Lookup(table)` — a letter from `[lookups][table]` indexed by the preceding vowel (else
    ///   `vowel_carrier`) + coda.
    /// - `Glide(letter)` — coda consonant `letter` + coda.
    /// - `Fili(tok)` — attach that vowel token's fili to the pending consonant (`-ny → ނީ`).
    /// - `Marker` — the geminate marker `އް` (medial glottal: `huhdha → ހުއްދަ`).
    fn emit_coda(&mut self, emit: &Emit, t: &TextScheme) {
        match emit {
            Emit::Lookup(table) => {
                let letter = self
                    .last_vowel
                    .as_ref()
                    .and_then(|v| t.lookups.get(table).and_then(|m| m.get(v)))
                    .cloned()
                    .unwrap_or_else(|| self.ab.vowel_carrier.clone());
                self.out.push_str(&letter);
                self.out.push_str(&self.ab.default_coda);
                self.awaiting_coda = false;
                self.reset_pending();
            }
            Emit::Glide(letter) => {
                self.out.push_str(letter);
                self.out.push_str(&self.ab.default_coda);
                self.awaiting_coda = false;
                self.reset_pending();
            }
            Emit::Fili(vowel_tok) => {
                if let Some(fili) = t.vowel_output(vowel_tok) {
                    self.out.push_str(fili);
                }
                self.awaiting_coda = false;
                self.prev_base = None;
                self.last_vowel = Some(vowel_tok.clone());
            }
            Emit::Marker => {
                self.out.push_str(&self.ab.geminate);
                self.awaiting_coda = false;
                self.reset_pending();
            }
        }
    }
}

/// Reverse-transliterate Thaana back to Latin for a `reversible` `text` scheme.
/// Returns `None` if the scheme is not reversible.
///
/// **Contract — reverse is not the exact inverse of forward.** The word-final glottal rules
/// (`Composer::flush` / `glide_coda`) are deliberately many-to-one (`rah` & `rash` both → `ރަށް`),
/// so these schemes are flagged `lossy`. Reverse *is* glottal-aware — a word-final `ށް`/`ތް`/`އް` comes
/// back as the glottal romanisation, not the bare consonant (`ރަށް → "rah"`, `ރަތް → "raiy"`, `ކޮށް → "koh"`)
/// — but it can only pick one reading, so `rash → ރަށް → "rah"` does not round-trip. Reverse stays a
/// stable, deterministic Thaana→Latin reading.
pub fn reverse(scheme: &Scheme, input: &str) -> Option<String> {
    let Scheme::Text(t) = scheme else {
        return None; // keys schemes are not reversed
    };
    if !t.meta.reversible {
        return None;
    }
    let ab = &t.abugida;
    let carrier: Vec<char> = ab.vowel_carrier.chars().collect();
    let coda: Vec<char> = ab.default_coda.chars().collect();
    let geminate: Vec<char> = ab.geminate.chars().collect();
    let geminate_nasal: Vec<char> = ab.geminate_nasal.chars().collect();
    let nasals: std::collections::HashSet<String> = ab.nasals.iter().cloned().collect();
    let coda_rules = &t.coda;
    // Reverse maps are derived from the forward rules. The glottal romanisation (word-final `އް` -> this,
    // e.g. "h") is the `tok` of the first `Emit::Lookup` coda rule; glide letters read back as the `tok`
    // of their `Emit::Glide` rule. Fallback: schemes whose glottal is a context-free `[literal]` (Áletinu
    // `q`, ISO 15919 `ʾ`) have no coda rule — read it back from the literal that outputs the marker `އް`.
    let final_glottal_latin = coda_rules
        .rules
        .iter()
        .find(|r| matches!(r.emit, Emit::Lookup(_)))
        .map(|r| r.tok.clone())
        .or_else(|| {
            t.literals
                .iter()
                .find(|&(_, v)| v == &ab.geminate)
                .map(|(k, _)| k.clone())
        });
    let prenasal_marker = ab.prenasal_marker.clone();

    // Word-final glottal letters read back as the glottal romanisation, NOT the bare consonant: forward
    // emits ށ/ތ for a word-final glottal (alifu އ is covered by the gemination branch below), so reverse
    // must undo that — `ރަށް → "rah"`, not "rash". Glide letters (ތ) read back as their glide spelling.
    let glottal_rev: std::collections::HashMap<String, String> = {
        let mut m = std::collections::HashMap::new();
        // each `Emit::Lookup(table)` rule: the table's letters read back as that rule's `tok`.
        for r in &coda_rules.rules {
            if let Emit::Lookup(table) = &r.emit {
                if let Some(tbl) = t.lookups.get(table) {
                    for letter in tbl.values() {
                        if letter != &ab.vowel_carrier {
                            m.entry(letter.clone()).or_insert_with(|| r.tok.clone());
                        }
                    }
                }
            }
        }
        for r in &coda_rules.rules {
            if let Emit::Glide(letter) = &r.emit {
                m.insert(letter.clone(), r.tok.clone()); // glide spelling wins (ތ -> "iy")
            }
        }
        m
    };

    let rev_exclude: HashSet<String> = t.meta.reverse_exclude.iter().cloned().collect();
    let inv_cons = invert(&t.consonants, &rev_exclude);
    let inv_vowel = invert(&t.vowels, &rev_exclude);
    let inv_literal = invert(&t.literals, &rev_exclude); // ، -> ,  q -> އް  etc.

    let chars: Vec<char> = input.chars().collect();
    let mut out = String::new();
    let mut i = 0;

    while i < chars.len() {
        // gemination marker (alifu+sukun) + consonant -> the consonant, doubled. We emit one extra
        // copy of the consonant's latin here and skip only the marker; the consonant itself is then
        // processed normally below (emitting its second copy + any vowel).
        if !geminate.is_empty() && starts_with(&chars[i..], &geminate) {
            if let Some(lc) = chars
                .get(i + geminate.len())
                .and_then(|c| inv_cons.get(&c.to_string()))
            {
                out.push_str(lc); // gemination: extra copy of the doubled consonant
            } else if let Some(g) = &final_glottal_latin {
                out.push_str(g); // word-final glottal stop -> its romanisation (e.g. "h")
            }
            i += geminate.len();
            continue;
        }
        // nasal gemination (noonu+sukun + nasal) -> doubled. Best-effort: a literal noonu-coda before
        // a nasal is ambiguous with this and will be read as gemination.
        if !geminate_nasal.is_empty() && starts_with(&chars[i..], &geminate_nasal) {
            if let Some(lc) = chars
                .get(i + geminate_nasal.len())
                .map(|c| c.to_string())
                .filter(|c| nasals.contains(c))
                .and_then(|c| inv_cons.get(&c))
            {
                out.push_str(lc);
                i += geminate_nasal.len();
                continue;
            }
            // not a geminate -> fall through to normal handling (noonu as a consonant + coda)
        }

        // standalone vowel: carrier + fili
        if starts_with(&chars[i..], &carrier) {
            let cn = carrier.len();
            if let Some(fili) = chars.get(i + cn) {
                if let Some(latin) = inv_vowel.get(&fili.to_string()) {
                    out.push_str(latin);
                    i += cn + 1;
                    continue;
                }
            }
            i += cn; // bare carrier — skip
            continue;
        }

        let c = chars[i].to_string();
        // word-final glottal letter (ށ/ތ) + sukun at a word boundary -> glottal romanisation, undoing
        // the forward final-glottal/glide swap. "Word-final" = the char after the sukun is not Thaana
        // (end / space / punctuation). Mid-word ށ/ތ + sukun stays the consonant reading.
        if let Some(rom) = glottal_rev.get(&c) {
            if starts_with(&chars[i + 1..], &coda) {
                let after = i + 1 + coda.len();
                if chars.get(after).is_none_or(|&ch| !is_thaana(ch)) {
                    // dedup a shared nucleus vowel: if `out` already ends with the spelling's first
                    // char (e.g. ި "i" before a ތ glide), drop it so ހިތް -> "hiy", not "hiiy".
                    let skip = rom
                        .chars()
                        .next()
                        .filter(|&first| out.ends_with(first))
                        .map_or(0, char::len_utf8);
                    out.push_str(&rom[skip..]);
                    i = after;
                    continue;
                }
            }
        }
        if let Some(latin_c) = inv_cons.get(&c) {
            out.push_str(latin_c);
            // following mark: fili (vowel) or coda (bare consonant)
            if let Some(next) = chars.get(i + 1) {
                let n = next.to_string();
                if starts_with(&chars[i + 1..], &coda) {
                    i += 1 + coda.len();
                    continue;
                }
                if let Some(latin_v) = inv_vowel.get(&n) {
                    out.push_str(latin_v);
                    i += 2;
                    continue;
                }
            }
            i += 1;
            // bare consonant directly before another consonant = prenasalisation -> re-insert the marker
            if !prenasal_marker.is_empty() {
                if let Some(nx) = chars.get(i) {
                    if inv_cons.contains_key(&nx.to_string()) {
                        out.push_str(&prenasal_marker);
                    }
                }
            }
            continue;
        }

        // literals: invert back to Latin (، -> ,  ؛ -> ;  ؟ -> ?  …)
        if let Some(latin) = inv_literal.get(&c) {
            out.push_str(latin);
            i += 1;
            continue;
        }

        // unknown: pass through
        out.push(chars[i]);
        i += 1;
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Keys path: stateful session
// ---------------------------------------------------------------------------

/// A key press resolved to its physical key label (US-QWERTY) and modifier layer.
#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub key: String,
    pub layer: Layer,
}

impl KeyEvent {
    pub fn new(key: impl Into<String>, layer: Layer) -> Self {
        Self {
            key: key.into(),
            layer,
        }
    }
}

/// What the IME should do with a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// Append `text`.
    Insert(String),
    /// Delete the last `delete_units` units, then insert `text`. Two callers, two unit conventions:
    /// the stateful [`Session::feed`] counts *committed history units*; the context-aware
    /// [`Session::resolve`] counts *trailing Unicode scalars of the supplied document prefix* (the
    /// host already read that text, so it maps the count to its own UTF-16 range exactly).
    Replace { delete_units: usize, text: String },
    /// This key *could* reach back (it's the final token of an active sequence rule), so the engine
    /// needs the live document context to decide. The host should read the text before the caret and
    /// call [`Session::resolve`]. Returned only by [`Session::peek`]. This is the lazy, per-key
    /// shrink of Keyman's `set_context_if_needed`: the costly context read happens only here, never
    /// on ordinary keys.
    NeedsContext,
    /// Not mapped — let the key fall through to the app unchanged.
    Passthrough,
}

#[derive(Debug, Clone)]
struct Unit {
    keys: Vec<String>,
    out: String,
}

/// A configured keys-path output transform + its runtime state (see [`crate::scheme::Transform`]).
enum ActiveTransform {
    /// A stateless 1:1 substitution (bracket-flip mirror map, rufiyaa `$`→`⃂`, …).
    Substitute(std::collections::HashMap<String, String>),
    /// Smart quotes — stateful: alternates open/close per `"` and `'`.
    SmartQuotes {
        double: Option<(String, String)>,
        single: Option<(String, String)>,
        double_open: bool,
        single_open: bool,
    },
}

impl ActiveTransform {
    /// Transform one key's output (mutates smart-quote alternation state).
    fn apply(&mut self, s: String) -> String {
        match self {
            ActiveTransform::Substitute(map) => map.get(&s).cloned().unwrap_or(s),
            ActiveTransform::SmartQuotes {
                double,
                single,
                double_open,
                single_open,
            } => {
                if s == "\"" {
                    if let Some((open, close)) = double {
                        let out = if *double_open { open } else { close }.clone();
                        *double_open = !*double_open;
                        return out;
                    }
                } else if s == "'" {
                    if let Some((open, close)) = single {
                        let out = if *single_open { open } else { close }.clone();
                        *single_open = !*single_open;
                        return out;
                    }
                }
                s
            }
        }
    }
    /// Reset per-session state (smart-quote alternation back to "next is opening").
    fn reset(&mut self) {
        if let ActiveTransform::SmartQuotes {
            double_open,
            single_open,
            ..
        } = self
        {
            *double_open = true;
            *single_open = true;
        }
    }
}

/// A named transform slot with its on/off state (the runtime IME toggle).
struct TransformSlot {
    name: String,
    enabled: bool,
    tf: ActiveTransform,
}

/// Stateful processor for a `keys` scheme. Owns its scheme (via `Arc`) so it has no lifetime — which
/// keeps it clean to expose across FFI bindings (wasm-bindgen, PyO3, UniFFI all want owned handles).
pub struct Session {
    scheme: Arc<Scheme>,
    enabled: HashSet<String>,
    history: Vec<Unit>,
    output: String,
    /// Keys-path output transforms (`[transforms]`), sorted by name for deterministic application.
    transforms: Vec<TransformSlot>,
}

impl Session {
    /// Create a session for a scheme. Accepts an owned `Scheme` or an `Arc<Scheme>` (cheap to share
    /// across sessions).
    pub fn new(scheme: impl Into<Arc<Scheme>>) -> Self {
        let scheme: Arc<Scheme> = scheme.into();
        // A session is a keys-scheme driver; a text scheme yields an inert (passthrough) session.
        let keys = scheme.as_keys();
        // optional rules start in their `default` state; non-optional rules are always active
        let enabled = keys
            .map(|k| {
                k.sequences
                    .iter()
                    .filter(|r| !r.optional || r.default)
                    .map(|r| r.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        // Configured output transforms, with their on/off default from the scheme. Sorted by name so
        // application order is deterministic (transforms are disjoint by key, so order is cosmetic).
        let mut transforms: Vec<TransformSlot> = keys
            .map(|k| {
                k.transforms
                    .iter()
                    .map(|(name, t)| {
                        let (enabled, tf) = match t {
                            Transform::SmartQuotes {
                                enabled,
                                double,
                                single,
                                ..
                            } => (
                                *enabled,
                                ActiveTransform::SmartQuotes {
                                    double: double.clone().map(|p| (p.open, p.close)),
                                    single: single.clone().map(|p| (p.open, p.close)),
                                    double_open: true,
                                    single_open: true,
                                },
                            ),
                            Transform::Substitute { enabled, map, .. } => {
                                (*enabled, ActiveTransform::Substitute(map.clone()))
                            }
                        };
                        TransformSlot {
                            name: name.clone(),
                            enabled,
                            tf,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        transforms.sort_by(|a, b| a.name.cmp(&b.name));
        Self {
            scheme,
            enabled,
            history: Vec::new(),
            output: String::new(),
            transforms,
        }
    }

    /// Turn an optional rule on/off (e.g. the `aa → aabaafili` toggle).
    pub fn set_rule(&mut self, name: &str, on: bool) {
        if on {
            self.enabled.insert(name.to_string());
        } else {
            self.enabled.remove(name);
        }
    }

    /// Turn a named output transform on/off (`[transforms.<name>]`). The generic toggle the IME exposes.
    pub fn set_transform(&mut self, name: &str, on: bool) {
        for slot in &mut self.transforms {
            if slot.name == name {
                slot.enabled = on;
            }
        }
    }

    /// Set a toggleable **option** by name, whether it's a transform or an optional rule (the host gets
    /// the option list from [`Scheme::options`] and doesn't need to know which kind it is).
    pub fn set_option(&mut self, name: &str, on: bool) {
        if self.transforms.iter().any(|s| s.name == name) {
            self.set_transform(name, on);
        } else {
            self.set_rule(name, on);
        }
    }

    /// Enable/disable smart (curly) quotes — convenience over the conventional `smart_quotes` transform.
    pub fn set_smart_quotes(&mut self, on: bool) {
        self.set_transform("smart_quotes", on);
    }

    /// Enable/disable RTL bracket-key flip — convenience over the conventional `bracket_flip` transform.
    pub fn set_bracket_flip(&mut self, on: bool) {
        self.set_transform("bracket_flip", on);
    }

    /// The committed output so far.
    pub fn output(&self) -> &str {
        &self.output
    }

    pub fn reset(&mut self) {
        self.history.clear();
        self.output.clear();
        for slot in &mut self.transforms {
            slot.tf.reset();
        }
    }

    /// Feed one key. Mutates committed output and returns the IME action.
    pub fn feed(&mut self, ev: KeyEvent) -> Response {
        let scheme = self.scheme.clone(); // cheap Arc clone; frees `self` for mutation below
        let Some(keys) = scheme.as_keys() else {
            return Response::Passthrough; // text scheme: inert session
        };
        let raw = match keys.layer_map(ev.layer).get(&ev.key) {
            Some(o) if !o.is_empty() => o.clone(),
            _ => return Response::Passthrough,
        };

        // Sequence rules take precedence (they match on keys, not output). The longest active rule
        // whose final token is this key and whose preceding tokens match the tail of history wins.
        if let Some((seq, emit)) = self.match_sequence_rule(&ev.key) {
            let retract = seq.len() - 1;
            for _ in 0..retract {
                if let Some(u) = self.history.pop() {
                    pop_chars(&mut self.output, u.out.chars().count());
                }
            }
            self.output.push_str(&emit);
            self.history.push(Unit {
                keys: seq,
                out: emit.clone(),
            });
            return Response::Replace {
                delete_units: retract,
                text: emit,
            };
        }

        let out_str = self.apply_transforms(raw);
        self.output.push_str(&out_str);
        self.history.push(Unit {
            keys: vec![ev.key],
            out: out_str.clone(),
        });
        Response::Insert(out_str)
    }

    /// Apply every enabled output transform to a key's raw output, in order (mutates smart-quote
    /// open/close alternation). Shared by [`Session::feed`] and the context-aware [`Session::peek`] /
    /// [`Session::resolve`].
    fn apply_transforms(&mut self, mut s: String) -> String {
        for slot in &mut self.transforms {
            if slot.enabled {
                s = slot.tf.apply(s);
            }
        }
        s
    }

    /// **Phase 1 of the context-aware (direct-keyboard) path.** Resolve a key *without* the document:
    /// `Passthrough` if unmapped; `NeedsContext` if it could reach back (defer to [`resolve`] with the
    /// live prefix); otherwise the finished `Insert`. Holds no committed buffer, so nothing can go
    /// stale on a mid-word click — unlike [`feed`], which trusts `history`.
    ///
    /// [`resolve`]: Session::resolve
    /// [`feed`]: Session::feed
    pub fn peek(&mut self, ev: KeyEvent) -> Response {
        let scheme = self.scheme.clone();
        let Some(keys) = scheme.as_keys() else {
            return Response::Passthrough; // text scheme: inert session
        };
        let raw = match keys.layer_map(ev.layer).get(&ev.key) {
            Some(o) if !o.is_empty() => o.clone(),
            _ => return Response::Passthrough,
        };
        if self.is_reachback_candidate(&ev.key) {
            return Response::NeedsContext; // host reads context, then calls resolve
        }
        Response::Insert(self.apply_transforms(raw))
    }

    /// **Phase 2 of the context-aware path.** Given the live document `prefix` (the Thaana immediately
    /// before the caret), match the active reach-back rules against it: on a hit, `Replace` the
    /// matched trailing scalars with the rule's emit (the `aa → ާ` double-tap, matched against the real
    /// `ަ` in the document — so it fires regardless of how the caret got there); otherwise a fresh
    /// `Insert`. `delete_units` here counts **Unicode scalars** of `prefix`, not history units.
    pub fn resolve(&mut self, ev: KeyEvent, prefix: &str) -> Response {
        let scheme = self.scheme.clone();
        let Some(keys) = scheme.as_keys() else {
            return Response::Passthrough;
        };
        let raw = match keys.layer_map(ev.layer).get(&ev.key) {
            Some(o) if !o.is_empty() => o.clone(),
            _ => return Response::Passthrough,
        };
        if let Some((delete_units, text)) = self.match_context_rule(&ev.key, prefix) {
            return Response::Replace { delete_units, text };
        }
        Response::Insert(self.apply_transforms(raw))
    }

    /// Could `key` reach back? True iff some active sequence rule ends with `key` and has a lookback
    /// prefix (len > 1). The cheap check [`peek`] uses to decide whether a context read is worth it.
    ///
    /// [`peek`]: Session::peek
    fn is_reachback_candidate(&self, key: &str) -> bool {
        self.scheme.as_keys().is_some_and(|k| {
            k.sequences
                .iter()
                .filter(|r| r.is_active(&self.enabled))
                .any(|r| {
                    let seq = r.match_tokens();
                    seq.len() > 1 && seq.last().map(String::as_str) == Some(key)
                })
        })
    }

    /// Match active reach-back rules against the document `prefix` (output-side, not key history):
    /// each rule's lookback keys are projected to their base-layer Thaana output and compared to the
    /// tail of `prefix`. On a match, return `(scalars_to_delete, emit)`, preferring the longest
    /// lookback. Returns `None` when nothing matches (the caller then inserts fresh).
    fn match_context_rule(&self, key: &str, prefix: &str) -> Option<(usize, String)> {
        let keys = self.scheme.as_keys()?;
        let prefix_chars: Vec<char> = prefix.chars().collect();
        let mut best: Option<(usize, String)> = None;
        for r in keys.sequences.iter().filter(|r| r.is_active(&self.enabled)) {
            let seq = r.match_tokens();
            if seq.len() < 2 || seq.last().map(String::as_str) != Some(key) {
                continue;
            }
            // The expected document tail = each lookback key's base-layer output, concatenated. (Base
            // layer: the reach-back niceties are vowel doublings, all typed on the base layer.)
            let mut expected = String::new();
            let mut resolvable = true;
            for lookback_key in &seq[..seq.len() - 1] {
                match keys.layers.base.get(lookback_key) {
                    Some(o) if !o.is_empty() => expected.push_str(o),
                    _ => {
                        resolvable = false;
                        break;
                    }
                }
            }
            if !resolvable {
                continue;
            }
            let exp_chars: Vec<char> = expected.chars().collect();
            let hit = exp_chars.len() <= prefix_chars.len()
                && prefix_chars[prefix_chars.len() - exp_chars.len()..] == exp_chars[..];
            if hit && best.as_ref().is_none_or(|(d, _)| exp_chars.len() > *d) {
                best = Some((exp_chars.len(), r.emit.clone()));
            }
        }
        best
    }

    /// Delete the last composed unit (akuru+fili), not a raw codepoint.
    pub fn backspace(&mut self) -> Response {
        match self.history.pop() {
            Some(u) => {
                pop_chars(&mut self.output, u.out.chars().count());
                Response::Replace {
                    delete_units: 1,
                    text: String::new(),
                }
            }
            None => Response::Passthrough,
        }
    }

    fn match_sequence_rule(&self, key: &str) -> Option<(Vec<String>, String)> {
        let sequences = &self.scheme.as_keys()?.sequences;
        let mut best: Option<(Vec<String>, String)> = None;
        for r in sequences.iter().filter(|r| r.is_active(&self.enabled)) {
            let seq = r.match_tokens();
            if seq.last().map(String::as_str) != Some(key) {
                continue;
            }
            let prefix = &seq[..seq.len() - 1];
            if prefix.len() > self.history.len() {
                continue;
            }
            let start = self.history.len() - prefix.len();
            let matches = prefix.iter().enumerate().all(|(k, want)| {
                let u = &self.history[start + k];
                u.keys.len() == 1 && &u.keys[0] == want
            });
            if matches {
                let longer = best.as_ref().is_none_or(|(b, _)| seq.len() > b.len());
                if longer {
                    best = Some((seq, r.emit.clone()));
                }
            }
        }
        best
    }
}

/// Convenience: is this scheme usable as a live keys session?
pub fn is_keys(scheme: &Scheme) -> bool {
    matches!(scheme, Scheme::Keys(_))
}
