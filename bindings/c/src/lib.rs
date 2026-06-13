//! C ABI binding for the ThariKey engine — the reusable native substrate.
//!
//! Swift (the macOS IME), C++, and Android (JNI) all consume this one flat C interface; a thin Swift
//! wrapper (`swift/Tharikey.swift`) gives the macOS app an ergonomic surface over it. The core engine
//! is already owned + I/O-free, so this layer is pure marshalling — no engine changes needed.
//!
//! ## Memory & null contract
//! - Every `char*` this library returns is **caller-owned**; free it with [`tk_string_free`].
//! - Every `TkSession*` from [`tk_session_new`] must be freed with [`tk_session_free`].
//! - All functions are null-safe: a null/invalid pointer yields a null/empty result, never UB.
//!
//! These functions dereference raw pointers from C by contract (the caller upholds validity), so they
//! are declared safe `extern "C"` for ergonomic FFI — hence the crate-level allow below.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::ffi::{c_char, CStr, CString};
use std::ptr;
use tharikey_core::{
    all_scheme_ids, clear_custom, is_custom, load, register_scheme, reverse as core_reverse,
    transliterate as core_transliterate, unregister_scheme, KeyEvent, Layer, Response, Scheme,
    Session as CoreSession,
};

/// Borrow a C string as `&str` (None if null or not valid UTF-8). Valid for the call's duration.
unsafe fn as_str<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    CStr::from_ptr(p).to_str().ok()
}

/// Move a Rust `String` into a freshly-allocated, caller-owned C string (free with `tk_string_free`).
fn to_c(s: String) -> *mut c_char {
    // Our outputs (Thaana / Latin / ids) never contain interior NULs, so this won't fail in practice.
    CString::new(s).map_or(ptr::null_mut(), CString::into_raw)
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

/// Free a string returned by this library. Passing null is a no-op.
#[no_mangle]
pub extern "C" fn tk_string_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe { drop(CString::from_raw(s)) };
    }
}

/// Engine version (the unified workspace version, e.g. "0.1.2"). Returns a **static**, NUL-terminated
/// string that is NOT caller-owned — do not pass it to `tk_string_free`. Equals `tharikey_core::version()`.
#[no_mangle]
pub extern "C" fn tk_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

// ---------------------------------------------------------------------------
// One-shot text transforms
// ---------------------------------------------------------------------------

/// Latin -> Thaana via a `text` scheme. Owned C string, or null on unknown scheme.
#[no_mangle]
pub extern "C" fn tk_transliterate(scheme_id: *const c_char, text: *const c_char) -> *mut c_char {
    let (Some(id), Some(text)) = (unsafe { as_str(scheme_id) }, unsafe { as_str(text) }) else {
        return ptr::null_mut();
    };
    match load(id) {
        Ok(scheme) => to_c(core_transliterate(&scheme, text)),
        Err(_) => ptr::null_mut(),
    }
}

/// Reverse (Thaana -> Latin) for a reversible `text` scheme. Owned C string, or null if not reversible
/// / unknown scheme.
#[no_mangle]
pub extern "C" fn tk_reverse(scheme_id: *const c_char, text: *const c_char) -> *mut c_char {
    let (Some(id), Some(text)) = (unsafe { as_str(scheme_id) }, unsafe { as_str(text) }) else {
        return ptr::null_mut();
    };
    match load(id) {
        Ok(scheme) => core_reverse(&scheme, text).map_or(ptr::null_mut(), to_c),
        Err(_) => ptr::null_mut(),
    }
}

// ---------------------------------------------------------------------------
// Scheme listing (count + accessors — no JSON, idiomatic C)
// ---------------------------------------------------------------------------

/// Number of usable schemes — bundled plus any host-registered (custom) ones.
#[no_mangle]
pub extern "C" fn tk_scheme_count() -> usize {
    all_scheme_ids().len()
}

