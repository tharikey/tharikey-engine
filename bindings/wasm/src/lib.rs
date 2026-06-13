//! WASM / JS-TS binding for the ThariKey engine.
//!
//! Thin wrapper over the core, at **full parity** with the C ABI: one-shot transforms, scheme metadata,
//! options, per-key output (for an on-screen keyboard), the host-fed custom-scheme registry, and a
//! stateful `Session` with the context-aware reach-back path (`peek`/`resolve`).
//!
//! The default schemes are embedded in the **core** crate, so nothing data-ships here. Returns real
//! structs/arrays (no hand-rolled JSON); errors surface as thrown `JsError`s.

use tharikey_core::{
    all_scheme_ids, clear_custom, is_custom, load, register_scheme, reverse as core_reverse,
    transliterate as core_transliterate, unregister_scheme, InputKind, KeyEvent, Layer, Response,
    Scheme, Session as CoreSession,
};
use wasm_bindgen::prelude::*;

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
#[wasm_bindgen]
pub fn transliterate(scheme_id: &str, text: &str) -> Result<String, JsError> {
    let s = load(scheme_id).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(core_transliterate(&s, text))
}

/// One-shot reverse (Thaana -> Latin) for a reversible `text` scheme; `null` if not reversible.
#[wasm_bindgen]
pub fn reverse(scheme_id: &str, text: &str) -> Result<Option<String>, JsError> {
    let s = load(scheme_id).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(core_reverse(&s, text))
}

/// Engine version (the unified workspace version, e.g. "0.1.2").
#[wasm_bindgen]
pub fn version() -> String {
    tharikey_core::version().to_string()
}

// ---------------------------------------------------------------------------
// Scheme metadata
// ---------------------------------------------------------------------------

