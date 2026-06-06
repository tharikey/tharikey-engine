//! Golden tests for the two v1 seed schemes.
//! Proves both input kinds (keys + text) and reversibility.

use tharikey_core::{
    bundled_scheme_ids, load_seed, merge_base, reverse, transliterate, KeyEvent, Layer, QuotePair,
    RawScheme, Response, Scheme, Session, Severity, Transform,
};

// ---- Engine-wide invariants ------------------------------------------------

#[test]
fn merge_base_three_policies() {
    // Pins the documented merge policy (see merge_base): Replace / Union / Append.
    let base = RawScheme::from_toml_str(
        "[meta]\nid='base'\ninput='text'\n\
         [abugida]\nvowel_carrier='އ'\ndefault_coda='ް'\n\
         [consonants]\nh='ހ'\nk='ކ'\n\
         [rewrites]\nrules=[['ހ','ހ','','ބ']]\n",
    )
    .unwrap();
    let child = RawScheme::from_toml_str(
        "[meta]\nid='child'\nbase='base'\ninput='text'\n\
         [consonants]\nk='ޚ'\nn='ނ'\n\
         [rewrites]\nrules=[['ނ','ނ','','މ']]\n",
    )
    .unwrap();
    let m = merge_base(base, child);
    // Replace: the child's meta wins; abugida (unset by child) is inherited from base.
    assert_eq!(m.meta.id, "child");
    assert_eq!(m.abugida.as_ref().unwrap().vowel_carrier, "އ");
    // Union: maps merge, child key wins on a clash.
    assert_eq!(m.consonants.get("h").map(String::as_str), Some("ހ")); // base-only kept
    assert_eq!(m.consonants.get("k").map(String::as_str), Some("ޚ")); // child overrides
    assert_eq!(m.consonants.get("n").map(String::as_str), Some("ނ")); // child-only added
                                                                      // Append: ordered rule lists concatenate, base first.
    assert_eq!(m.rewrites.rules.len(), 2);
    assert_eq!(m.rewrites.rules[0][0], "ހ"); // base rule first
    assert_eq!(m.rewrites.rules[1][0], "ނ"); // child rule second
}

#[test]
fn all_bundled_schemes_validate_clean() {
    // Validation only runs on demand (cli / tests), not on the load path — so pin it here: every
    // shipped scheme must load and validate with zero error-level issues.
    for id in bundled_scheme_ids() {
        let s = load_seed(id).unwrap_or_else(|e| panic!("scheme `{id}` failed to load: {e}"));
        let errors: Vec<_> = s
            .validate()
            .into_iter()
            .filter(|i| i.severity == Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "scheme `{id}` has validation errors: {errors:?}"
        );
    }
}

#[test]
fn male_latin_reverse_glottal_contract() {
    // Reverse is glottal-aware: word-final ށް/ތް read back as the glottal romanisation, not the bare
    // consonant ("rash"). It is still lossy (one reading only). Pins the contract (see reverse() docs).
    let s = load_seed("male-latin").unwrap();
    assert_eq!(reverse(&s, "ރަށް").as_deref(), Some("rah")); // NOT "rash"
    assert_eq!(reverse(&s, "ކޮށް").as_deref(), Some("koh"));
    assert_eq!(reverse(&s, "ރަތް").as_deref(), Some("raiy")); // thaa glide
    assert_eq!(reverse(&s, "ހިތް").as_deref(), Some("hiy")); // thaa after an i-nucleus (deduped)
}

// ---- Malé Latin (text, reversible) ----------------------------------------

#[test]
fn male_latin_loads_and_validates() {
    let s = load_seed("male-latin").expect("load male-latin");
    let issues = s.validate();
    assert!(
        issues
            .iter()
            .all(|i| !matches!(i.severity, tharikey_core::Severity::Error)),
        "validation errors: {issues:?}"
    );
}

#[test]
fn male_latin_transliterate() {
    let s = load_seed("male-latin").unwrap();
    // bas = ބަސް  (consonant takes a default sukun at the word boundary)
    assert_eq!(transliterate(&s, "bas"), "ބަސް");
    // maale = މާލެ  (long aa as a digraph; final vowel attaches, no sukun)
    assert_eq!(transliterate(&s, "maale"), "މާލެ");
    // dhathuru = ދަތުރު  (digraphs dh/th win via longest-match)
    assert_eq!(transliterate(&s, "dhathuru"), "ދަތުރު");
    // rah = ރަށް  (sh digraph -> SHAVIYANI, final sukun)
    assert_eq!(transliterate(&s, "rash"), "ރަށް");
}

