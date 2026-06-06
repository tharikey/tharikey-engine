//! Scheme model + TOML (de)serialization + validation.
//!
//! A *scheme* is the data that drives the engine. Two input kinds share one model:
//! - `input = "keys"`  — physical keyboard layouts (Phonetic, Typewriter): [`Layers`] tables.
//! - `input = "text"`  — romanization (Malé Latin): `consonants`/`vowels` maps + [`Abugida`].
//!
//! See `docs/scheme-format.md` for the full spec.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;

/// What the user types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputKind {
    /// Physical keyboard keys (layer tables).
    Keys,
    /// Romanized text sequences (maps + abugida composition).
    Text,
}

/// Modifier layer for `keys` schemes. Modifiers are *structural*, never string-encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    Base,
    Shift,
    Opt,
    ShiftOpt,
    Caps,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Meta {
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// Short one-line "what this is" blurb (the scheme cards). Scheme data, not host-hardcoded.
    #[serde(default)]
    pub description: String,
    /// Longer "about this keyboard" text — details + **attribution** (the source layout / license).
    /// Surfaced on the scheme's detail page.
    #[serde(default)]
    pub about: String,
    /// Optional parent scheme to inherit shared defaults from (e.g. `thaana-common`). Resolved
    /// relative to this scheme's file. The child overrides the base.
    #[serde(default)]
    pub base: Option<String>,
    pub input: InputKind,
    #[serde(default = "default_output")]
    pub output: String,
    #[serde(default)]
    pub reversible: bool,
    #[serde(default)]
    pub lossy: bool,
    #[serde(default)]
    pub version: String,
    /// Forward-only input tokens that must NOT become the reverse canonical for their output (e.g.
    /// Malé Latin `c` is a convenience alias for ކ, but ކ reverses to `k`). Excluded keys still map
    /// forward; reverse just prefers a non-excluded key for the same output.
    #[serde(default)]
    pub reverse_exclude: Vec<String>,
}

fn default_output() -> String {
    "thaana".to_string()
}

/// Layer tables for `keys` schemes: each layer maps a US-QWERTY key label to its output.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Layers {
    #[serde(default)]
    pub base: BTreeMap<String, String>,
    #[serde(default)]
    pub shift: BTreeMap<String, String>,
    #[serde(default)]
    pub opt: BTreeMap<String, String>,
    #[serde(default)]
    pub shift_opt: BTreeMap<String, String>,
    #[serde(default)]
    pub caps: BTreeMap<String, String>,
}

/// Abugida role bindings for `text` schemes. These are the *role table* the engine references
/// instead of hardcoded literals (so the processor stays script-agnostic — see Appendix A of
/// `docs/transliteration-engine.md`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Abugida {
    /// Independent-vowel carrier (Thaana: alifu އ). A standalone vowel = carrier + fili.
    #[serde(default)]
    pub vowel_carrier: String,
    /// Pure-killer applied to a consonant with no following vowel (Thaana: sukun ް).
    #[serde(default)]
    pub default_coda: String,
    /// Geminate marker emitted before a doubled consonant, replacing the first (Thaana: `އް`).
    /// Empty = gemination handling off.
    #[serde(default)]
    pub geminate: String,
    /// Geminate marker used when the doubled consonant is a nasal (Thaana: `ން`). Falls back to
    /// `geminate` when empty.
    #[serde(default)]
    pub geminate_nasal: String,
    /// Output bases treated as nasal for gemination (Thaana: މ, ނ).
    #[serde(default)]
    pub nasals: Vec<String>,
    /// Token that suppresses the pending consonant's coda — prenasalisation / hus-noonu. Malé Latin
    /// `n'd` → ނޑ (bare noonu, no sukun) instead of `nd` → ންޑ. Reverse re-inserts it between a bare
    /// consonant and a following consonant.
    #[serde(default)]
    pub prenasal_marker: String,
}

