# ThariKey Engine

A small, fast, **data-driven Thaana (Dhivehi) transliteration / keymap engine** in Rust.

> _ThariKey_ — ތަރި (thari, "star") + key — and a nod to ތާރީޚީ (*thaareekhee*, "historical").

The engine turns a stream of input tokens (physical keys *or* Latin text) into Thaana, driven entirely
by **scheme files** (TOML). A "direct keyboard layout" is just the trivial scheme; richer schemes add
sequences, abugida composition, and reverse transliteration. It has no UI or platform dependencies — it
backs the macOS input method (`tharikey-macos`), a future WASM/React demo (`tharikey-react`), the CLI
here, and tests.

## Layout

`tharikey-engine` is the umbrella repo — a Cargo **workspace**, not a single crate. Two trees:

```
crates/
  tharikey-core/      # the transliteration / keymap engine — pure Rust (serde + toml), lexicon-free
bindings/
  c/      python/  wasm/   # FFI wrappers → header+xcframework / wheel / npm pkg (built with their own tools)
```

`crates/` holds the portable Rust libraries; `bindings/` holds the foreign-language package roots (each
wraps the core but ships a non-Rust artifact). Suggestions are deliberately separate: the core never
memorises a corpus, so keeping the data-driven suggestion engine in its own crate makes that boundary a
compile-time fact and lets lean consumers (the wasm playground) skip it entirely.

## Schemes

| id | kind | what |
|----|------|------|
| `phonetic` | keys | Thaana phonetic layout (seeded from `kudanai/Thaana-OSX`) |
| `typewriter` | keys | standard MV typewriter layout |
| `dives-akuru` | keys | the historic Dives Akuru script (InScript; emits SMP codepoints) |
| `male-latin` | text | Malé Latin (1976) romanization — reversible |
| `male-latin-corpus` | text | `male-latin` + a data-mined, net-scored `[rewrites]` layer (+4.14 pts held-out) |
| `aletinu` | text | "New Dhivehi Latin" (Áletinu), attributed to thatmaldivesblog |
| `iso15919` | text | ISO 15919 scholarly transliteration |

Shared conventions (RTL punctuation, plus the `[transforms]` layer — smart quotes, bracket flip, the
`$`→rufiyaa substitution) live once in `thaana-common` and are inherited via `base`.

## Use

```sh
cargo test                                   # golden tests
cargo run --bin tharikey -- list
cargo run --bin tharikey -- transliterate --scheme male-latin "raajje"   # ރާއްޖެ
cargo run --bin tharikey -- reverse       --scheme male-latin "ދިވެހި"   # dhivehi
cargo run --bin tharikey -- validate      --scheme phonetic
```

```rust
use tharikey_core::{load_seed, transliterate};
let s = load_seed("male-latin").unwrap();
assert_eq!(transliterate(&s, "bas"), "ބަސް");
```

## Install as a dependency

Consume the engine from git (both toolchains build it from source):

```toml
# Rust — Cargo.toml
tharikey-core = { git = "https://github.com/tharikey/tharikey-engine", package = "tharikey-core" }
# pin a tag/branch/rev for reproducibility
```

```sh
# Python — maturin builds from source, so a Rust toolchain is required
pip install "git+https://github.com/tharikey/tharikey-engine.git#subdirectory=bindings/python"
```

The **wasm** binding is published to npm as **`@tharikey/engine`** (`npm install @tharikey/engine`); the
**C ABI** is a cbindgen-generated header + library consumed by the macOS app. See
[`bindings/`](bindings/README.md).

## Features

- **Two input kinds, one engine:** `keys` (layer tables) and `text` (maps + abugida composition).
- **Role-based abugida processor** (consonant + fili, default sukun, vowel carriers, gemination) —
  references abstract roles per output script, not hardcoded Thaana literals.
- **Reverse transliteration** for reversible text schemes (incl. gemination).
- Stateful keys session: unit-wise backspace, toggleable sequence niceties (e.g. `aa → aabaafili`),
  a toggleable `[transforms]` layer (smart quotes, RTL bracket-key flip, `$`→rufiyaa), long-press
  variants (which bypass transforms).
- **Scheme inheritance** (`base`) for aligned, shared defaults.

## Releasing

Cut from `main` (GitHub-flow — no `develop`). One `[workspace.package] version` drives the git tag and
every published artifact; crates inherit it via `version.workspace = true`.

1. **`scripts/bump-version.sh`** — verifies `main` is clean and in sync, prompts for the new version,
   branches `release/x.y.z`, and bumps the workspace version.
2. Add a `## x.y.z` entry to [`docs/changelog.md`](docs/changelog.md), then commit + push + open a PR
   to `main`.
3. On merge, CI ([`release.yml`](.github/workflows/release.yml)) tags `vx.y.z` and publishes
   (`@tharikey/engine` → npm). Idempotent — it only fires when the version changed.

License: MIT.