#[test]
fn male_latin_standalone_vowel_carrier() {
    let s = load_seed("male-latin").unwrap();
    // ana = އަނަ  (word-initial vowel uses the alifu carrier)
    assert_eq!(transliterate(&s, "ana"), "އަނަ");
}

#[test]
fn male_latin_glottal_letter_by_preceding_vowel() {
    // The word-final glottal letter is chosen by the preceding vowel (corpus-derived, ≈90%/bucket):
    // a/o/u -> shaviyani ށ, e/ey -> alifu އ (the indefinite -eh), i/ee -> thaa ތ.
    let s = load_seed("male-latin").unwrap();
    assert_eq!(transliterate(&s, "rah"), "ރަށް"); // a -> SHAVIYANI (island)
    assert_eq!(transliterate(&s, "koh"), "ކޮށް"); // o -> SHAVIYANI
    assert_eq!(transliterate(&s, "ekeh"), "އެކެއް"); // e -> ALIFU (indefinite)
    assert_eq!(transliterate(&s, "hih"), "ހިތް"); // i -> THAA
    assert_eq!(transliterate(&s, "rangah"), "ރަންގަށް"); // last vowel before glottal wins
}

#[test]
fn male_latin_word_final_y_is_eebeefili() {
    let s = load_seed("male-latin").unwrap();
    // word-final y on a pending consonant -> consonant + ee: native verb endings (-ny -> ނީ) + loan -ty.
    assert_eq!(transliterate(&s, "dheny"), "ދެނީ");
    assert_eq!(transliterate(&s, "gennany"), "ގެންނަނީ");
    // word-initial / medial y stays yaa (a consonant), unaffected.
    assert_eq!(transliterate(&s, "yageen"), "ޔަގީން");
    assert_eq!(transliterate(&s, "liyun"), "ލިޔުން");
}

#[test]
fn male_latin_c_is_a_forward_only_alias() {
    let s = load_seed("male-latin").unwrap();
    // standalone c -> kaafu (was unmapped → raw-Latin garbage); the `ch` digraph still wins.
    assert_eq!(transliterate(&s, "coach"), "ކޯޗް");
    assert_eq!(transliterate(&s, "chat"), "ޗަޓް");
    // c is forward-only (`reverse_exclude`): ކ reverses to "k", not "c".
    assert_eq!(reverse(&s, "ކ").as_deref(), Some("k"));
    assert_eq!(reverse(&s, "ކަށި").as_deref(), Some("kashi"));
}

#[test]
fn male_latin_word_final_glide_to_thaa() {
    // Word-final "iy" (after a vowel nucleus) is the glottal glide -> thaa+sukun ތް.
    let s = load_seed("male-latin").unwrap();
    assert_eq!(transliterate(&s, "raiy"), "ރަތް"); // red
    assert_eq!(transliterate(&s, "koiy"), "ކޮތް");
    // guards: the bare diphthong and medial "iy" must NOT glottalise.
    assert_eq!(transliterate(&s, "rai"), "ރައި"); // real vowel sequence, untouched
    assert_eq!(transliterate(&s, "liyun"), "ލިޔުން"); // medial iy untouched
}

#[test]
fn male_latin_medial_h_glottal() {
    let s = load_seed("male-latin").unwrap();
    // medial "h" before a consonant -> the glottal/geminate marker އް (humans write the marker as `h`)
    assert_eq!(transliterate(&s, "huhdha"), "ހުއްދަ");
    assert_eq!(transliterate(&s, "mahsala"), "މައްސަލަ");
    assert_eq!(transliterate(&s, "mahchah"), "މައްޗަށް"); // medial marker + word-final glottal
                                                      // but "o" + h + consonant is a stem-final glottal glued in a compound -> ށ, not the marker
    assert_eq!(transliterate(&s, "kohfi"), "ކޮށްފި");
    assert_eq!(transliterate(&s, "dhookohlaifi"), "ދޫކޮށްލައިފި");
}

