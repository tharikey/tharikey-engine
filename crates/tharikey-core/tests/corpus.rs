//! Regression tests against a **human-transliterated corpus** (separate from the self-authored unit
//! tests in `golden.rs`).
//!
//! Hand-picked exact-match pairs from `politecat314/dhivehi-transliteration` (HuggingFace) — human
//! Latin→Thaana that our `male-latin` engine reproduces *exactly*. They double as real-world coverage:
//! between them they exercise gemination, the word-final glottal stop, long vowels, RTL punctuation
//! (`?`→`؟`), and a thiki letter (`kh`→ޚ). Forward direction only — the human gold standard is
//! Latin→Thaana.

use tharikey_core::{load_seed, transliterate};

const PAIRS: &[(&str, &str)] = &[
    ("haadha reechchey", "ހާދަ ރީއްޗޭ"), // gemination chch→އްޗ, long -ey→ޭ
    ("loabiviyas vakivaan ves dhaskurey!", "ލޯބިވިޔަސް ވަކިވާން ވެސް ދަސްކުރޭ!"), // loa→ޯ, '!' passthrough
    (
        "isthashi feybun huttuvaanee kihineh?",
        "އިސްތަށި ފޭބުން ހުއްޓުވާނީ ކިހިނެއް؟",
    ), // tt, glottal, ?→؟
    (
        "imaamunge mahaasinthaa anna honihiru dhuvahu",
        "އިމާމުންގެ މަހާސިންތާ އަންނަ ހޮނިހިރު ދުވަހު",
    ), // nasal gemination anna→އަންނަ
    (
        "isthikhaaraa namaadhaa gulhunhuri baeh vaahaka",
        "އިސްތިޚާރާ ނަމާދާ ގުޅުންހުރި ބައެއް ވާހަކަ",
    ), // kh→ޚ, baeh→ބައެއް
];

#[test]
fn male_latin_matches_human_corpus() {
    let s = load_seed("male-latin").unwrap();
    for (latin, thaana) in PAIRS {
        assert_eq!(transliterate(&s, latin), *thaana, "corpus pair: {latin:?}");
    }
}
