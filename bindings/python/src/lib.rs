//! Python (PyO3) binding for the ThariKey engine.
//!
//! Thin wrapper over the core, at **full parity** with the C ABI: one-shot transforms, scheme metadata,
//! options, per-key output, the host-fed custom-scheme registry, and a stateful `Session` with the
//! context-aware reach-back path (`peek`/`resolve`).
//!
//! The default schemes are embedded in the core crate, so no data files ship with the wheel. Built with
//! maturin.
//!
//! ```python
//! import tharikey
//! tharikey.transliterate("male-latin", "raajje")   # 'ރާއްޖެ'
//! tharikey.reverse("male-latin", "ދިވެހި")          # 'dhivehi'
//! s = tharikey.Session("phonetic"); s.feed("b", "base"); s.output()
//! ```

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use tharikey_core::{
    all_scheme_ids, clear_custom, is_custom, load, reverse as core_reverse,
    transliterate as core_transliterate, InputKind, KeyEvent, Layer, Response, Scheme,
    Session as CoreSession,
};
// `register_scheme` / `unregister_scheme` are intentionally NOT imported — the `#[pyfunction]`s below
// reuse those exact names for the Python API, so we call the core ones fully-qualified.

fn err<E: std::fmt::Display>(e: E) -> PyErr {
    PyValueError::new_err(e.to_string())
}

fn parse_layer(s: &str) -> Layer {
    match s {
        "shift" => Layer::Shift,
        "opt" => Layer::Opt,
        "shift_opt" => Layer::ShiftOpt,
        "caps" => Layer::Caps,
        _ => Layer::Base,
    }
}

fn kind_str(s: &Scheme) -> &'static str {
    match s.meta().input {
        InputKind::Keys => "keys",
        InputKind::Text => "text",
    }
}

// ---------------------------------------------------------------------------
// One-shot text transforms
// ---------------------------------------------------------------------------

/// Latin -> Thaana via a `text` scheme.
#[pyfunction]
fn transliterate(scheme_id: &str, text: &str) -> PyResult<String> {
    let s = load(scheme_id).map_err(err)?;
    Ok(core_transliterate(&s, text))
}

/// Reverse (Thaana -> Latin) for a reversible `text` scheme; `None` if not reversible.
#[pyfunction]
fn reverse(scheme_id: &str, text: &str) -> PyResult<Option<String>> {
    let s = load(scheme_id).map_err(err)?;
    Ok(core_reverse(&s, text))
}

// ---------------------------------------------------------------------------
// Scheme metadata
// ---------------------------------------------------------------------------

/// Metadata for one scheme — `kind` is `"keys"`/`"text"`, `custom` flags a host-registered scheme.
#[pyclass(get_all)]
struct SchemeInfo {
    id: String,
    name: String,
    kind: String,
    description: String,
    about: String,
    custom: bool,
}

fn scheme_info(id: &str) -> Option<SchemeInfo> {
    let s = load(id).ok()?;
    let m = s.meta();
    Some(SchemeInfo {
        id: id.to_string(),
        name: m.name.clone(),
        kind: kind_str(&s).to_string(),
        description: m.description.clone(),
        about: m.about.clone(),
        custom: is_custom(id),
    })
}

/// All usable schemes (bundled + custom), in listing order.
#[pyfunction]
fn list_schemes() -> Vec<SchemeInfo> {
    all_scheme_ids()
        .iter()
        .filter_map(|id| scheme_info(id))
        .collect()
}

/// Metadata for one scheme by id; raises if the id is unknown.
#[pyfunction]
#[pyo3(name = "scheme_info")]
fn scheme_info_py(scheme_id: &str) -> PyResult<SchemeInfo> {
    scheme_info(scheme_id).ok_or_else(|| err(format!("unknown scheme: {scheme_id}")))
}

/// A user-toggleable option (a transform or an optional sequence rule).
#[pyclass(get_all)]
struct SchemeOption {
    name: String,
    label: String,
    description: String,
    default: bool,
}

/// The toggleable options for a scheme (empty for a text scheme); raises if the id is unknown.
#[pyfunction]
fn scheme_options(scheme_id: &str) -> PyResult<Vec<SchemeOption>> {
    let s = load(scheme_id).map_err(err)?;
    Ok(s.options()
        .into_iter()
        .map(|o| SchemeOption {
            name: o.name,
            label: o.label,
            description: o.description,
            default: o.default,
        })
        .collect())
}

/// The Thaana a key produces in a layer (`base`/`shift`/`opt`/`shift_opt`/`caps`). Empty string if the
/// key is unmapped or this is a text scheme; raises on unknown id.
#[pyfunction]
fn key_output(scheme_id: &str, layer: &str, key: &str) -> PyResult<String> {
    let s = load(scheme_id).map_err(err)?;
    Ok(s.key_output(parse_layer(layer), key)
        .unwrap_or("")
        .to_string())
}

// ---------------------------------------------------------------------------
// Custom / third-party schemes — registered at runtime (host owns the store)
// ---------------------------------------------------------------------------

/// Register a custom scheme `id` from its TOML (same format as bundled). Raises on collision with a
/// built-in / an already-registered id, or invalid TOML.
#[pyfunction]
fn register_scheme(scheme_id: &str, toml: &str) -> PyResult<()> {
    tharikey_core::register_scheme(scheme_id, toml).map_err(err)
}