#[test]
fn male_latin_corpus_rewrite_layer() {
    // male-latin-corpus = male-latin + a data-mined `rewrites` post-correction layer.
    let base = load_seed("male-latin").unwrap();
    let corpus = load_seed("male-latin-corpus").unwrap();
    // prenasal: the baseline composes a literal noonu-sukun; the rewrite strips it (kandu = ocean)
    assert_eq!(transliterate(&base, "kandu"), "ކަންޑު");
    assert_eq!(transliterate(&corpus, "kandu"), "ކަނޑު");
    // a final-glottal letter correction
    assert_eq!(transliterate(&corpus, "thah"), "ތައް");
    // baseline-correct words are untouched by the rewrite layer
    assert_eq!(
        transliterate(&corpus, "bappa"),
        transliterate(&base, "bappa")
    );
}

#[test]
fn male_latin_gemination() {
    let s = load_seed("male-latin").unwrap();
    // doubled consonant -> alifu+sukun (އް) + the consonant, once
    assert_eq!(transliterate(&s, "raajje"), "ރާއްޖެ"); // jj
    assert_eq!(transliterate(&s, "bappa"), "ބައްޕަ"); // pp
    assert_eq!(transliterate(&s, "labba"), "ލައްބަ"); // bb
    assert_eq!(transliterate(&s, "vettun"), "ވެއްޓުން"); // tt
                                                     // nasal gemination uses noonu+sukun (ން)
    assert_eq!(transliterate(&s, "mamma"), "މަންމަ"); // mm
}

#[test]
fn male_latin_round_trips() {
    let s = load_seed("male-latin").unwrap();
    // only non-lossy words: word-final glottals (ރަށް etc.) are intentionally many-to-one, covered by
    // `male_latin_reverse_glottal_contract`. `kashi` keeps `sh`-digraph coverage (mid-word, reversible).
    for word in ["bas", "maale", "dhathuru", "kashi", "ana"] {
        let thaana = transliterate(&s, word);
        assert_eq!(
            reverse(&s, &thaana).as_deref(),
            Some(word),
            "round-trip failed for {word} ({thaana})"
        );
    }
}

#[test]
fn male_latin_reverse_gemination() {
    let s = load_seed("male-latin").unwrap();
    assert_eq!(reverse(&s, "ރާއްޖެ").as_deref(), Some("raajje"));
    assert_eq!(reverse(&s, "ބައްޕަ").as_deref(), Some("bappa"));
    assert_eq!(reverse(&s, "ލައްބަ").as_deref(), Some("labba"));
    assert_eq!(reverse(&s, "ވެއްޓުން").as_deref(), Some("vettun"));
    assert_eq!(reverse(&s, "މަންމަ").as_deref(), Some("mamma")); // nasal -> noonu+sukun
                                                              // full round-trip now works for geminated words
    for w in ["raajje", "bappa", "labba", "vettun", "mamma"] {
        assert_eq!(
            reverse(&s, &transliterate(&s, w)).as_deref(),
            Some(w),
            "round-trip {w}"
        );
    }
}

#[test]
fn male_latin_passthrough_unknown() {
    let s = load_seed("male-latin").unwrap();
    // space is a boundary; the trailing consonant still gets its sukun
    assert_eq!(transliterate(&s, "bas mas"), "ބަސް މަސް");
}

// ---- Phonetic (keys) ------------------------------------------------------

fn feed_base(session: &mut Session, keys: &str) {
    for k in keys.chars() {
        session.feed(KeyEvent::new(k.to_string(), Layer::Base));
    }
}

#[test]
fn phonetic_loads() {
    let s = load_seed("phonetic").expect("load phonetic");
    assert!(s
        .validate()
        .iter()
        .all(|i| !matches!(i.severity, tharikey_core::Severity::Error)));
}

#[test]
fn phonetic_keys_are_1to1() {
    let s = load_seed("phonetic").unwrap();
    let mut sess = Session::new(s.clone());
    // typing the phonetic keys b a s q -> ބަސް (q is the sukun key)
    feed_base(&mut sess, "basq");
    assert_eq!(sess.output(), "ބަސް");
}

#[test]
fn phonetic_shift_layer() {
    let s = load_seed("phonetic").unwrap();
    let mut sess = Session::new(s.clone());
    // Shift+a = AABAAFILI (long aa)
    let r = sess.feed(KeyEvent::new("a", Layer::Shift));
    assert_eq!(r, Response::Insert("ާ".to_string()));
    assert_eq!(sess.output(), "ާ");
}