/// Id of the usable scheme at `index` (owned C string), or null if out of range. Bundled first, then
/// custom — so freshly registered schemes appear at the tail.
#[no_mangle]
pub extern "C" fn tk_scheme_id_at(index: usize) -> *mut c_char {
    all_scheme_ids()
        .get(index)
        .map_or(ptr::null_mut(), |id| to_c(id.clone()))
}

/// Display name of a scheme (owned C string), or null if unknown.
#[no_mangle]
pub extern "C" fn tk_scheme_name(scheme_id: *const c_char) -> *mut c_char {
    let Some(id) = (unsafe { as_str(scheme_id) }) else {
        return ptr::null_mut();
    };
    load(id).map_or(ptr::null_mut(), |s| to_c(s.meta().name.clone()))
}

/// Kind of a scheme — `"keys"` or `"text"` (owned C string), or null if unknown.
#[no_mangle]
pub extern "C" fn tk_scheme_kind(scheme_id: *const c_char) -> *mut c_char {
    let Some(id) = (unsafe { as_str(scheme_id) }) else {
        return ptr::null_mut();
    };
    match load(id) {
        Ok(Scheme::Keys(_)) => to_c("keys".to_string()),
        Ok(Scheme::Text(_)) => to_c("text".to_string()),
        Err(_) => ptr::null_mut(),
    }
}

/// Short "what this is" blurb for a scheme (owned C string; empty if none, null if unknown scheme).
#[no_mangle]
pub extern "C" fn tk_scheme_description(scheme_id: *const c_char) -> *mut c_char {
    let Some(id) = (unsafe { as_str(scheme_id) }) else {
        return ptr::null_mut();
    };
    load(id).map_or(ptr::null_mut(), |s| to_c(s.meta().description.clone()))
}

/// Longer "about this keyboard" text (details + attribution) for a scheme (owned C string; empty if
/// none, null if unknown scheme).
#[no_mangle]
pub extern "C" fn tk_scheme_about(scheme_id: *const c_char) -> *mut c_char {
    let Some(id) = (unsafe { as_str(scheme_id) }) else {
        return ptr::null_mut();
    };
    load(id).map_or(ptr::null_mut(), |s| to_c(s.meta().about.clone()))
}

// ---------------------------------------------------------------------------
// Scheme options (the toggleable transforms + optional rules) — count + accessors
// ---------------------------------------------------------------------------

/// Number of user-toggleable options for a scheme (transforms + optional rules). 0 for a text scheme or
/// an unknown id.
#[no_mangle]
pub extern "C" fn tk_scheme_option_count(scheme_id: *const c_char) -> usize {
    let Some(id) = (unsafe { as_str(scheme_id) }) else {
        return 0;
    };
    load(id).map(|s| s.options().len()).unwrap_or(0)
}

fn option_at(scheme_id: *const c_char, index: usize) -> Option<tharikey_core::OptionInfo> {
    let id = unsafe { as_str(scheme_id) }?;
    load(id).ok()?.options().into_iter().nth(index)
}

/// Toggle key of the option at `index` — the name passed to `tk_session_set_transform`/`_set_rule`
/// (owned C string), or null if out of range.
#[no_mangle]
pub extern "C" fn tk_scheme_option_name_at(scheme_id: *const c_char, index: usize) -> *mut c_char {
    option_at(scheme_id, index).map_or(ptr::null_mut(), |o| to_c(o.name))
}

/// User-facing label of the option at `index` (owned C string), or null if out of range.
#[no_mangle]
pub extern "C" fn tk_scheme_option_label_at(scheme_id: *const c_char, index: usize) -> *mut c_char {
    option_at(scheme_id, index).map_or(ptr::null_mut(), |o| to_c(o.label))
}

/// "What this is" description of the option at `index` (owned C string; empty if none, null if out of range).
#[no_mangle]
pub extern "C" fn tk_scheme_option_desc_at(scheme_id: *const c_char, index: usize) -> *mut c_char {
    option_at(scheme_id, index).map_or(ptr::null_mut(), |o| to_c(o.description))
}

