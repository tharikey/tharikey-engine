//! `tharikey-core` — a small, data-driven Thaana transliteration / keymap engine.
//!
//! Schemes are TOML data (see `docs/scheme-format.md`). The engine is script-agnostic in its logic:
//! the abugida processor references abstract roles supplied by the scheme, not hardcoded Thaana
//! codepoints, so it generalizes to other abugidas without a rewrite (see Appendix A of
//! `docs/transliteration-engine.md`).
//!
//! ```
//! use tharikey_core::{RawScheme, transliterate, reverse};
//! let toml = r#"
//! [meta]
//! id = "demo"
//! input = "text"
//! reversible = true
//! [abugida]
//! vowel_carrier = "އ"
//! default_coda = "ް"
//! [consonants]
//! b = "ބ"
//! s = "ސ"
//! [vowels]
//! a = "ަ"
//! "#;
//! let scheme = RawScheme::from_toml_str(toml).unwrap().resolve().unwrap();
//! assert_eq!(transliterate(&scheme, "bas"), "ބަސް");
//! assert_eq!(reverse(&scheme, "ބަސް").as_deref(), Some("bas"));
//! ```

mod engine;
mod matcher;
mod scheme;
mod util;

pub use engine::{is_keys, reverse, transliterate, KeyEvent, Response, Session};
pub use scheme::{
    merge_base, Abugida, Coda, InputKind, KeysScheme, Layer, Layers, Meta, OptionInfo, OptionKind,
    QuotePair, RawScheme, Scheme, SchemeError, Sequence, Severity, TextScheme, Transform,
    ValidationIssue,
};

/// The default schemes, **embedded into the binary** so they're available with no filesystem access —
/// the same set ships to every binding (wasm, python, swift, android, …). `thaana-common` is the base
/// fragment (not a usable scheme on its own). User/custom schemes come in separately via
/// [`RawScheme::from_toml_str`] (then `.resolve()`), or the host-fed runtime registry below — the core
/// reads no files (that's the host's job; the CLI does its own file loading).
const BUNDLED: &[(&str, &str)] = &[
    ("phonetic", include_str!("../schemes/phonetic.toml")),
    ("typewriter", include_str!("../schemes/typewriter.toml")),
    ("dives-akuru", include_str!("../schemes/dives-akuru.toml")),
    ("male-latin", include_str!("../schemes/male-latin.toml")),
    (
        "male-latin-corpus",
        include_str!("../schemes/male-latin-corpus.toml"),
    ),
    ("aletinu", include_str!("../schemes/aletinu.toml")),
    ("iso15919", include_str!("../schemes/iso15919.toml")),
    (
        "thaana-common",
        include_str!("../schemes/thaana-common.toml"),
    ),
];

/// The engine version — the unified workspace version (`[workspace.package] version`), inherited by
/// every crate via `version.workspace = true`. Surfaced through the bindings for diagnostics.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Ids of the bundled, *usable* schemes (excludes the `thaana-common` base fragment).
pub fn bundled_scheme_ids() -> Vec<&'static str> {
    BUNDLED
        .iter()
        .map(|(k, _)| *k)
        .filter(|k| *k != "thaana-common")
        .collect()
}

/// Load a bundled scheme by id (e.g. `"phonetic"`, `"male-latin"`), resolving `base` inheritance from
/// the embedded set. No filesystem — works identically in every binding.
pub fn load_seed(id: &str) -> Result<Scheme, SchemeError> {
    load_seed_raw(id)?.resolve()
}

/// Load + merge a bundled scheme as a flat [`RawScheme`] (inheritance composes before the kind is
/// resolved, so a `base` fragment may carry both kinds' fields).
fn load_seed_raw(id: &str) -> Result<RawScheme, SchemeError> {
    let toml = BUNDLED
        .iter()
        .find(|(k, _)| *k == id)
        .map(|(_, v)| *v)
        .ok_or_else(|| SchemeError::Invalid(format!("unknown scheme: {id}")))?;
    let scheme = RawScheme::from_toml_str(toml)?;
    match scheme.meta.base.clone() {
        Some(base_id) => Ok(merge_base(load_seed_raw(&base_id)?, scheme)),
        None => Ok(scheme),
    }
}

// ---------------------------------------------------------------------------
// Runtime scheme registry — custom / third-party schemes, fed in by the host
// ---------------------------------------------------------------------------
//
// A registered scheme is the *runtime twin* of a `BUNDLED` one: the **exact same TOML scheme format**,
// resolved through the **exact same path** (optional `base` inheritance → `resolve`). No second format.
//
// The registry is **process-global and ephemeral** — the engine does no I/O and holds no persistence.
// The host (e.g. the IME) re-hydrates it on every process start, *synchronously before it serves input*,
// from its own store. So there is no engine-side init/persistence to manage; rehydration timing is the
// host's responsibility (do it on the launch path, never async, and a cold respawn never drops input —
// the OS holds the keystroke until the IMK connection is up, which is after `didFinishLaunching`).

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

fn registry() -> &'static RwLock<HashMap<String, String>> {
    static R: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();
    R.get_or_init(|| RwLock::new(HashMap::new()))
}