#[test]
fn phonetic_long_press_variants() {
    let s = load_seed("phonetic").unwrap();
    let k = s.as_keys().unwrap();
    // ޱ NAA now has a real key on `x` base (the one Thaana letter SIL omits); x's old × cruft is dropped.
    assert_eq!(k.layers.base.get("x").map(String::as_str), Some("ޱ"));
    assert!(s.variants_for("x").is_empty());
    assert!(s.variants_for("a").is_empty());
    // ﷲ ALLAH ligature is on Shift+F (the slot we'd been leaving empty)
    assert_eq!(k.layers.shift.get("f").map(String::as_str), Some("ﷲ"));
    // ﷽ BISMILLAH on backtick (one-shot insert; presentation form)
    assert_eq!(k.layers.base.get("`").map(String::as_str), Some("﷽"));
}

#[test]
fn phonetic_unmapped_key_passes_through() {
    let s = load_seed("phonetic").unwrap();
    let mut sess = Session::new(s.clone());
    // `1` is not bound in the base layer
    assert_eq!(
        sess.feed(KeyEvent::new("1", Layer::Base)),
        Response::Passthrough
    );
    assert_eq!(sess.output(), "");
}

#[test]
fn phonetic_long_vowel_double_tap_off_by_default() {
    let s = load_seed("phonetic").unwrap();
    // default: toggle off -> two short filis, for every vowel
    for (key, short_pair) in [("a", "ަަ"), ("i", "ިި"), ("u", "ުު"), ("e", "ެެ"), ("o", "ޮޮ")]
    {
        let mut off = Session::new(s.clone());
        feed_base(&mut off, &format!("{key}{key}"));
        assert_eq!(off.output(), short_pair, "key {key}");
    }
}

#[test]
fn phonetic_long_vowel_double_tap_on() {
    let s = load_seed("phonetic").unwrap();
    // one toggle enables all five; each double-tap collapses to the long form
    for (key, long) in [("a", "ާ"), ("i", "ީ"), ("u", "ޫ"), ("e", "ޭ"), ("o", "ޯ")] {
        let mut on = Session::new(s.clone());
        on.set_rule("long-vowel-double-tap", true);
        let r1 = on.feed(KeyEvent::new(key, Layer::Base));
        assert!(matches!(r1, Response::Insert(_)));
        let r2 = on.feed(KeyEvent::new(key, Layer::Base));
        assert_eq!(
            r2,
            Response::Replace {
                delete_units: 1,
                text: long.to_string()
            },
            "key {key}"
        );
        assert_eq!(on.output(), long, "key {key}");
    }
}

// ---- Context-aware (direct-keyboard) path: peek / resolve ----------------

#[test]
fn peek_passes_through_and_inserts_without_context() {
    let s = load_seed("phonetic").unwrap();
    let mut sess = Session::new(s.clone());
    // unmapped key → passthrough, no context read needed
    assert_eq!(
        sess.peek(KeyEvent::new("1", Layer::Base)),
        Response::Passthrough
    );
    // ordinary mapped key with the double-tap OFF → finished Insert, never NeedsContext
    assert_eq!(
        sess.peek(KeyEvent::new("a", Layer::Base)),
        Response::Insert("ަ".into())
    );
    assert_eq!(
        sess.peek(KeyEvent::new("k", Layer::Base)),
        Response::Insert("ކ".into())
    );
}

#[test]
fn peek_requests_context_only_for_active_reachback_keys() {
    let s = load_seed("phonetic").unwrap();
    let mut sess = Session::new(s.clone());
    sess.set_rule("long-vowel-double-tap", true);
    // a vowel that can double now defers to resolve…
    assert_eq!(
        sess.peek(KeyEvent::new("a", Layer::Base)),
        Response::NeedsContext
    );
    // …but a consonant (no reach-back rule) is still resolved inline — no wasted context read.
    assert_eq!(
        sess.peek(KeyEvent::new("k", Layer::Base)),
        Response::Insert("ކ".into())
    );
}

#[test]
fn resolve_doubles_against_real_document_context() {
    let s = load_seed("phonetic").unwrap();
    let mut sess = Session::new(s.clone());
    sess.set_rule("long-vowel-double-tap", true);
    // no matching context → fresh insert of the short fili
    assert_eq!(
        sess.resolve(KeyEvent::new("a", Layer::Base), ""),
        Response::Insert("ަ".into())
    );
    assert_eq!(
        sess.resolve(KeyEvent::new("a", Layer::Base), "ކ"),
        Response::Insert("ަ".into()),
        "after a bare consonant, `a` is a fresh short fili, not a doubling"
    );
    // document tail IS the short fili → reach back and replace it with the long form,
    // regardless of how it got there (typed, or clicked back onto an existing ަ)
    assert_eq!(
        sess.resolve(KeyEvent::new("a", Layer::Base), "ކަ"),
        Response::Replace {
            delete_units: 1,
            text: "ާ".into()
        },
        "`a` after ަ doubles — context-matched, no session history"
    );
}