/// Metadata for one scheme — `kind` is `"keys"`/`"text"`, `custom` flags a host-registered scheme.
#[wasm_bindgen(getter_with_clone)]
pub struct SchemeInfo {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub description: String,
    pub about: String,
    pub custom: bool,
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
#[wasm_bindgen(js_name = listSchemes)]
pub fn list_schemes() -> Vec<SchemeInfo> {
    all_scheme_ids()
        .iter()
        .filter_map(|id| scheme_info(id))
        .collect()
}

/// Metadata for one scheme by id; throws if the id is unknown.
#[wasm_bindgen(js_name = schemeInfo)]
pub fn scheme_info_js(scheme_id: &str) -> Result<SchemeInfo, JsError> {
    scheme_info(scheme_id).ok_or_else(|| JsError::new(&format!("unknown scheme: {scheme_id}")))
}

/// A user-toggleable option (a transform or an optional sequence rule).
#[wasm_bindgen(getter_with_clone)]
pub struct SchemeOption {
    pub name: String,
    pub label: String,
    pub description: String,
    pub default: bool,
}

/// The toggleable options for a scheme (empty for a text scheme); throws if the id is unknown.
#[wasm_bindgen(js_name = schemeOptions)]
pub fn scheme_options(scheme_id: &str) -> Result<Vec<SchemeOption>, JsError> {
    let s = load(scheme_id).map_err(|e| JsError::new(&e.to_string()))?;
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

/// The Thaana a key produces in a layer (`base`/`shift`/`opt`/`shift_opt`/`caps`) — for an on-screen
/// keyboard preview. Empty string if the key is unmapped or this is a text scheme; throws on unknown id.
#[wasm_bindgen(js_name = keyOutput)]
pub fn key_output(scheme_id: &str, layer: &str, key: &str) -> Result<String, JsError> {
    let s = load(scheme_id).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(s.key_output(parse_layer(layer), key)
        .unwrap_or("")
        .to_string())
}

// ---------------------------------------------------------------------------
// Custom / third-party schemes — registered at runtime (host owns the store)
// ---------------------------------------------------------------------------

/// Register a custom scheme `id` from its TOML (same format as bundled). Throws on collision with a
/// built-in / an already-registered id, or invalid TOML.
#[wasm_bindgen(js_name = registerScheme)]
pub fn register_scheme_js(scheme_id: &str, toml: &str) -> Result<(), JsError> {
    register_scheme(scheme_id, toml).map_err(|e| JsError::new(&e.to_string()))
}

/// Unregister a custom scheme (no-op if absent / built-in).
#[wasm_bindgen(js_name = unregisterScheme)]
pub fn unregister_scheme_js(scheme_id: &str) {
    unregister_scheme(scheme_id);
}

/// Drop all custom schemes (built-ins untouched) — for deterministic re-hydration.
#[wasm_bindgen(js_name = clearCustomSchemes)]
pub fn clear_custom_schemes() {
    clear_custom();
}

/// Whether `id` is a host-registered (custom) scheme.
#[wasm_bindgen(js_name = isCustom)]
pub fn is_custom_js(scheme_id: &str) -> bool {
    is_custom(scheme_id)
}

// ---------------------------------------------------------------------------
// Session — the stateful keys-scheme driver
// ---------------------------------------------------------------------------

/// The per-key action from the engine (mirrors the core `Response`):
/// - `action`: `"passthrough"` | `"insert"` | `"replace"` | `"needs_context"`.
/// - `deleteUnits`: replace only. From `resolve` it counts trailing Unicode scalars of the prefix you
///   passed; from `feed`/`backspace` it counts committed history units.
/// - `text`: text to insert (insert/replace), empty otherwise.
#[wasm_bindgen(getter_with_clone)]
pub struct KeyResponse {
    pub action: String,
    #[wasm_bindgen(js_name = deleteUnits)]
    pub delete_units: usize,
    pub text: String,
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

/// Stateful keys-scheme session (mirrors what the IMK controller does).
#[wasm_bindgen]
pub struct Session {
    inner: CoreSession,
}

#[wasm_bindgen]
impl Session {
    /// Create a session for a scheme id (bundled or custom). Throws on unknown id.
    #[wasm_bindgen(constructor)]
    pub fn new(scheme_id: &str) -> Result<Session, JsError> {
        let s = load(scheme_id).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Session {
            inner: CoreSession::new(s),
        })
    }

    /// Feed a key (US-QWERTY label + layer). Returns the per-key edit; read `output()` for the buffer.
    pub fn feed(&mut self, key: &str, layer: &str) -> KeyResponse {
        to_response(self.inner.feed(KeyEvent::new(key, parse_layer(layer))))
    }

    /// Phase 1 of the context-aware path: resolve a key *without* the document. Returns `insert`/
    /// `passthrough`, or `needs_context` — then read the text before the caret and call `resolve`.
    pub fn peek(&mut self, key: &str, layer: &str) -> KeyResponse {
        to_response(self.inner.peek(KeyEvent::new(key, parse_layer(layer))))
    }

    /// Phase 2 of the context-aware path: given `prefix` (Thaana before the caret), match reach-back
    /// rules. Returns `replace` on a hit (the `aa → ާ` double-tap) or a fresh `insert`.
    pub fn resolve(&mut self, key: &str, layer: &str, prefix: &str) -> KeyResponse {
        to_response(
            self.inner
                .resolve(KeyEvent::new(key, parse_layer(layer)), prefix),
        )
    }

    /// Delete the last composed unit. Returns the edit to apply.
    pub fn backspace(&mut self) -> KeyResponse {
        to_response(self.inner.backspace())
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    pub fn output(&self) -> String {
        self.inner.output().to_string()
    }

    #[wasm_bindgen(js_name = setSmartQuotes)]
    pub fn set_smart_quotes(&mut self, on: bool) {
        self.inner.set_smart_quotes(on);
    }

    #[wasm_bindgen(js_name = setBracketFlip)]
    pub fn set_bracket_flip(&mut self, on: bool) {
        self.inner.set_bracket_flip(on);
    }

    #[wasm_bindgen(js_name = setTransform)]
    pub fn set_transform(&mut self, name: &str, on: bool) {
        self.inner.set_transform(name, on);
    }

    #[wasm_bindgen(js_name = setRule)]
    pub fn set_rule(&mut self, name: &str, on: bool) {
        self.inner.set_rule(name, on);
    }

    /// Set a toggleable option by name, whether it's a transform or an optional rule (names come from
    /// `schemeOptions`).
    #[wasm_bindgen(js_name = setOption)]
    pub fn set_option(&mut self, name: &str, on: bool) {
        self.inner.set_option(name, on);
    }
}