// Poison-tolerant access: the registry is a plain `HashMap`, so a panic in one caller while holding the
// lock must NOT poison-panic everyone after it — that would take down the host (the IME runs the engine
// in-process). Recover the guard via `into_inner`; the worst case is a half-applied insert, not a crash.
fn reg_read() -> RwLockReadGuard<'static, HashMap<String, String>> {
    registry().read().unwrap_or_else(|e| e.into_inner())
}
fn reg_write() -> RwLockWriteGuard<'static, HashMap<String, String>> {
    registry().write().unwrap_or_else(|e| e.into_inner())
}

/// True if `id` is one of the compiled-in schemes (including the `thaana-common` base fragment).
fn is_bundled(id: &str) -> bool {
    BUNDLED.iter().any(|(k, _)| *k == id)
}

/// Whether `id` names a host-registered (custom) scheme.
pub fn is_custom(id: &str) -> bool {
    reg_read().contains_key(id)
}

/// Ids of all host-registered schemes (unordered).
pub fn custom_scheme_ids() -> Vec<String> {
    reg_read().keys().cloned().collect()
}

/// All usable scheme ids — bundled (minus the base fragment) then custom.
pub fn all_scheme_ids() -> Vec<String> {
    let mut ids: Vec<String> = bundled_scheme_ids().iter().map(|s| s.to_string()).collect();
    ids.extend(custom_scheme_ids());
    ids
}

/// Register a custom scheme under `id` from its TOML (the same scheme format as bundled schemes). Errors
/// cleanly if `id` collides with a built-in or an already-registered scheme, or if the TOML doesn't
/// parse/resolve (including its optional `base`). Updates = [`unregister_scheme`] then register.
pub fn register_scheme(id: &str, toml: &str) -> Result<(), SchemeError> {
    if is_bundled(id) {
        return Err(SchemeError::Invalid(format!(
            "scheme id '{id}' collides with a built-in scheme"
        )));
    }
    if is_custom(id) {
        return Err(SchemeError::Invalid(format!(
            "scheme id '{id}' is already registered"
        )));
    }
    resolve_toml(toml)?; // validate end-to-end (parse + base + resolve) before storing
    reg_write().insert(id.to_string(), toml.to_string());
    Ok(())
}

/// Remove a host-registered scheme (no-op if absent / built-in).
pub fn unregister_scheme(id: &str) {
    reg_write().remove(id);
}

/// Drop all host-registered schemes (built-ins untouched) — lets the host re-hydrate deterministically.
pub fn clear_custom() {
    reg_write().clear();
}

/// Load + resolve a scheme by id — **custom registry first, then bundled**. The custom-aware twin of
/// [`load_seed`]; all id-keyed lookups should go through this so registered schemes work everywhere a
/// bundled id does.
pub fn load(id: &str) -> Result<Scheme, SchemeError> {
    load_raw(id)?.resolve()
}

/// Resolve a scheme's *raw* form by id, custom registry first then bundled, composing `base` inheritance
/// through the same path (custom may inherit custom or bundled). Depth-guarded against cyclic bases.
fn load_raw(id: &str) -> Result<RawScheme, SchemeError> {
    load_raw_depth(id, 0)
}

fn load_raw_depth(id: &str, depth: usize) -> Result<RawScheme, SchemeError> {
    if depth > 16 {
        return Err(SchemeError::Invalid(format!(
            "scheme '{id}' base chain too deep (cyclic?)"
        )));
    }
    let custom = reg_read().get(id).cloned();
    if let Some(toml) = custom {
        let raw = RawScheme::from_toml_str(&toml)?;
        return match raw.meta.base.clone() {
            Some(base) => Ok(merge_base(load_raw_depth(&base, depth + 1)?, raw)),
            None => Ok(raw),
        };
    }
    load_seed_raw(id)
}

/// Parse + resolve a scheme TOML directly (composing its optional `base`). Used to validate at register.
fn resolve_toml(toml: &str) -> Result<Scheme, SchemeError> {
    let raw = RawScheme::from_toml_str(toml)?;
    let merged = match raw.meta.base.clone() {
        Some(base) => merge_base(load_raw(&base)?, raw),
        None => raw,
    };
    merged.resolve()
}

#[cfg(test)]
mod registry_tests {
    use super::*;

    // The engine does NOT author schemes — the host builds the TOML (same format as bundled). Here we
    // hand-author a sparse keys scheme inheriting a bundled base and overriding one key, exactly as the
    // app's TOML serializer would.
    const CUSTOM: &str = r#"
[meta]
id = "my-custom"
name = "My Custom"
input = "keys"
base = "phonetic"
[layers.base]
q = "ޝ"
"#;

    #[test]
    fn register_override_and_collisions() {
        clear_custom();
        register_scheme("my-custom", CUSTOM).unwrap();

        assert!(is_custom("my-custom"));
        assert!(all_scheme_ids().iter().any(|s| s == "my-custom"));

        let scheme = load("my-custom").unwrap();
        assert_eq!(scheme.meta().name, "My Custom");
        assert_eq!(scheme.key_output(Layer::Base, "q"), Some("ޝ")); // overridden
                                                                    // A key we didn't touch still resolves from the base (inherited), not lost.
        let base = load("phonetic").unwrap();
        if let Some(out) = base.key_output(Layer::Base, "a") {
            assert_eq!(scheme.key_output(Layer::Base, "a"), Some(out));
        }

        // Collisions error cleanly.
        assert!(register_scheme("phonetic", CUSTOM).is_err()); // built-in
        assert!(register_scheme("my-custom", CUSTOM).is_err()); // already registered

        unregister_scheme("my-custom");
        assert!(!is_custom("my-custom"));
        clear_custom();
    }
}
