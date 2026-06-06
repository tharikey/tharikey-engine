---
title: "ThariKey"
---

**A small, data-driven Thaana transliteration & keymap engine.**

ThariKey turns Latin into Thaana (and back), and drives keyboard layouts — from one tiny, deterministic
Rust core that embeds its schemes and touches no filesystem. From that core, thin bindings reach native
code (a C ABI for Swift, C++, and Android), the web (WebAssembly), and Python.

<div class="tk-hero" markdown>

**Transliterate**


```text
raajje      →  ރާއްޖެ
dhivehi     →  ދިވެހި
rah         →  ރަށް
```

**Type (keys)**


```text
b a s q     →  ބަސް   (Phonetic layout, q = sukun)
```

</div>

[Getting started →](getting-started.md){ .md-button }

## Why it's built this way

- **Deterministic, not a model.** Composition is a fixed abugida processor over a longest-match token
  stream. Predictable, fast, and explainable — every output traces to a rule or a scheme entry.
- **Schemes are data.** Layouts and romanizations live in commented [TOML](scheme-format.md),
  resolved into a typed model where illegal states can't exist. Adding a scheme is data, not code.
- **One core, every surface.** The engine is owned and I/O-free, which makes the bindings mechanical —
  a [C ABI](bindings.md) for native (Swift / C++ / Android), plus Python and wasm.
- **Corpus-grounded.** The trickiest call — which letter spells a word-final glottal — is solved by a
  rule mined from ~290k human transliterations, not a dictionary. See
  [the architecture](architecture.md).

## What's in the box

Six bundled schemes — two keyboard layouts, four romanizations — and a reversible Malé-Latin path:

| Scheme | Kind | What it is |
|--------|------|-----------|
| **Phonetic** | keys | The common Dhivehi phonetic layout |
| **Typewriter** | keys | The classic typewriter layout |
| **Malé Latin** | text | The everyday 1976 romanization (reversible) |
| **Áletinu** · **ISO 15919** · **Dives Akuru** | text / keys | Experimental & scholarly |

See [bundled schemes](schemes/index.md) for the full list.

## Influences

- Composition UX (backspace deletes a composed unit, not a codepoint) — **UniKey**.
- Scheme/processor split with first-class reverse — **Lipika**.
- Romanization schemes — **ITRANS**, **Harvard-Kyoto**, **IAST / ISO 15919**.
- The glottal and indefinite phenomena — **Sonja Fritz's** grammar.

[Prior art & references](references.md) has the full list.

:::note[Scope]
ThariKey is the **engine** — transliteration and keymaps. Lexical features (loanword spelling, a
dictionary) are deliberately out of scope, so the core stays small and deterministic; those belong
in a layer built on top.
:::
