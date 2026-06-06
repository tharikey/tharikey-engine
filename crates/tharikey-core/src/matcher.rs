//! Longest-match over a set of input tokens.
//!
//! Tokens are short (≤3 chars in practice), so a length-descending scan over a `HashSet` is both
//! simple and fast. Kept behind this small type so the engine can stay declarative.

use std::collections::HashSet;

pub struct Matcher {
    tokens: HashSet<String>,
    max_len: usize,
}

impl Matcher {
    pub fn new<I, S>(tokens: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let tokens: HashSet<String> = tokens.into_iter().map(Into::into).collect();
        let max_len = tokens.iter().map(|t| t.chars().count()).max().unwrap_or(0);
        Self { tokens, max_len }
    }

    /// Longest token that is a prefix of `chars`. Returns `(token, len_in_chars)`.
    pub fn longest(&self, chars: &[char]) -> Option<(String, usize)> {
        let upper = self.max_len.min(chars.len());
        for len in (1..=upper).rev() {
            let cand: String = chars[..len].iter().collect();
            if self.tokens.contains(&cand) {
                return Some((cand, len));
            }
        }
        None
    }
}