#[test]
fn resolve_is_inert_when_reachback_disabled() {
    let s = load_seed("phonetic").unwrap();
    let mut sess = Session::new(s.clone()); // double-tap off by default
                                            // even with a ަ in the document, nothing reaches back when the rule is off
    assert_eq!(
        sess.resolve(KeyEvent::new("a", Layer::Base), "ކަ"),
        Response::Insert("ަ".into())
    );
}

// ---- Toggleable options (transforms + optional rules), with metadata ------

#[test]
fn phonetic_options_listed_with_metadata() {
    let s = load_seed("phonetic").unwrap();
    let opts = s.options();
    let names: Vec<&str> = opts.iter().map(|o| o.name.as_str()).collect();
    // inherited transforms + the double-tap (its five rules collapse to ONE option)
    assert!(names.contains(&"bracket_flip"));
    assert!(names.contains(&"rufiyaa"));
    assert!(names.contains(&"smart_quotes"));
    assert_eq!(
        names
            .iter()
            .filter(|n| **n == "long-vowel-double-tap")
            .count(),
        1
    );
    // each carries user-facing metadata + its default state
    let dt = opts
        .iter()
        .find(|o| o.name == "long-vowel-double-tap")
        .unwrap();
    assert_eq!(dt.label, "Long-vowel double-tap");
    assert!(!dt.description.is_empty());
    assert!(!dt.default, "double-tap is off by default");
    let rf = opts.iter().find(|o| o.name == "rufiyaa").unwrap();
    assert_eq!(rf.label, "Rufiyaa sign");
    assert!(rf.default, "rufiyaa is on by default");
}

#[test]
fn text_scheme_has_no_session_options() {
    assert!(load_seed("male-latin").unwrap().options().is_empty());
}

// ---- Shared aligned defaults (thaana-common inheritance) ------------------

#[test]
fn schemes_inherit_aligned_defaults() {
    let p = load_seed("phonetic").unwrap();
    let t = load_seed("typewriter").unwrap();
    let m = load_seed("male-latin").unwrap();
    let a = load_seed("aletinu").unwrap();

    // smart-quote defaults come from thaana-common -> identical across schemes
    let pq = smart_quote_double(&p);
    let tq = smart_quote_double(&t);
    assert_eq!((pq.open, pq.close), (tq.open, tq.close));

    // text schemes gain the shared RTL punctuation (now in [literals])
    for s in [&m, &a] {
        let lit = &s.as_text().unwrap().literals;
        assert_eq!(lit.get(","), Some(&"،".to_string()));
        assert_eq!(lit.get(";"), Some(&"؛".to_string()));
        assert_eq!(lit.get("?"), Some(&"؟".to_string()));
    }
    // and they apply in transliteration (correct on the Latin path, per owner)
    assert_eq!(transliterate(&m, "bas, mas"), "ބަސް، މަސް");
    assert_eq!(
        transliterate(&m, "kihineh?"),
        transliterate(&m, "kihineh") + "؟"
    );

    // bracket-flip default is now shared via common -> on for both keys schemes
    assert!(transform_enabled(&p, "bracket_flip"));
    assert!(transform_enabled(&t, "bracket_flip"));

    // Dives Akuru (different script) does NOT inherit thaana-common (keys scheme, no inherited quotes)
    let d = load_seed("dives-akuru").unwrap();
    assert!(!d.as_keys().unwrap().transforms.contains_key("smart_quotes"));
}

/// The `smart_quotes` transform's double-quote pair (panics if absent / wrong kind).
fn smart_quote_double(s: &Scheme) -> QuotePair {
    match s.as_keys().unwrap().transforms.get("smart_quotes") {
        Some(Transform::SmartQuotes { double, .. }) => double.clone().unwrap(),
        _ => panic!("expected a [transforms.smart_quotes]"),
    }
}

/// Whether a named transform is present and enabled by default.
fn transform_enabled(s: &Scheme, name: &str) -> bool {
    s.as_keys()
        .unwrap()
        .transforms
        .get(name)
        .is_some_and(|t| t.is_enabled())
}

// ---- Rufiyaa sign + NAA ---------------------------------------------------