/// A **context atom** — the shared vocabulary for both coda and rewrite rule contexts (DVTextUtils
/// `\V`/`\C`, ICU `{ }`/`[:set:]`, foma sets; see `docs/rule-system-research.md`). A bare string is a
/// **class keyword** (`"vowel"`/`"consonant"`/`"boundary"`) if reserved, else a **literal** token/char;
/// a sub-array is an **inline one-of set**. The membership of `vowel`/`consonant` is resolved against the
/// scheme's own `[consonants]`/`[vowels]` tables (keys on the Latin side, values on the Thaana side) —
/// never hardcoded.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Atom {
    /// A class keyword (`vowel`/`consonant`/`boundary`) or a literal token/char.
    Word(String),
    /// An inline one-of set.
    Set(Vec<String>),
}

impl Atom {
    /// Whether `w` is one of the reserved class keywords (vs a literal).
    pub fn is_keyword(w: &str) -> bool {
        matches!(w, "vowel" | "consonant" | "boundary")
    }
}

/// What a matched [`CodaRule`] emits. Outputs reference **abugida roles** (Lipika-style) — the
/// `Composer` owns the mechanics; the rule only names intent. Authored as `"glottal"`/`"marker"` (a bare
/// role) or `["glide","ތ"]`/`["fili","ee"]` (a role + argument).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Emit {
    /// A letter from a top-level `[lookups]` table indexed by the **preceding vowel** the rule's `pre`
    /// matched, + `Abugida::default_coda`. The general parallel-array primitive (Keyman `any()`/`index()`);
    /// the engine has no notion of "glottal" — that's just a scheme-chosen table name.
    Lookup(String),
    /// The geminate marker (`Abugida::geminate`, `އް`) — a medial glottal / gemination.
    Marker,
    /// A coda consonant `letter` + `Abugida::default_coda` (a word-final glide).
    Glide(String),
    /// Attach the named vowel token's fili to the pending consonant.
    Fili(String),
}

impl Emit {
    /// Parse a coda rule's emit field (an [`Atom`]): `Word("marker")`, or `Set([kind, arg])` for
    /// `["lookup", table]` / `["glide", letter]` / `["fili", vowel-token]`.
    pub fn from_atom(a: &Atom) -> Option<Emit> {
        match a {
            Atom::Word(w) if w == "marker" => Some(Emit::Marker),
            Atom::Set(v) if v.len() == 2 && v[0] == "lookup" => Some(Emit::Lookup(v[1].clone())),
            Atom::Set(v) if v.len() == 2 && v[0] == "glide" => Some(Emit::Glide(v[1].clone())),
            Atom::Set(v) if v.len() == 2 && v[0] == "fili" => Some(Emit::Fili(v[1].clone())),
            _ => None,
        }
    }
}

/// One **resolved** coda rule, matched first-to-last (first match wins). `tok` is a Latin input token
/// matched as a literal prefix in the stream; `pre`/`suf` are atom sequences (coda uses a single-position
/// context). Authored compactly as `[tok, pre, suf, emit]` — see `docs/scheme-format.md`.
#[derive(Debug, Clone)]
pub struct CodaRule {
    pub tok: String,
    pub pre: Vec<Atom>,
    pub suf: Vec<Atom>,
    pub emit: Emit,
}

/// One coda rule as authored: `[tok, pre, suf, emit]`.
type RawCodaRule = (String, Vec<Atom>, Vec<Atom>, Atom);

/// The `[coda]` table as deserialized (the compact rule arrays); resolved into [`Coda`]. Lookup tables
/// referenced by `["lookup", name]` emits live top-level in `[lookups]`, not here.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawCoda {
    #[serde(default)]
    pub rules: Vec<RawCodaRule>,
}

/// Language-specific **coda orthography** for `text` schemes: an ordered, first-match-wins context-rule
/// list (the former bespoke final-glottal / glide / final-vowel / medial-glottal fields collapsed into
/// one `[tok, pre, suf, emit]` shape — see `docs/rule-spike.md`). Kept out of the universal [`Abugida`]
/// role table: Dhivehi/Malé-Latin facts, corpus-derived from politecat314/dhivehi-transliteration.
#[derive(Debug, Clone, Default)]
pub struct Coda {
    /// Ordered, first-match-wins coda rules.
    pub rules: Vec<CodaRule>,
}

