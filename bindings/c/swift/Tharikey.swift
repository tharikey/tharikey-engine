// Ergonomic Swift surface over the ThariKey C ABI (`tharikey.h` via the `TharikeyC` module map).
// The macOS IME / GUI use these types; raw C pointers never escape this file.
//
// Build: link `libtharikey.a` (from `cargo build -p tharikey-c`) and expose `include/tharikey.h`
// through the `TharikeyC` module (see `module.modulemap`).

import Foundation
import TharikeyC

/// Take ownership of a C string from the library, copy into a Swift `String`, and free it.
private func take(_ p: UnsafeMutablePointer<CChar>?) -> String? {
    guard let p = p else { return nil }
    defer { tk_string_free(p) }
    return String(cString: p)
}

public enum Tharikey {
    /// Latin → Thaana via a `text` scheme. `nil` on unknown scheme.
    public static func transliterate(_ schemeId: String, _ text: String) -> String? {
        take(tk_transliterate(schemeId, text))
    }

    /// Thaana → Latin for a reversible `text` scheme. `nil` if not reversible.
    public static func reverse(_ schemeId: String, _ text: String) -> String? {
        take(tk_reverse(schemeId, text))
    }

    public struct SchemeInfo {
        public let id: String
        public let name: String
        public let kind: String         // "keys" | "text"
        public let description: String  // short "what this is" blurb
        public let about: String        // longer "about this keyboard" (details + attribution)
    }

    /// The bundled, usable schemes.
    public static func schemes() -> [SchemeInfo] {
        (0..<tk_scheme_count()).compactMap { i in
            guard let id = take(tk_scheme_id_at(i)) else { return nil }
            let name = take(tk_scheme_name(id)) ?? id
            let kind = take(tk_scheme_kind(id)) ?? "keys"
            let description = take(tk_scheme_description(id)) ?? ""
            let about = take(tk_scheme_about(id)) ?? ""
            return SchemeInfo(id: id, name: name, kind: kind, description: description, about: about)
        }
    }

    /// A user-toggleable scheme option (a transform or an optional rule), with display metadata.
    public struct OptionInfo {
        public let name: String         // toggle key (for TharikeySession.setRule / setTransform)
        public let label: String        // user-facing name
        public let description: String  // "what this is"
        public let isOnByDefault: Bool
    }

    /// The Thaana a key produces in a layer (`base`/`shift`/`opt`/`shift_opt`) — for the on-screen
    /// keyboard preview. Empty if the key is unmapped or this is a text scheme.
    public static func keyOutput(_ schemeId: String, layer: String, key: String) -> String {
        take(tk_scheme_key_output(schemeId, layer, key)) ?? ""
    }

    /// The toggleable options for a scheme (transforms + optional rules) — empty for a text scheme.
    public static func options(_ schemeId: String) -> [OptionInfo] {
        (0..<tk_scheme_option_count(schemeId)).compactMap { i in
            guard let name = take(tk_scheme_option_name_at(schemeId, i)) else { return nil }
            let label = take(tk_scheme_option_label_at(schemeId, i)) ?? name
            let description = take(tk_scheme_option_desc_at(schemeId, i)) ?? ""
            return OptionInfo(name: name, label: label, description: description,
                              isOnByDefault: tk_scheme_option_default_at(schemeId, i))
        }
    }

    // MARK: Custom / third-party schemes (registered at runtime; the host owns the store)

    /// Registration failed — carries the engine's clean message (collision / invalid TOML).
    public struct RegisterError: Error { public let message: String }

    /// Register a custom scheme from its TOML (same format as bundled schemes). Throws `RegisterError`
    /// on a clean failure (id collides with a built-in or another registered scheme, or invalid TOML).
    /// Once registered, the id works everywhere a bundled id does (`schemes()`, sessions, key output…).
    public static func registerScheme(_ id: String, toml: String) throws {
        if let err = take(tk_scheme_register(id, toml)) {
            throw RegisterError(message: err)
        }
    }

    public static func unregisterScheme(_ id: String) { tk_scheme_unregister(id) }

