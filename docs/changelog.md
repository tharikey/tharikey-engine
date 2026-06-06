# Changelog

## Unreleased

**Schemes**

- **Runtime scheme registry.** A host can register custom / third-party schemes at runtime
  (`register_scheme(id, toml)`) — the *same* TOML format as bundled schemes (no second standard),
  resolved through the same `base` inheritance, working everywhere a bundled id does (listing, sessions,
  key output). The registry is process-global and **ephemeral** (the engine still does no I/O — the host
  owns persistence and re-hydration); ids are collision-checked against built-ins and each other. The
  engine only resolves/reports — it does **not** author schemes; the host builds the scheme TOML (same
  format) and registers it. Exposed across the C ABI (`tk_scheme_register` / `tk_scheme_unregister` /
  `tk_scheme_clear_custom` / `tk_scheme_is_custom`) and the Swift wrapper.

## 0.1.0

The first release of the ThariKey engine.

**Transliteration**

- Latin → Thaana composition via a deterministic abugida processor (gemination, prenasalisation,
  standalone vowels, default sukun).
- Coda orthography as an ordered, first-match-wins **context-rule list** (`pre { tok } suf > emit`):
  the word-final glottal letter chosen from the preceding vowel (≈90%/bucket, corpus-mined), a
  word-final glide (`raiy → ރަތް`), word-final `y` → eebeefili (`dheny → ދެނީ`), and medial `h` → the
  geminate marker (`huhdha → ހުއްދަ`, the way humans write gemination). ≈56% word-level exact match
  (per-occurrence) against a 290k-pair human corpus, deterministically, with no shipped data.
- Reverse (Thaana → Latin) for reversible schemes, glottal-aware.
- Optional **`[rewrites]`** layer (FST-inspired): ordered context-rewrites over the composed Thaana,
  **data-mined and net-scored** offline. The bundled `male-latin-corpus` (= `male-latin` + 99 mined
  rules) reaches **+4.14 pts word-exact on a held-out split**, lexicon-free.

**Schemes**

- Seven bundled schemes: Phonetic, Typewriter, Dives Akuru (keys); Malé Latin, Malé Latin (corpus),
  Áletinu, ISO 15919 (text).
- Schemes are TOML, resolved into a typed model where illegal states are unrepresentable.
- Stateful keyboard sessions with modifier layers, long-press variants, optional sequence rules, and a
  toggleable `[transforms]` layer — smart quotes, RTL bracket flip, and the `$`→rufiyaa substitution
  (long-press for the literal `$` / `€`). Explicit inserts bypass transforms.
- Two keys-path drivers: the stateful `feed`/`backspace` buffer (wasm, Python, CLI) and a **bufferless
  `peek`/`resolve` context handshake** for input methods — the reach-back double-tap (`aa → ާ`) matches the
  live document before the caret (`TkResponse`/`KeyResponse` `needsContext` → `resolve`), so it stays
  correct on mid-word edits.

**Bindings**

- A C ABI (native — Swift, C++, Android JNI) with a thin Swift wrapper.
- Python (PyO3) and WebAssembly (wasm-bindgen) bindings.