/// A post-composition rewrite rule (the `rewrites` list): replace `match` with `out` when preceded by
/// `pre` and followed by `post` (contexts are matched, not consumed). Applied as an ordered
/// left-to-right pass over the **composed Thaana output**, each rule over the previous result — an
/// **FST-inspired** correction layer (cf. cvutils' foma g2p) that we run as a *rewrite cascade* rather
/// than a compiled state machine (see the docs for why). **Data-mined and net-scored** by an offline
/// pipeline (in the toolkit, outside this engine), so a scheme that carries them trades the hand-curated baseline's debuggability for
/// corpus-derived accuracy; the rules still *generalise* (held-out gain), so the scheme stays
/// lexicon-free. See `docs/rule-spike.md`.
///
/// Authored compactly, one rule per line, as a 4-field array `[pre, match, post, out]`. Each Thaana
/// segment is its own quoted string, so an editor's bidi stays local to each field instead of scrambling
/// a whole mixed-direction line:
/// ```toml
/// rewrites = [
///   ["ތަ", "ށ", "ް", "އ"],   # after ތަ, before ް: ށ → އ
///   ["ަނ", "ް", "ޑު", ""],    # after ަނ, before ޑު: delete the ް (prenasal)
/// ]
/// ```
#[derive(Debug, Clone)]
pub struct RewriteRule {
    pub pre: String,
    pub match_: String,
    pub post: String,
    pub out: String,
}

impl RewriteRule {
    /// Build from the `[pre, match, post, out]` array form. `None` if the array isn't 4 fields or
    /// `match` is empty.
    pub fn from_fields(f: Vec<String>) -> Option<RewriteRule> {
        let [pre, match_, post, out]: [String; 4] = f.try_into().ok()?;
        if match_.is_empty() {
            return None;
        }
        Some(RewriteRule {
            pre,
            match_,
            post,
            out,
        })
    }
}

/// The `[rewrites]` table: the post-correction rule list (and room for future knobs, e.g. an `enabled`
/// toggle). A *table* so `[meta]` can stay first — a bare top-level array would have to precede every
/// table header. Each rule is one `[pre, match, post, out]` line.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RewriteTable {
    #[serde(default)]
    pub rules: Vec<Vec<String>>,
}

/// A key-**sequence** macro for `keys` schemes (the escape hatch): `match` is a whitespace-separated
/// sequence of key labels matched against typed history; on a match it emits a replacement. Toggleable.
/// Distinct from the `text`-path `coda.rules` / `rewrites.rules` (those are context-rewrite rule lists).
#[derive(Debug, Clone, Deserialize)]
pub struct Sequence {
    pub name: String,
    #[serde(default)]
    pub desc: String,
    /// User-facing name + "what this is" for the Options screen (an optional rule is a toggleable
    /// option). Rules sharing a `name` (e.g. the five double-tap rules) are one option — set these on
    /// the first; the host dedupes by name.
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "match")]
    pub match_: String,
    pub emit: String,
    /// `true` = a user-toggleable nicety (e.g. `aa → aabaafili`).
    #[serde(default)]
    pub optional: bool,
    /// Default on/off state for an optional rule.
    #[serde(default)]
    pub default: bool,
}

impl Sequence {
    /// The match expressed as a sequence of tokens.
    pub fn match_tokens(&self) -> Vec<String> {
        self.match_
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }
    /// Whether this rule is active given the set of enabled optional-rule names.
    pub fn is_active(&self, enabled: &std::collections::HashSet<String>) -> bool {
        !self.optional || enabled.contains(&self.name)
    }
}

/// An open/close glyph pair for a quotation mark. RTL note: `open` is the logically-first mark (it
/// renders on the *right* in RTL text), `close` the logically-second (left). Glyphs are data so the
/// RTL convention is a config choice, not hardcoded.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct QuotePair {
    pub open: String,
    pub close: String,
}

