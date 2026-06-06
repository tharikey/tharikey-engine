---
title: "Prior art & references"
---

Where ThariKey's design, schemes, and data come from.

## Input-method & transliteration design

Ideas borrowed, not code:

- **[RIME](https://rime.im)** — a schema-driven, cross-platform input-method engine with thin per-OS
  frontends (Squirrel on macOS, Weasel on Windows, …). Its "schemas as data over one portable core,
  switched at runtime" model is the shape ThariKey is growing into.
- **[Lipika](https://github.com/ratreya/lipika-ime)** — *input-scheme × pivot-script* transliteration
  with first-class reverse. ThariKey's scheme (data) / script-agnostic processor split follows it. (GPL;
  architecture only.)
- **[UniKey](https://en.wikipedia.org/wiki/UniKey)** (Vietnamese) — composition UX: backspace deletes
  the last composed unit (consonant + vowel sign), not a codepoint. The keys-mode `Session` does this.
- **CLDR/ICU transforms** and **Keyman (KMN)** — rule semantics (longest-context-first matching, dead
  keys, contextual rules). Our sequence-rule surface is a minimal version.

We evaluated **Keyman Core**, **CLDR LDML keyboards**, **m17n**, and **RIME** as engines to embed and
chose not to — their interop formats aren't worth the weight for a single script, hence ThariKey's
[own scheme format](scheme-format.md).

## Romanization standards

The `text` schemes follow scheme-based Indic romanization:

- **[Harvard-Kyoto](https://en.wikipedia.org/wiki/Harvard-Kyoto)** and
  **[ITRANS](https://en.wikipedia.org/wiki/ITRANS)** — ASCII transliteration of Devanagari/Sanskrit.
- **[IAST](https://en.wikipedia.org/wiki/International_Alphabet_of_Sanskrit_Transliteration)** and its
  successor **[ISO 15919](https://en.wikipedia.org/wiki/ISO_15919)** — the diacritic standard the
  bundled [ISO 15919 scheme](schemes/iso15919.md) follows.

## Scheme & corpus sources

| Scheme / data | Source |
|---|---|
| Phonetic layout | SIL Global — [Keyman `basic_kbddiv1`](https://keyman.com/keyboards/basic_kbddiv1) "Divehi Phonetic Basic" (MIT); base + shift follow it, departing only to place `ޱ` NAA on `x` (the letter SIL omits) |
| Typewriter layout | SIL Global — [Keyman `basic_kbddiv2`](https://keyman.com/keyboards/basic_kbddiv2) "Divehi Typewriter Basic" (MIT); base + shift match byte-for-byte |
| Dives Akuru layout | SIL Global — [Keyman `dives_akuru_inscript`](https://keyman.com/keyboards/dives_akuru_inscript) (MIT) |
| Áletinu | *thatmaldivesblog*, [New Dhivehi Latin](https://thatmaldivesblog.wordpress.com/2016/06/07/new-dhivehi-latin/) (2016) |
| ISO 15919 column | [Romanization of Dhivehi](https://en.wikipedia.org/wiki/Romanization_of_Dhivehi) (after Romero-Frías) |
| Glottal-rule corpus | [`politecat314/dhivehi-transliteration`](https://huggingface.co/datasets/politecat314/dhivehi-transliteration) — ~290k human pairs |

## Language & encoding

- **Sonja Fritz**, *The Dhivehi Language: A Descriptive and Historical Grammar of Maldivian and Its
  Dialects* (2002) — the reference grammar for the glottal and indefinite phenomena.
- *thatmaldivesblog* lessons — the **five word-final sukun letters**, gemination, and the indefinite
  `-eh`, behind the [coda rules](architecture.md#coda-orthography-rules-not-a-lexicon).
- **Anshuman Pandey**,
  [*Proposal to encode Dives Akuru in Unicode* (L2/18-016R)](https://www.unicode.org/L2/L2018/18016r-dives-akuru.pdf)
  — the model behind the [Dives Akuru scheme](schemes/dives-akuru.md) (Unicode 13.0, U+11900–U+1195F).
- The **Unicode Standard** — Thaana (U+0780–U+07BF) and Dives Akuru blocks.
- **[CLDR](https://github.com/unicode-org/cldr)** — the RTL quote conventions and Latin–Thaana transform
  behind the shared `thaana-common` defaults.

:::note[On attribution]
Keyboard layouts (key positions) are facts, not creative works, and are re-encoded as such. Sources
that carry a licence or request attribution are named above and in the scheme files.
:::