/// Default on/off state of the option at `index` (false if out of range).
#[no_mangle]
pub extern "C" fn tk_scheme_option_default_at(scheme_id: *const c_char, index: usize) -> bool {
    option_at(scheme_id, index)
        .map(|o| o.default)
        .unwrap_or(false)
}

/// The Thaana a key produces in a layer (`base`/`shift`/`opt`/`shift_opt`/`caps`) — for an on-screen
/// keyboard preview. Owned C string; **empty** if the key is unmapped or this is a text scheme, **null**
/// only if the scheme id is unknown.
#[no_mangle]
pub extern "C" fn tk_scheme_key_output(
    scheme_id: *const c_char,
    layer: *const c_char,
    key: *const c_char,
) -> *mut c_char {
    let (Some(id), Some(layer), Some(key)) = (
        unsafe { as_str(scheme_id) },
        unsafe { as_str(layer) },
        unsafe { as_str(key) },
    ) else {
        return ptr::null_mut();
    };
    match load(id) {
        Ok(s) => to_c(
            s.key_output(parse_layer(layer), key)
                .unwrap_or("")
                .to_string(),
        ),
        Err(_) => ptr::null_mut(),
    }
}

// ---------------------------------------------------------------------------
// Custom / third-party schemes — registered at runtime (host owns the store)
// ---------------------------------------------------------------------------
//
// A registered scheme is the runtime twin of a bundled one (same TOML format). The host re-hydrates the
// registry on every process start. Once registered, a custom id works everywhere a bundled id does.

/// Register a custom scheme `id` from its TOML. Returns **null on success**, or an owned C string with a
/// clean error message (collision with a built-in / already registered / invalid TOML). Free with
/// [`tk_string_free`].
#[no_mangle]
pub extern "C" fn tk_scheme_register(scheme_id: *const c_char, toml: *const c_char) -> *mut c_char {
    let (Some(id), Some(toml)) = (unsafe { as_str(scheme_id) }, unsafe { as_str(toml) }) else {
        return to_c("null id or toml".to_string());
    };
    match register_scheme(id, toml) {
        Ok(()) => ptr::null_mut(),
        Err(e) => to_c(e.to_string()),
    }
}

/// Unregister a custom scheme (no-op if absent / built-in).
#[no_mangle]
pub extern "C" fn tk_scheme_unregister(scheme_id: *const c_char) {
    if let Some(id) = unsafe { as_str(scheme_id) } {
        unregister_scheme(id);
    }
}

/// Drop all custom schemes (built-ins untouched) — for deterministic re-hydration.
#[no_mangle]
pub extern "C" fn tk_scheme_clear_custom() {
    clear_custom();
}

/// Whether `id` is a host-registered (custom) scheme.
#[no_mangle]
pub extern "C" fn tk_scheme_is_custom(scheme_id: *const c_char) -> bool {
    unsafe { as_str(scheme_id) }.map(is_custom).unwrap_or(false)
}

// NOTE: the engine does NOT author or edit schemes — no custom_new / set_key / set_meta here. The host
// builds the scheme TOML (same format as bundled) and registers it. The engine only resolves + reports.

// ---------------------------------------------------------------------------
// Session — the stateful keys-scheme driver (what the IME controller calls)
// ---------------------------------------------------------------------------

/// Opaque stateful keys-scheme session. Create with [`tk_session_new`], free with [`tk_session_free`].
pub struct TkSession(CoreSession);