/// A toggleable **output transform** for the `keys` path — a *preference* (distinct from the always-on
/// orthographic *facts* like `[literals]`/`[abugida]`). The `Session` applies the enabled ones to a
/// keypress's output, dispatched by `kind`. Two kinds today:
/// - **`smart-quotes`** — stateful: a straight `"`/`'` → its curly glyph, alternating open/close.
/// - **`substitute`** — a stateless 1:1 map. Covers the RTL bracket-flip (the mirror pairs, now *data*
///   not a hardcoded fn) and the rufiyaa convenience (`$` → `⃂`).
///
/// Only live typing (`Session::feed`) is transformed; explicit inserts (long-press variants) bypass.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Transform {
    SmartQuotes {
        #[serde(default)]
        label: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        enabled: bool,
        #[serde(default)]
        double: Option<QuotePair>,
        #[serde(default)]
        single: Option<QuotePair>,
    },
    Substitute {
        #[serde(default)]
        label: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        enabled: bool,
        #[serde(default)]
        map: std::collections::HashMap<String, String>,
    },
}

impl Transform {
    /// User-facing name (the Options screen). Empty if the scheme didn't set one.
    pub fn label(&self) -> &str {
        match self {
            Transform::SmartQuotes { label, .. } | Transform::Substitute { label, .. } => label,
        }
    }
    /// "What this is" blurb. Empty if the scheme didn't set one.
    pub fn description(&self) -> &str {
        match self {
            Transform::SmartQuotes { description, .. }
            | Transform::Substitute { description, .. } => description,
        }
    }
    /// Whether this transform is currently on.
    pub fn is_enabled(&self) -> bool {
        match self {
            Transform::SmartQuotes { enabled, .. } | Transform::Substitute { enabled, .. } => {
                *enabled
            }
        }
    }
    /// Toggle this transform on/off (the runtime IME setting).
    pub fn set_enabled(&mut self, on: bool) {
        match self {
            Transform::SmartQuotes { enabled, .. } | Transform::Substitute { enabled, .. } => {
                *enabled = on
            }
        }
    }
}

/// A user-toggleable scheme option — a `[transforms]` entry or an optional `[[sequence]]` rule —
/// flattened into one shape with display metadata, for a host's Options screen. `name` is the toggle key
/// for [`crate::Session::set_transform`] / `set_rule`.
#[derive(Debug, Clone)]
pub struct OptionInfo {
    pub name: String,
    pub label: String,
    pub description: String,
    pub default: bool,
    pub kind: OptionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionKind {
    Transform,
    Rule,
}

/// The on-disk shape: a flat record with every section optional, used only as the **deserialization +
/// inheritance** target. It is resolved into the typed [`Scheme`] sum type (which makes illegal states
/// — e.g. a scheme with both `[layers]` and `[abugida]` — unrepresentable). Keep this internal-ish;
/// downstream code works with [`Scheme`].
#[derive(Debug, Clone, Deserialize)]
pub struct RawScheme {
    pub meta: Meta,
    #[serde(default)]
    pub layers: Layers,
    #[serde(default)]
    pub abugida: Option<Abugida>,
    #[serde(default)]
    pub coda: Option<RawCoda>,
    /// Top-level `[lookups]`: named key→value tables referenced by `["lookup", name]` emits (the general
    /// parallel-array primitive). Reusable across rule layers; not coda-specific.
    #[serde(default)]
    pub lookups: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    /// The `[rewrites]` table (post-composition corrections; validated in `resolve`).
    #[serde(default)]
    pub rewrites: RewriteTable,
    #[serde(default)]
    pub consonants: BTreeMap<String, String>,
    #[serde(default)]
    pub vowels: BTreeMap<String, String>,
    #[serde(default)]
    pub literals: BTreeMap<String, String>,
    #[serde(default)]
    pub variants: BTreeMap<String, Vec<String>>,
    /// `[transforms]`: named, toggleable keys-path output transforms (smart-quotes, substitutions).
    #[serde(default)]
    pub transforms: std::collections::HashMap<String, Transform>,
    #[serde(default, rename = "sequence")]
    pub sequences: Vec<Sequence>,
}

impl RawScheme {
    pub fn from_toml_str(s: &str) -> Result<RawScheme, SchemeError> {
        basic_toml::from_str(s).map_err(|e| SchemeError::Parse(e.to_string()))
    }

    // NOTE: the core does **no filesystem I/O** — it works identically in every binding (wasm has no fs).
    // Hosts read bytes themselves and call `from_toml_str` (then `resolve`); the runtime registry
    // (see `lib.rs`) holds host-fed schemes. The CLI's file-loading + directory-relative `base` walk
    // lives in `src/bin/cli.rs`, built on this public API + `merge_base`.