    /// Drop all custom schemes (built-ins untouched) — for deterministic re-hydration at host start.
    public static func clearCustomSchemes() { tk_scheme_clear_custom() }

    public static func isCustom(_ id: String) -> Bool { tk_scheme_is_custom(id) }

    // The engine does not author/edit schemes. The host (the app) builds the scheme TOML itself and
    // registers it; these accessors just *report* a registered scheme's resolved metadata.

    /// Direct metadata accessors by id (resolve through the registry) — for seeding the editor with a
    /// scheme's current values without going through the cached `schemes()` list.
    public static func name(_ id: String) -> String { take(tk_scheme_name(id)) ?? id }
    public static func schemeDescription(_ id: String) -> String { take(tk_scheme_description(id)) ?? "" }
    public static func about(_ id: String) -> String { take(tk_scheme_about(id)) ?? "" }
}

/// The per-key edit to apply (mirrors the engine's `Response`). The IMK controller turns this into the
/// exact document mutation: `insert` appends; `replace` retracts `deleteUnits` units then inserts;
/// `needsContext` means "this key could reach back — read the text before the caret and call
/// `resolve`" (returned only by `peek`).
public enum KeyResponse: Equatable {
    case passthrough
    case insert(String)
    /// Delete `deleteUnits` units then insert `text`. From `resolve`, `deleteUnits` counts the
    /// **trailing Unicode scalars of the prefix you passed in** (the `aa→ާ` double-tap deletes the `ަ`).
    case replace(deleteUnits: Int, text: String)
    /// `peek` deferred: the key is a reach-back candidate; read the document context and call `resolve`.
    case needsContext

    fileprivate init(_ r: TkResponse) {
        let text = take(r.text) ?? ""   // takes ownership of + frees r.text
        switch r.action {
        case 1:  self = .insert(text)
        case 2:  self = .replace(deleteUnits: Int(r.delete_units), text: text)
        case 3:  self = .needsContext
        default: self = .passthrough
        }
    }
}

/// Stateful keys-scheme session (what the IMK controller drives). Frees the native handle on deinit.
public final class TharikeySession {
    private let handle: OpaquePointer

    /// `nil` if the scheme id is unknown.
    public init?(_ schemeId: String) {
        guard let h = tk_session_new(schemeId) else { return nil }
        handle = h
    }

    deinit { tk_session_free(handle) }

    /// Feed a key (US-QWERTY label + layer). Returns the per-key edit to apply.
    public func feed(key: String, layer: String) -> KeyResponse {
        KeyResponse(tk_session_feed(handle, key, layer))
    }

    /// Context-aware path, phase 1: resolve a key *without* the document. Returns a finished
    /// `insert`/`passthrough`, or `needsContext` when the key could reach back (then read the text
    /// before the caret and call `resolve`). No committed buffer — nothing goes stale on a mid-word edit.
    public func peek(key: String, layer: String) -> KeyResponse {
        KeyResponse(tk_session_peek(handle, key, layer))
    }

    /// Context-aware path, phase 2: given the Thaana `prefix` immediately before the caret, match the
    /// active reach-back rules. Returns `replace` on a hit (the double-tap) or a fresh `insert`.
    public func resolve(key: String, layer: String, prefix: String) -> KeyResponse {
        KeyResponse(tk_session_resolve(handle, key, layer, prefix))
    }

    /// Delete the last composed unit. Returns the edit to apply (usually a `replace` of 1 unit).
    public func backspace() -> KeyResponse { KeyResponse(tk_session_backspace(handle)) }

    public func reset() { tk_session_reset(handle) }

    public var output: String { take(tk_session_output(handle)) ?? "" }

    public func setSmartQuotes(_ on: Bool) { tk_session_set_smart_quotes(handle, on) }
    public func setBracketFlip(_ on: Bool) { tk_session_set_bracket_flip(handle, on) }
    public func setRule(_ name: String, _ on: Bool) { tk_session_set_rule(handle, name, on) }

    /// Set a toggleable option by name (transform or optional rule). What the IME calls to apply a
    /// scheme's options from the user's saved settings.
    public func setOption(_ name: String, _ on: Bool) { tk_session_set_option(handle, name, on) }
}