/// Unregister a custom scheme (no-op if absent / built-in).
#[pyfunction]
fn unregister_scheme(scheme_id: &str) {
    tharikey_core::unregister_scheme(scheme_id);
}

/// Drop all custom schemes (built-ins untouched) — for deterministic re-hydration.
#[pyfunction]
fn clear_custom_schemes() {
    clear_custom();
}

/// Whether `id` is a host-registered (custom) scheme.
#[pyfunction]
#[pyo3(name = "is_custom")]
fn is_custom_py(scheme_id: &str) -> bool {
    is_custom(scheme_id)
}

// ---------------------------------------------------------------------------
// Session — the stateful keys-scheme driver
// ---------------------------------------------------------------------------

/// The per-key action from the engine (mirrors the core `Response`):
/// - `action`: `"passthrough"` | `"insert"` | `"replace"` | `"needs_context"`.
/// - `delete_units`: replace only. From `resolve` it counts trailing Unicode scalars of the prefix you
///   passed; from `feed`/`backspace` it counts committed history units.
/// - `text`: text to insert (insert/replace), empty otherwise.
#[pyclass(get_all)]
struct KeyResponse {
    action: String,
    delete_units: usize,
    text: String,
}

fn to_response(r: Response) -> KeyResponse {
    match r {
        Response::Passthrough => KeyResponse {
            action: "passthrough".into(),
            delete_units: 0,
            text: String::new(),
        },
        Response::Insert(t) => KeyResponse {
            action: "insert".into(),
            delete_units: 0,
            text: t,
        },
        Response::Replace { delete_units, text } => KeyResponse {
            action: "replace".into(),
            delete_units,
            text,
        },
        Response::NeedsContext => KeyResponse {
            action: "needs_context".into(),
            delete_units: 0,
            text: String::new(),
        },
    }
}

/// Stateful keys-scheme session.
#[pyclass]
struct Session {
    inner: CoreSession,
}

#[pymethods]
impl Session {
    /// Create a session for a scheme id (bundled or custom). Raises on unknown id.
    #[new]
    fn new(scheme_id: &str) -> PyResult<Self> {
        let s = load(scheme_id).map_err(err)?;
        Ok(Session {
            inner: CoreSession::new(s),
        })
    }

    /// Feed a key (US-QWERTY label + layer). Returns the per-key edit; read `output()` for the buffer.
    fn feed(&mut self, key: &str, layer: &str) -> KeyResponse {
        to_response(self.inner.feed(KeyEvent::new(key, parse_layer(layer))))
    }

    /// Phase 1 of the context-aware path: resolve a key *without* the document. Returns `insert`/
    /// `passthrough`, or `needs_context` — then read the text before the caret and call `resolve`.
    fn peek(&mut self, key: &str, layer: &str) -> KeyResponse {
        to_response(self.inner.peek(KeyEvent::new(key, parse_layer(layer))))
    }

    /// Phase 2 of the context-aware path: given `prefix` (Thaana before the caret), match reach-back
    /// rules. Returns `replace` on a hit (the `aa → ާ` double-tap) or a fresh `insert`.
    fn resolve(&mut self, key: &str, layer: &str, prefix: &str) -> KeyResponse {
        to_response(
            self.inner
                .resolve(KeyEvent::new(key, parse_layer(layer)), prefix),
        )
    }

    /// Delete the last composed unit. Returns the edit to apply.
    fn backspace(&mut self) -> KeyResponse {
        to_response(self.inner.backspace())
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn output(&self) -> String {
        self.inner.output().to_string()
    }

    fn set_smart_quotes(&mut self, on: bool) {
        self.inner.set_smart_quotes(on);
    }

    fn set_bracket_flip(&mut self, on: bool) {
        self.inner.set_bracket_flip(on);
    }

    fn set_transform(&mut self, name: &str, on: bool) {
        self.inner.set_transform(name, on);
    }

    fn set_rule(&mut self, name: &str, on: bool) {
        self.inner.set_rule(name, on);
    }

    /// Set a toggleable option by name, whether it's a transform or an optional rule (names come from
    /// `scheme_options`).
    fn set_option(&mut self, name: &str, on: bool) {
        self.inner.set_option(name, on);
    }
}

#[pymodule]
fn tharikey(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(transliterate, m)?)?;
    m.add_function(wrap_pyfunction!(reverse, m)?)?;
    m.add_function(wrap_pyfunction!(list_schemes, m)?)?;
    m.add_function(wrap_pyfunction!(scheme_info_py, m)?)?;
    m.add_function(wrap_pyfunction!(scheme_options, m)?)?;
    m.add_function(wrap_pyfunction!(key_output, m)?)?;
    m.add_function(wrap_pyfunction!(register_scheme, m)?)?;
    m.add_function(wrap_pyfunction!(unregister_scheme, m)?)?;
    m.add_function(wrap_pyfunction!(clear_custom_schemes, m)?)?;
    m.add_function(wrap_pyfunction!(is_custom_py, m)?)?;
    m.add_class::<SchemeInfo>()?;
    m.add_class::<SchemeOption>()?;
    m.add_class::<KeyResponse>()?;
    m.add_class::<Session>()?;
    Ok(())
}