    /// Partition the flat record into the typed [`Scheme`] by `meta.input`, dropping the inert sections
    /// of the other kind. Hard-errors on a structurally impossible scheme (text without `[abugida]`).
    pub fn resolve(self) -> Result<Scheme, SchemeError> {
        match self.meta.input {
            InputKind::Keys => Ok(Scheme::Keys(KeysScheme {
                meta: self.meta,
                layers: self.layers,
                variants: self.variants,
                transforms: self.transforms,
                sequences: self.sequences,
            })),
            InputKind::Text => {
                let abugida = self.abugida.ok_or_else(|| {
                    SchemeError::Invalid("text scheme is missing `[abugida]`".into())
                })?;
                let rewrites = self
                    .rewrites
                    .rules
                    .into_iter()
                    .map(|f| {
                        let dbg = f.clone();
                        RewriteRule::from_fields(f).ok_or_else(|| {
                            SchemeError::Invalid(format!(
                                "malformed rewrite rule (empty match): {dbg:?}"
                            ))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let mut coda = Coda { rules: Vec::new() };
                for (tok, pre, suf, emit_atom) in self.coda.unwrap_or_default().rules {
                    let emit = Emit::from_atom(&emit_atom).ok_or_else(|| {
                        SchemeError::Invalid(format!("malformed coda rule emit: {emit_atom:?}"))
                    })?;
                    coda.rules.push(CodaRule {
                        tok,
                        pre,
                        suf,
                        emit,
                    });
                }
                Ok(Scheme::Text(TextScheme {
                    meta: self.meta,
                    abugida,
                    coda,
                    rewrites,
                    lookups: self.lookups,
                    consonants: self.consonants,
                    vowels: self.vowels,
                    literals: self.literals,
                }))
            }
        }
    }
}

/// A `keys` scheme: physical-layout layer tables + IME niceties. No text-composition sections exist on
/// this type — they cannot be set.
#[derive(Debug, Clone)]
pub struct KeysScheme {
    pub meta: Meta,
    pub layers: Layers,
    pub variants: BTreeMap<String, Vec<String>>,
    /// Named, toggleable keys-path output transforms (`[transforms]`).
    pub transforms: std::collections::HashMap<String, Transform>,
    pub sequences: Vec<Sequence>,
}

impl KeysScheme {
    /// The key->output map for a given layer (`caps` falls back to `base`).
    pub fn layer_map(&self, layer: Layer) -> &BTreeMap<String, String> {
        match layer {
            Layer::Base => &self.layers.base,
            Layer::Shift => &self.layers.shift,
            Layer::Opt => &self.layers.opt,
            Layer::ShiftOpt => &self.layers.shift_opt,
            Layer::Caps if !self.layers.caps.is_empty() => &self.layers.caps,
            Layer::Caps => &self.layers.base,
        }
    }

    /// Long-press variant outputs for a key label (empty if none).
    pub fn variants_for(&self, key: &str) -> &[String] {
        self.variants.get(key).map(Vec::as_slice).unwrap_or(&[])
    }

    /// The user-toggleable options for this layout — `[transforms]` + optional `[[sequence]]` rules —
    /// with display metadata, for a host's Options screen. Deterministic order: transforms by name, then
    /// optional rules in declared order (deduped by name — rules sharing a name are one option).
    pub fn options(&self) -> Vec<OptionInfo> {
        let mut out = Vec::new();
        let mut names: Vec<&String> = self.transforms.keys().collect();
        names.sort();
        for name in names {
            let t = &self.transforms[name];
            out.push(OptionInfo {
                name: name.clone(),
                label: if t.label().is_empty() {
                    name.clone()
                } else {
                    t.label().to_string()
                },
                description: t.description().to_string(),
                default: t.is_enabled(),
                kind: OptionKind::Transform,
            });
        }
        let mut seen = std::collections::HashSet::new();
        for r in self.sequences.iter().filter(|r| r.optional) {
            if seen.insert(r.name.clone()) {
                out.push(OptionInfo {
                    name: r.name.clone(),
                    label: if r.label.is_empty() {
                        r.name.clone()
                    } else {
                        r.label.clone()
                    },
                    description: r.description.clone(),
                    default: r.default,
                    kind: OptionKind::Rule,
                });
            }
        }
        out
    }
}

/// A `text` scheme: romanization maps + abugida composition. `abugida` is mandatory (a text scheme
/// without it is unrepresentable); `coda` defaults to empty.
#[derive(Debug, Clone)]
pub struct TextScheme {
    pub meta: Meta,
    pub abugida: Abugida,
    pub coda: Coda,
    /// Post-composition rewrite rules (the `rewrites` list); empty for the hand-curated baseline.
    pub rewrites: Vec<RewriteRule>,
    /// Named key→value lookup tables (`[lookups]`), referenced by `Emit::Lookup`.
    pub lookups: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    pub consonants: BTreeMap<String, String>,
    pub vowels: BTreeMap<String, String>,
    pub literals: BTreeMap<String, String>,
}

impl TextScheme {
    /// The fili (dependent vowel) for a text token.
    pub fn vowel_output(&self, token: &str) -> Option<&str> {
        self.vowels.get(token).map(|s| s.as_str())
    }

    /// All input tokens used by the scheme (for longest-match).
    pub fn text_tokens(&self) -> Vec<String> {
        self.consonants
            .keys()
            .chain(self.vowels.keys())
            .chain(self.literals.keys())
            .cloned()
            .collect()
    }
}

/// A resolved, usable scheme — one of the two input kinds. The variant is the discriminator, so the
/// engine never has to ask "is this the right kind?" via an optional field.
#[derive(Debug, Clone)]
pub enum Scheme {
    Keys(KeysScheme),
    Text(TextScheme),
}

impl Scheme {
    /// Shared metadata, regardless of kind.
    pub fn meta(&self) -> &Meta {
        match self {
            Scheme::Keys(k) => &k.meta,
            Scheme::Text(t) => &t.meta,
        }
    }

    pub fn as_keys(&self) -> Option<&KeysScheme> {
        match self {
            Scheme::Keys(k) => Some(k),
            Scheme::Text(_) => None,
        }
    }

    pub fn as_text(&self) -> Option<&TextScheme> {
        match self {
            Scheme::Text(t) => Some(t),
            Scheme::Keys(_) => None,
        }
    }

    /// Long-press variants for a key label (empty for a text scheme).
    pub fn variants_for(&self, key: &str) -> &[String] {
        self.as_keys().map(|k| k.variants_for(key)).unwrap_or(&[])
    }

    /// User-toggleable options (empty for a text scheme — its niceties run in the one-shot path, not as
    /// session toggles).
    pub fn options(&self) -> Vec<OptionInfo> {
        self.as_keys().map(KeysScheme::options).unwrap_or_default()
    }

    /// The output a key produces in a layer — for a host's on-screen-keyboard preview. `None` if the key
    /// is unmapped in that layer, or this is a text scheme (no physical layout).
    pub fn key_output(&self, layer: Layer, key: &str) -> Option<&str> {
        self.as_keys()
            .and_then(|k| k.layer_map(layer).get(key))
            .map(String::as_str)
    }

    /// Lightweight validation. Returns issues (empty = clean). Structural impossibilities are caught
    /// earlier by [`RawScheme::resolve`]; this is the soft layer (empty outputs, lossy collisions).
    pub fn validate(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        match self {
            Scheme::Keys(k) => {
                if k.layers.base.is_empty() {
                    issues.push(ValidationIssue::error(
                        "keys scheme has an empty `base` layer",
                    ));
                }
                for (layer_name, map) in [
                    ("base", &k.layers.base),
                    ("shift", &k.layers.shift),
                    ("opt", &k.layers.opt),
                    ("shift_opt", &k.layers.shift_opt),
                    ("caps", &k.layers.caps),
                ] {
                    for (key, v) in map {
                        if v.is_empty() {
                            issues.push(ValidationIssue::warn(format!(
                                "empty output for key `{key}` in layer `{layer_name}`"
                            )));
                        }
                    }
                }
                for r in &k.sequences {
                    if r.match_tokens().is_empty() {
                        issues.push(ValidationIssue::error(format!(
                            "rule `{}` has empty match",
                            r.name
                        )));
                    }
                }
            }
            Scheme::Text(t) => {
                // the engine references these roles for every standalone vowel / bare consonant;
                // empty = silently malformed output, so treat as errors.
                if t.abugida.vowel_carrier.is_empty() {
                    issues.push(ValidationIssue::error("abugida has empty `vowel_carrier`"));
                }
                if t.abugida.default_coda.is_empty() {
                    issues.push(ValidationIssue::error("abugida has empty `default_coda`"));
                }
                if t.consonants.is_empty() {
                    issues.push(ValidationIssue::error("text scheme has no `[consonants]`"));
                }
                let collides = has_value_collision(&t.consonants) || has_value_collision(&t.vowels);
                if t.meta.reversible && collides && !t.meta.lossy {
                    issues.push(ValidationIssue::warn(
                        "reversible scheme has output collisions but is not flagged `lossy`",
                    ));
                }
            }
        }
        issues
    }
}

/// Replace `slot` with `child` only when the child actually set it — the **Replace** merge policy for
/// optional whole-block config (see [`merge_base`]).
fn replace_if_set<T>(slot: &mut Option<T>, child: Option<T>) {
    if child.is_some() {
        *slot = child;
    }
}

/// Merge a `base` raw scheme with a `child` that declared `base = "<id>"`. Operates on [`RawScheme`] so
/// inheritance composes *before* the kind is resolved (a `base` fragment like `thaana-common` may carry
/// fields for both kinds). Every field follows one of **three explicit policies** — and which one is the
/// whole point of this function, so it is grouped and labelled accordingly:
///
/// - **Replace** — the child's value wins outright. The scheme's identity + whole-block config a child
///   either inherits untouched or redefines wholesale: `meta` (always the child's), and the optional
///   `abugida` / `coda` blocks (replaced only when the child sets them).
/// - **Union** — key→value maps merge, **child key wins** on a clash (shared defaults from the base,
///   child overrides): every `layers.*`, `consonants`, `vowels`, `literals`, `variants`, `lookups`, and
///   `transforms` (union at the *name* level — a child entry replaces a base entry of the same name).
/// - **Append** — ordered rule lists concatenate, **base first then child**: `rewrites.rules`,
///   `sequences`.
pub fn merge_base(base: RawScheme, child: RawScheme) -> RawScheme {
    let mut out = base;

    // Replace: the child wins outright.
    out.meta = child.meta;
    replace_if_set(&mut out.abugida, child.abugida);
    replace_if_set(&mut out.coda, child.coda);

    // Union: maps merge, child key wins.
    out.layers.base.extend(child.layers.base);
    out.layers.shift.extend(child.layers.shift);
    out.layers.opt.extend(child.layers.opt);
    out.layers.shift_opt.extend(child.layers.shift_opt);
    out.layers.caps.extend(child.layers.caps);
    out.consonants.extend(child.consonants);
    out.vowels.extend(child.vowels);
    out.literals.extend(child.literals);
    out.variants.extend(child.variants);
    out.lookups.extend(child.lookups);
    out.transforms.extend(child.transforms);

    // Append: ordered rule lists concatenate (base first, then child).
    out.rewrites.rules.extend(child.rewrites.rules);
    out.sequences.extend(child.sequences);

    out
}

fn has_value_collision(map: &BTreeMap<String, String>) -> bool {
    let mut seen = std::collections::HashSet::new();
    map.values().any(|v| !seen.insert(v))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub message: String,
}

impl ValidationIssue {
    fn warn(m: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warn,
            message: m.into(),
        }
    }
    fn error(m: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: m.into(),
        }
    }
}

/// Errors from parsing/resolving a scheme. The core does **no I/O**, so there is no `Io` variant —
/// hosts read bytes themselves and surface their own read errors (the CLI does this in `cli.rs`).
#[derive(Debug)]
pub enum SchemeError {
    Parse(String),
    Invalid(String),
}

impl fmt::Display for SchemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemeError::Parse(e) => write!(f, "parse error: {e}"),
            SchemeError::Invalid(e) => write!(f, "invalid scheme: {e}"),
        }
    }
}

impl std::error::Error for SchemeError {}
