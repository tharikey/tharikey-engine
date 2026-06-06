---
title: "Malé Latin"
---

`male-latin` · **text** · reversible (lossy) · the everyday 1976 romanization — the transliteration
workhorse. Type Latin, the abugida processor composes Thaana.

```text
raajje   →  ރާއްޖެ
dhivehi  →  ދިވެހި
maale    →  މާލެ
```

## Consonants

Digraphs win over single letters via longest match (`dh` before `d`, `th` before `t`, `sh` before `s`):

| | | | | | | | |
|--|--|--|--|--|--|--|--|
| `h` ހ | `n` ނ | `r` ރ | `b` ބ | `k` ކ | `v` ވ | `m` މ | `f` ފ |
| `l` ލ | `g` ގ | `s` ސ | `z` ޒ | `y` ޔ | `p` ޕ | `j` ޖ | `t` ޓ |
| `d` ޑ | `dh` ދ | `th` ތ | `sh` ށ | `lh` ޅ | `gn` ޏ | `ch` ޗ | |

Arabic-loan letters: `q` → ޤ , `w` → ޥ (loan waaw, distinct from `v` → ވ), `kh` → ޚ .

## Vowels

| short | | long | |
|--|--|--|--|
| `a` ަ | `e` ެ | `aa` ާ | `ey` ޭ |
| `i` ި | `o` ޮ | `ee` ީ | `oa` ޯ |
| `u` ު | | `oo` ޫ | |

## Word-final glottals

A word-final glottal stop is spelled by the **preceding vowel** — corpus-derived, no lexicon (see
[the architecture](../architecture.md#coda-orthography-rules-not-a-lexicon)):

| preceding vowel | letter | example |
|--|--|--|
| `a` · `o` · `u` | ށ shaviyani | `rah → ރަށް` |
| `e` · `ey` | އ alifu (the indefinite `-eh`) | `ekeh → އެކެއް` |
| `i` · `ee` | ތ thaa | `hih → ހިތް` |

A word-final **glide** `iy` becomes ތ too (`raiy → ރަތް`), while the bare diphthong `-ai` is left as a
real vowel.

## Composition niceties

- **Gemination** — a doubled consonant → `އް` + the consonant (`raajje → ރާއްޖެ`, `bappa → ބައްޕަ`);
  doubled nasals use `ން` (`mamma → މަންމަ`).
- **Prenasalisation** — `'` drops a coda for hus-noonu: `dhan'du → ދަނޑު`.
- **Rufiyaa sign** — `$` → ⃂ .

## Reversibility

Malé Latin is reversible and **glottal-aware** on the way back (`ރަށް → "rah"`, not "rash"). It is
**lossy on word-final glottals** — `rah` and `rash` both produce `ރަށް`, so reverse picks one reading.
Everything else round-trips.

## Limitations

Malé Latin is an *everyday* romanization, and it has two honest gaps.

- **The *thiki* letters aren't here.** The ten Arabic-loan consonants — ޘ ޙ ޛ ޜ ޝ ޞ ޟ ޠ ޡ ޢ, used in
  Qur'anic and Arabic-derived text — have no everyday Latin form. Representing them would need obscure
  combining diacritics nobody types casually, and the obvious ALA-LC choices collide with the native
  letters (the same reason [ISO 15919](iso15919.md) omits them). To *type* thiki, use the
  [Phonetic](phonetic.md) or [Typewriter](typewriter.md) layout — they're on the Shift layer.
- **The lexical tail.** Deterministic transliteration only goes so far: loanword spellings (English
  borrowings), indefinite stem alternations (`mas → maheh`), and the residual glottal ambiguity are
  *lexical* — they need a dictionary, not a rule. Malé Latin reaches **≈56% word-level exact match**
  against a human corpus; the rest is deliberately
  [out of the engine's scope](../architecture.md#design-notes), left to a layer on top.

These are by design — the engine stays small and deterministic, and is honest about where a word needs
knowledge it doesn't have.