#[test]
fn rufiyaa_sign_keys_and_text() {
    // keys: Shift+4 emits "$", the rufiyaa transform (on by default) substitutes it to U+20C2
    let p = load_seed("phonetic").unwrap();
    let mut sess = Session::new(p.clone());
    assert_eq!(
        sess.feed(KeyEvent::new("4", Layer::Shift)),
        Response::Insert("\u{20C2}".into())
    );
    // text: `$` in male-latin emits the rufiyaa sign (deterministic [literals] mapping)
    let m = load_seed("male-latin").unwrap();
    assert_eq!(transliterate(&m, "$"), "\u{20C2}");
}

#[test]
fn rufiyaa_transform_toggle_and_variants() {
    let p = load_seed("phonetic").unwrap();
    // toggle the rufiyaa substitution OFF -> Shift+4 yields a literal "$"
    let mut off = Session::new(p.clone());
    off.set_transform("rufiyaa", false);
    assert_eq!(
        off.feed(KeyEvent::new("4", Layer::Shift)),
        Response::Insert("$".into())
    );
    // the long-press variants are the literal currency signs (explicit inserts bypass the transform)
    assert_eq!(
        p.as_keys().unwrap().variants_for("4"),
        &["$".to_string(), "€".to_string()]
    );
}

#[test]
fn male_latin_naa() {
    let s = load_seed("male-latin").unwrap();
    // dn -> NAA (U+07B1), then a short fili
    assert_eq!(transliterate(&s, "dna"), "ޱަ");
    assert_eq!(reverse(&s, "ޱަ").as_deref(), Some("dna"));
}

#[test]
fn male_latin_prenasalization() {
    let s = load_seed("male-latin").unwrap();
    // ' drops the pending consonant's coda -> hus-noonu (no sukun on the ނ)
    assert_eq!(transliterate(&s, "dhan'du"), "ދަނޑު");
    // contrast: plain "nd" keeps the sukun
    assert_eq!(transliterate(&s, "bandhu"), "ބަންދު");
    // both round-trip (reverse re-inserts ' for the bare consonant)
    assert_eq!(reverse(&s, "ދަނޑު").as_deref(), Some("dhan'du"));
    assert_eq!(reverse(&s, "ބަންދު").as_deref(), Some("bandhu"));
}

#[test]
fn male_latin_reverse_punctuation() {
    let s = load_seed("male-latin").unwrap();
    let t = transliterate(&s, "bas, mas?");
    assert_eq!(t, "ބަސް، މަސް؟"); // forward: RTL-correct punctuation
    assert_eq!(reverse(&s, &t).as_deref(), Some("bas, mas?")); // reverse restores Latin punctuation
}

#[test]
fn male_latin_arabic_loan_q_w() {
    let s = load_seed("male-latin").unwrap();
    assert_eq!(transliterate(&s, "qa"), "ޤަ"); // QAAFU
    assert_eq!(transliterate(&s, "wa"), "ޥަ"); // WAAVU (loan, distinct from v)
    assert_eq!(reverse(&s, "ޤަ").as_deref(), Some("qa"));
    assert_eq!(reverse(&s, "ޥަ").as_deref(), Some("wa"));
}

#[test]
fn male_latin_final_glottal() {
    let s = load_seed("male-latin").unwrap();
    // word-final "h" is the glottal stop އް (ހް is illegal); default to alifu
    assert_eq!(transliterate(&s, "kihineh"), "ކިހިނެއް");
    assert_eq!(transliterate(&s, "maheh"), "މަހެއް");
    // medial "h" (a vowel follows) stays ހ
    assert_eq!(transliterate(&s, "bahaaru"), "ބަހާރު");
    // round-trips: word-final glottal comes back as "h"
    assert_eq!(reverse(&s, "ކިހިނެއް").as_deref(), Some("kihineh"));
    assert_eq!(
        reverse(&s, &transliterate(&s, "maheh")).as_deref(),
        Some("maheh")
    );
}

// ---- Áletinu (text, thatmaldivesblog) -------------------------------------

#[test]
fn aletinu_loads_and_validates() {
    let s = load_seed("aletinu").expect("load aletinu");
    assert!(s
        .validate()
        .iter()
        .all(|i| !matches!(i.severity, tharikey_core::Severity::Error)));
}