/// The per-key action from the engine — mirrors `Response`, so the IME applies an exact edit.
/// - `action`: 0 = passthrough (key not mapped — let it through), 1 = insert, 2 = replace,
///   3 = needs_context (from `tk_session_peek` only: read the document before the caret and call
///   `tk_session_resolve` with it).
/// - `delete_units`: replace only. From `tk_session_resolve` this counts **trailing Unicode scalars
///   of the prefix you passed in** (the host already has that text, so it maps the count to its own
///   UTF-16 range); from the stateful `tk_session_feed`/`_backspace` it counts committed history units.
/// - `text`: caller-owned C string to insert (insert/replace), null otherwise; free with
///   `tk_string_free`.
#[repr(C)]
pub struct TkResponse {
    pub action: u8,
    pub delete_units: usize,
    pub text: *mut c_char,
}

/// The inert/passthrough response — also the safe fallback if a per-key call ever panics.
fn passthrough_response() -> TkResponse {
    TkResponse {
        action: 0,
        delete_units: 0,
        text: ptr::null_mut(),
    }
}

/// Panic firewall for the per-keystroke session calls. The engine is panic-free by design, but it runs
/// **in-process inside the IME**, so a stray panic on some adversarial input must degrade to passthrough
/// (the key falls through unchanged), never unwind across the C boundary and abort the host. The session
/// may be left slightly inconsistent, but the user keeps typing. `AssertUnwindSafe`: we accept that.
fn guard_response(f: impl FnOnce() -> TkResponse) -> TkResponse {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .unwrap_or_else(|_| passthrough_response())
}

fn to_response(r: Response) -> TkResponse {
    match r {
        Response::Passthrough => TkResponse {
            action: 0,
            delete_units: 0,
            text: ptr::null_mut(),
        },
        Response::Insert(t) => TkResponse {
            action: 1,
            delete_units: 0,
            text: to_c(t),
        },
        Response::Replace { delete_units, text } => TkResponse {
            action: 2,
            delete_units,
            text: to_c(text),
        },
        Response::NeedsContext => TkResponse {
            action: 3,
            delete_units: 0,
            text: ptr::null_mut(),
        },
    }
}

