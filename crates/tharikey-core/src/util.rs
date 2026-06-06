//! Small, reusable string / char / script helpers shared across the engine. No engine semantics live
//! here — just generic utilities kept out of the composition logic so it stays declarative.

use std::collections::{BTreeMap, HashMap};

/// Does `hay` begin with `needle`? (An empty `needle` is never a match.)
pub(crate) fn starts_with(hay: &[char], needle: &[char]) -> bool {
    !needle.is_empty() && hay.len() >= needle.len() && hay[..needle.len()] == *needle
}

/// Pop the last `n` characters off `s`.
pub(crate) fn pop_chars(s: &mut String, n: usize) {
    for _ in 0..n {
        s.pop();
    }
}

/// Invert a `key -> value` map to `value -> key`. First binding wins (deterministic via the source
/// `BTreeMap`'s key order) so reverse transliteration is stable even when a scheme is `lossy`.
pub(crate) fn invert(
    map: &BTreeMap<String, String>,
    exclude: &std::collections::HashSet<String>,
) -> HashMap<String, String> {
    let mut out = HashMap::new();
    // Non-excluded keys win the canonical (first by BTreeMap order); excluded forward-only aliases
    // fill only the gaps where nothing else maps to that output.
    for (k, v) in map {
        if !exclude.contains(k) {
            out.entry(v.clone()).or_insert_with(|| k.clone());
        }
    }
    for (k, v) in map {
        if exclude.contains(k) {
            out.entry(v.clone()).or_insert_with(|| k.clone());
        }
    }
    out
}

/// Is `c` in the Thaana Unicode block (U+0780–U+07BF)? Lets reverse tell a word-internal letter from a
/// boundary (space / punctuation / Latin / end-of-input).
pub(crate) fn is_thaana(c: char) -> bool {
    ('\u{0780}'..='\u{07BF}').contains(&c)
}