#[test]
fn aletinu_transliterate() {
    let s = load_seed("aletinu").unwrap();
    assert_eq!(transliterate(&s, "ðaþuru"), "ދަތުރު"); // Ðaþuru
    assert_eq!(transliterate(&s, "aharen"), "އަހަރެން"); // standalone vowel carrier
    assert_eq!(transliterate(&s, "kalé"), "ކަލޭ"); // long é
    assert_eq!(transliterate(&s, "máþ"), "މާތް"); // long á + final sukun
    assert_eq!(transliterate(&s, "boq"), "ބޮއް"); // q glottal -> alifu+sukun
    assert_eq!(transliterate(&s, "rax"), "ރަށް"); // x -> SHAVIYANI
    assert_eq!(transliterate(&s, "fæ"), "ފައި"); // æ special vowel
}

#[test]
fn aletinu_reverse() {
    let s = load_seed("aletinu").unwrap();
    // common words round-trip (reverse = Thaana -> Áletinu, the useful direction)
    for w in ["ðaþuru", "kalé", "máþ", "boq", "rax", "aharen"] {
        let thaana = transliterate(&s, w);
        assert_eq!(reverse(&s, &thaana).as_deref(), Some(w), "round-trip {w}");
    }
    // word-final glottal -> q; gemination -> the doubled consonant
    assert_eq!(reverse(&s, "ބޮއް").as_deref(), Some("boq"));
    assert_eq!(
        reverse(&s, &transliterate(&s, "bappa")).as_deref(),
        Some("bappa")
    );
}

#[test]
fn iso15919_round_trips() {
    let s = load_seed("iso15919").unwrap();
    assert!(s
        .validate()
        .iter()
        .all(|i| !matches!(i.severity, tharikey_core::Severity::Error)));
    // forward (scholarly: dots-below, macrons, dental d/t, glottal ʾ)
    assert_eq!(transliterate(&s, "raṣ"), "ރަށް");
    assert_eq!(transliterate(&s, "dā"), "ދާ");
    assert_eq!(transliterate(&s, "māt"), "މާތް");
    // reverse (Thaana -> ISO 15919, the useful direction) round-trips
    for w in ["raṣ", "dā", "māt", "kihineʾ", "rājje"] {
        let thaana = transliterate(&s, w);
        assert_eq!(reverse(&s, &thaana).as_deref(), Some(w), "round-trip {w}");
    }
}

#[test]
fn text_schemes_case_insensitive() {
    // sentence-capitalised romanization works (case is orthographic, not phonemic)
    let a = load_seed("aletinu").unwrap();
    assert_eq!(transliterate(&a, "Ðaþuru"), "ދަތުރު"); // capital Ð/Þ fold to ð/þ
    assert_eq!(transliterate(&a, "Aharen"), "އަހަރެން"); // capital A
    let m = load_seed("male-latin").unwrap();
    assert_eq!(transliterate(&m, "Raajje"), "ރާއްޖެ"); // capital R
}

// ---- Dives Akuru (keys, InScript) -----------------------------------------

#[test]
fn dives_akuru_loads_and_validates() {
    let s = load_seed("dives-akuru").expect("load dives-akuru");
    assert!(s
        .validate()
        .iter()
        .all(|i| !matches!(i.severity, tharikey_core::Severity::Error)));
}

#[test]
fn dives_akuru_emits_astral_codepoints() {
    let s = load_seed("dives-akuru").unwrap();
    let mut sess = Session::new(s.clone());
    // base `k` = DIVES AKURU LETTER KA (U+1190C)
    assert_eq!(
        sess.feed(KeyEvent::new("k", Layer::Base)),
        Response::Insert("\u{1190C}".into())
    );
    // Shift+`d` = DIVES AKURU LETTER A, an independent vowel (U+11900)
    assert_eq!(
        sess.feed(KeyEvent::new("d", Layer::Shift)),
        Response::Insert("\u{11900}".into())
    );
    // base `d` = VIRAMA (U+1193E), the conjunct former
    assert_eq!(
        sess.feed(KeyEvent::new("d", Layer::Base)),
        Response::Insert("\u{1193E}".into())
    );
    assert_eq!(sess.output(), "\u{1190C}\u{11900}\u{1193E}");
}

// ---- Typewriter (keys) ----------------------------------------------------

#[test]
fn typewriter_loads_and_validates() {
    let s = load_seed("typewriter").expect("load typewriter");
    assert!(s
        .validate()
        .iter()
        .all(|i| !matches!(i.severity, tharikey_core::Severity::Error)));
}

