# Bindings

Thin language wrappers over the `tharikey-core` crate. Each binding exposes the **same small uniform
surface** (`transliterate`, `reverse`, `list_schemes`, `load_scheme`, and a stateful `Session`) and
depends on the core via `tharikey-core = { path = "../../crates/tharikey-core" }` — never the reverse.
The core stays I/O-free (schemes embedded), so the wrappers are mechanical.

| dir | crate | tooling | target |
|-----|-------|---------|--------|
| `c/` | `tharikey-c` | `extern "C"` + cbindgen | **The native substrate** — Swift (macOS IME, via the thin `swift/Tharikey.swift` wrapper), C++, and Android JNI all reuse this one flat C ABI |
| `wasm/` | `tharikey-wasm` | wasm-bindgen / wasm-pack | JS/TS (browser, node, npm) + portable wasm; powers the playground |
| `python/` | `tharikey-py` | PyO3 / maturin | Python — the data pipeline (radheef / spell-check), NLP/research |

**Why C, not UniFFI, for the native path:** UniFFI added a generated-scaffolding + runtime layer and a
full-Xcode dependency for codegen, for marshalling overhead that only matters on throughput paths (bulk
transliterate, radheef search) — at keystroke cadence it's negligible either way. A hand-written C ABI
is leaner, has no codegen step, and is the **universal substrate**: Swift/C++/JNI/ctypes all speak C,
so Android *reuses* it (a thin JNI shim) rather than re-implementing. The `c/` crate ships the staticlib
+ cdylib, a cbindgen-generated `include/tharikey.h`, a `swift/` module map + ergonomic wrapper. Memory
contract: every returned `char*` is caller-owned (`tk_string_free`); every `TkSession*` is freed with
`tk_session_free`; all functions are null-safe.

**Distribution** (full table in [docs/bindings.md](../docs/bindings.md)): Rust + Python from **git**
(crates.io / PyPI planned); **wasm** → npm `@tharikey/engine`; **C ABI** → GitHub Release assets
(header + universal lib + xcframework); **Android** → Maven `.aar` over the C ABI (planned). One engine
version per git tag. Install commands: [getting-started](../docs/getting-started.md).