/// Create a session for a scheme id. Null on unknown scheme. (A text scheme yields an inert session.)
#[no_mangle]
pub extern "C" fn tk_session_new(scheme_id: *const c_char) -> *mut TkSession {
    let Some(id) = (unsafe { as_str(scheme_id) }) else {
        return ptr::null_mut();
    };
    match load(id) {
        Ok(scheme) => Box::into_raw(Box::new(TkSession(CoreSession::new(scheme)))),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a session. Passing null is a no-op.
#[no_mangle]
pub extern "C" fn tk_session_free(s: *mut TkSession) {
    if !s.is_null() {
        unsafe { drop(Box::from_raw(s)) };
    }
}

/// Feed a key (US-QWERTY label + layer name: `base`/`shift`/`opt`/`shift_opt`/`caps`). Returns the
/// per-key edit to apply (see [`TkResponse`]).
#[no_mangle]
pub extern "C" fn tk_session_feed(
    s: *mut TkSession,
    key: *const c_char,
    layer: *const c_char,
) -> TkResponse {
    if s.is_null() {
        return passthrough_response();
    }
    let (Some(key), Some(layer)) = (unsafe { as_str(key) }, unsafe { as_str(layer) }) else {
        return passthrough_response();
    };
    let sess = unsafe { &mut (*s).0 };
    guard_response(|| to_response(sess.feed(KeyEvent::new(key, parse_layer(layer)))))
}

/// Phase 1 of the context-aware (direct-keyboard) path: resolve a key *without* the document. Returns
/// `insert`/`passthrough` for ordinary keys, or `needs_context` (action 3) if the key could reach back
/// — then the host reads the text before the caret and calls [`tk_session_resolve`]. Holds no committed
/// buffer, so nothing goes stale on a mid-word click.
#[no_mangle]
pub extern "C" fn tk_session_peek(
    s: *mut TkSession,
    key: *const c_char,
    layer: *const c_char,
) -> TkResponse {
    if s.is_null() {
        return passthrough_response();
    }
    let (Some(key), Some(layer)) = (unsafe { as_str(key) }, unsafe { as_str(layer) }) else {
        return passthrough_response();
    };
    let sess = unsafe { &mut (*s).0 };
    guard_response(|| to_response(sess.peek(KeyEvent::new(key, parse_layer(layer)))))
}

/// Phase 2 of the context-aware path: given `prefix` (the Thaana immediately before the caret), match
/// the active reach-back rules against it. Returns `replace` on a hit (the `aa → ާ` double-tap,
/// `delete_units` = trailing scalars of `prefix` to remove) or a fresh `insert` otherwise.
#[no_mangle]
pub extern "C" fn tk_session_resolve(
    s: *mut TkSession,
    key: *const c_char,
    layer: *const c_char,
    prefix: *const c_char,
) -> TkResponse {
    if s.is_null() {
        return passthrough_response();
    }
    let (Some(key), Some(layer)) = (unsafe { as_str(key) }, unsafe { as_str(layer) }) else {
        return passthrough_response();
    };
    let prefix = unsafe { as_str(prefix) }.unwrap_or(""); // null/invalid prefix = empty context
    let sess = unsafe { &mut (*s).0 };
    guard_response(|| to_response(sess.resolve(KeyEvent::new(key, parse_layer(layer)), prefix)))
}

/// Delete the last composed unit. Returns the edit to apply (typically `replace` deleting 1 unit).
#[no_mangle]
pub extern "C" fn tk_session_backspace(s: *mut TkSession) -> TkResponse {
    if s.is_null() {
        return passthrough_response();
    }
    let sess = unsafe { &mut (*s).0 };
    guard_response(|| to_response(sess.backspace()))
}

/// Reset the session to empty.
#[no_mangle]
pub extern "C" fn tk_session_reset(s: *mut TkSession) {
    if !s.is_null() {
        unsafe { (*s).0.reset() };
    }
}

/// Current committed output (owned C string).
#[no_mangle]
pub extern "C" fn tk_session_output(s: *mut TkSession) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    to_c(unsafe { (*s).0.output() }.to_string())
}

/// Enable/disable smart (curly) quotes.
#[no_mangle]
pub extern "C" fn tk_session_set_smart_quotes(s: *mut TkSession, on: bool) {
    if !s.is_null() {
        unsafe { (*s).0.set_smart_quotes(on) };
    }
}

/// Enable/disable RTL bracket-key flip.
#[no_mangle]
pub extern "C" fn tk_session_set_bracket_flip(s: *mut TkSession, on: bool) {
    if !s.is_null() {
        unsafe { (*s).0.set_bracket_flip(on) };
    }
}

/// Turn a named output transform on/off (`[transforms.<name>]`, e.g. "rufiyaa").
#[no_mangle]
pub extern "C" fn tk_session_set_transform(s: *mut TkSession, name: *const c_char, on: bool) {
    if s.is_null() {
        return;
    }
    if let Some(name) = unsafe { as_str(name) } {
        unsafe { (*s).0.set_transform(name, on) };
    }
}

/// Turn an optional sequence rule on/off by name.
#[no_mangle]
pub extern "C" fn tk_session_set_rule(s: *mut TkSession, name: *const c_char, on: bool) {
    if s.is_null() {
        return;
    }
    if let Some(name) = unsafe { as_str(name) } {
        unsafe { (*s).0.set_rule(name, on) };
    }
}

/// Set a toggleable option by name, whether it's a transform or an optional rule (the unified setter the
/// IME uses to apply a scheme's options — names come from `tk_scheme_option_name_at`).
#[no_mangle]
pub extern "C" fn tk_session_set_option(s: *mut TkSession, name: *const c_char, on: bool) {
    if s.is_null() {
        return;
    }
    if let Some(name) = unsafe { as_str(name) } {
        unsafe { (*s).0.set_option(name, on) };
    }
}
