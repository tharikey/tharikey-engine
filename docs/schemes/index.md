# Bundled schemes

Seven schemes ship embedded in the engine. Two are **keyboard layouts** (`keys`) and five are
**romanizations** (`text`). `thaana-common` is a shared base fragment (RTL punctuation + the shared
`[transforms]`: smart-quote glyphs, bracket flip, the rufiyaa substitution), not a usable scheme on its
own.

| scheme | kind | Reversible | Summary |
|----|------|:---:|---------|
| [**Phonetic**](phonetic.md) | keys | — | The common Dhivehi phonetic keyboard layout. Thiki letters on Shift; `ޱ` NAA on `x`; optional `aa → aabaafili` double-tap. |
| [**Typewriter**](typewriter.md) | keys | — | The standard typewriter layout — filis on the left hand, consonants on the right. |
| [**Malé Latin**](male-latin.md) | text | ✓ (lossy) | The everyday 1976 romanization — `dh/th/sh/lh` digraphs, gemination, prenasalisation, corpus-derived word-final glottals. The workhorse. |
| [**Malé Latin (corpus)**](male-latin-corpus.md) | text | — | `male-latin` plus a data-mined `[rewrites]` correction layer (+4.14 pts word-exact, held-out). Corpus-tuned, lexicon-free. |
| [**Áletinu**](aletinu.md) | text | ✓ (lossy) | **Experimental.** A strict one-sound-one-letter romanization using diacritic letters. |
| [**ISO 15919**](iso15919.md) | text | ✓ (lossy) | **Scholarly.** The ISO 15919 academic transliteration (single letters + diacritics). |
| [**Dives Akuru**](dives-akuru.md) | keys | — | A layout for the historic **Dives Akuru** script (SMP codepoints) — not a Thaana→Dives transliteration. |

## Keys vs text

- **`keys`** schemes are physical-layout tables (one output per key + modifier layer). They drive a live
  [`Session`](../getting-started.md) — each keypress maps 1:1, plus optional toggleable sequence rules.
- **`text`** schemes are romanization maps plus an abugida composer. They run one-shot
  `transliterate` / `reverse`. See [the architecture](../architecture.md) for how composition
  works.

## Malé Latin at a glance

```text
raajje   →  ރާއްޖެ      (gemination: jj → އް + ޖ)
rah      →  ރަށް        (word-final glottal: a + h → shaviyani)
ekeh     →  އެކެއް       (word-final glottal: e + h → alifu, the indefinite -eh)
```

Malé Latin is reversible, but **lossy on word-final glottals** — `rah` and `rash` both produce `ރަށް`,
so reverse picks one reading. Everything else round-trips.

## Adding your own

Schemes are [TOML data](../scheme-format.md). A new layout or romanization is a new file —
no engine code.
