---
title: "Getting started"
---

ThariKey is a Cargo workspace (`tharikey-engine`); the engine itself is the `tharikey-core` crate, with
thin language bindings around it. The schemes are embedded in the binary, so there is nothing to install
at runtime and no filesystem access.

## The shape of the API

Every binding exposes the same small surface:

- `transliterate(scheme_id, text)` — Latin → Thaana via a `text` scheme.
- `reverse(scheme_id, text)` — Thaana → Latin for a reversible scheme.
- `list_schemes()` — the bundled, usable schemes.
- `Session` — a stateful driver for a `keys` scheme (what an IME feeds key-by-key).

## Rust

Depend on it from git (Cargo builds it from source):

```toml
# Cargo.toml
[dependencies]
tharikey-core = { git = "https://github.com/tharikey/tharikey-engine", package = "tharikey-core" }
# pin a tag/branch/rev for reproducibility
```

```rust
use tharikey_core::{load_seed, transliterate, reverse};

let scheme = load_seed("male-latin").unwrap();
assert_eq!(transliterate(&scheme, "raajje"), "ރާއްޖެ");
assert_eq!(reverse(&scheme, "ދިވެހި").as_deref(), Some("dhivehi"));
```

A stateful keys session:

```rust
use tharikey_core::{load_seed, KeyEvent, Layer, Session};

let scheme = load_seed("phonetic").unwrap();
let mut s = Session::new(scheme);
for k in ["b", "a", "s", "q"] {        // q = sukun on the phonetic layout
    s.feed(KeyEvent::new(k, Layer::Base));
}
assert_eq!(s.output(), "ބަސް");
```

## Python

Install from git — maturin builds it from source, so a **Rust toolchain is required** on the machine:

```sh
pip install "git+https://github.com/tharikey/tharikey-engine.git#subdirectory=bindings/python"
```

```python
import tharikey

tharikey.transliterate("male-latin", "raajje")   # 'ރާއްޖެ'
tharikey.reverse("male-latin", "ދިވެހި")          # 'dhivehi'

s = tharikey.Session("phonetic")
for k in "basq":
    s.feed(k, "base")
s.output()                                         # 'ބަސް'
```

## Command line

The crate ships a small CLI for quick checks:

```console
$ tharikey list
$ tharikey transliterate --scheme male-latin "raajje"
$ tharikey reverse       --scheme male-latin "ރާއްޖެ"
```

See [Bindings](bindings.md) for the C ABI (native / Swift / C++ / Android) and the wasm build that
powers the playground.