#[test]
fn typewriter_types_a_word() {
    let s = load_seed("typewriter").unwrap();
    let mut sess = Session::new(s.clone());
    // ބަސް on the typewriter: ބ=m, ަ=f, ސ=c, ް=d
    feed_base(&mut sess, "mfcd");
    assert_eq!(sess.output(), "ބަސް");
}

#[test]
fn typewriter_shift_loanword_letter() {
    let s = load_seed("typewriter").unwrap();
    let mut sess = Session::new(s.clone());
    // Shift+p = HHAA ޙ
    assert_eq!(
        sess.feed(KeyEvent::new("p", Layer::Shift)),
        Response::Insert("ޙ".to_string())
    );
}

#[test]
fn phonetic_smart_quotes() {
    let s = load_seed("phonetic").unwrap();
    let (enabled, dq, sg) = match s.as_keys().unwrap().transforms.get("smart_quotes") {
        Some(Transform::SmartQuotes {
            enabled,
            double,
            single,
            ..
        }) => (*enabled, double.clone().unwrap(), single.clone().unwrap()),
        _ => panic!("phonetic has [transforms.smart_quotes]"),
    };

    // default OFF: straight quotes pass through unchanged
    assert!(!enabled);
    let mut off = Session::new(s.clone());
    assert_eq!(
        off.feed(KeyEvent::new("'", Layer::Shift)),
        Response::Insert("\"".into())
    ); // straight "
    assert_eq!(
        off.feed(KeyEvent::new("'", Layer::Base)),
        Response::Insert("'".into())
    ); // straight '

    // ON: double quotes alternate open -> close
    let mut on = Session::new(s.clone());
    on.set_smart_quotes(true);
    assert_eq!(
        on.feed(KeyEvent::new("'", Layer::Shift)),
        Response::Insert(dq.open.clone())
    );
    assert_eq!(
        on.feed(KeyEvent::new("'", Layer::Shift)),
        Response::Insert(dq.close.clone())
    );
    assert_eq!(
        on.feed(KeyEvent::new("'", Layer::Shift)),
        Response::Insert(dq.open.clone())
    );

    // ON: single quotes get the same treatment, independently
    let mut on2 = Session::new(s.clone());
    on2.set_smart_quotes(true);
    assert_eq!(
        on2.feed(KeyEvent::new("'", Layer::Base)),
        Response::Insert(sg.open.clone())
    );
    assert_eq!(
        on2.feed(KeyEvent::new("'", Layer::Base)),
        Response::Insert(sg.close.clone())
    );
}

#[test]
fn phonetic_bracket_flip() {
    let s = load_seed("phonetic").unwrap();
    assert!(transform_enabled(&s, "bracket_flip")); // default on (RTL)

    // flip ON (default): the right-hand "open" key emits the logical OPEN codepoint
    let mut on = Session::new(s.clone());
    assert_eq!(
        on.feed(KeyEvent::new("]", Layer::Base)),
        Response::Insert("[".into())
    ); // ] key -> [
    assert_eq!(
        on.feed(KeyEvent::new("[", Layer::Base)),
        Response::Insert("]".into())
    ); // [ key -> ]
    assert_eq!(
        on.feed(KeyEvent::new("0", Layer::Shift)),
        Response::Insert("(".into())
    ); // ) -> (
    assert_eq!(
        on.feed(KeyEvent::new("9", Layer::Shift)),
        Response::Insert(")".into())
    ); // ( -> )
    assert_eq!(
        on.feed(KeyEvent::new("]", Layer::Shift)),
        Response::Insert("{".into())
    ); // } -> {

    // flip OFF: US convention, brackets pass through as mapped
    let mut off = Session::new(s.clone());
    off.set_bracket_flip(false);
    assert_eq!(
        off.feed(KeyEvent::new("[", Layer::Base)),
        Response::Insert("[".into())
    );
    assert_eq!(
        off.feed(KeyEvent::new("9", Layer::Shift)),
        Response::Insert("(".into())
    );
}

#[test]
fn phonetic_unit_backspace() {
    let s = load_seed("phonetic").unwrap();
    let mut sess = Session::new(s.clone());
    feed_base(&mut sess, "bas");
    assert_eq!(sess.output(), "ބަސް".chars().take(3).collect::<String>()); // ބަސ (no sukun yet)
    sess.backspace();
    assert_eq!(sess.output(), "ބަ");
    sess.backspace();
    assert_eq!(sess.output(), "ބ");
}
