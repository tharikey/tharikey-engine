---
title: "Phonetic"
---

`phonetic` · **keys** · the common Dhivehi phonetic keyboard layout — letters sit on the Latin keys
they sound like (`h → ހ`, `b → ބ`, `k → ކ`).

The base and shift layers follow **SIL Global's "Divehi Phonetic Basic"** (Keyman `basic_kbddiv1`, MIT),
with one deliberate departure: **`ޱ` NAA on `x`** — the single Thaana letter SIL leaves unmapped (Apple's
Dhivehi-QWERTY places it here too; `x`'s old `×` was legacy cruft, dropped). ThariKey **adds**: `ﷲ` on
++shift+f++, a **full Arabic layer on ++option++ / ++shift+option++** (borrowed from Apple's Dhivehi-QWERTY),
the optional long-vowel double-tap, the rufiyaa sign, and RTL bracket-flip (inherited from `thaana-common`).

## Base layer

Consonants:

| key | | key | | key | | key | |
|--|--|--|--|--|--|--|--|
| `h` | ހ | `s` | ސ | `d` | ދ | `f` | ފ |
| `g` | ގ | `z` | ޒ | `c` | ޗ | `v` | ވ |
| `b` | ބ | `r` | ރ | `y` | ޔ | `t` | ތ |
| `p` | ޕ | `l` | ލ | `j` | ޖ | `k` | ކ |
| `n` | ނ | `m` | މ | `x` | ޱ (naa) | | |

Vowels & marks:

| key | glyph | name |
|--|--|--|
| `a` | ަ | abafili |
| `e` | ެ | ebefili |
| `i` | ި | ibifili |
| `o` | ޮ | obofili |
| `u` | ު | ubufili |
| `w` | އ | alifu (vowel carrier) |
| `q` | ް | sukun |

## Shift layer

Long vowels and the Arabic-loan (*thiki*) consonants:

| key | glyph | | key | glyph | | key | glyph |
|--|--|--|--|--|--|--|--|
| `a` | ާ (aabaafili) | | `e` | ޭ (eybeyfili) | | `i` | ީ (eebeefili) |
| `o` | ޯ (oaboafili) | | `u` | ޫ (ooboofili) | | | |
| `s` | ށ shaviyani | | `d` | ޑ daviyani | | `t` | ޓ taviyani |
| `l` | ޅ lhaviyani | | `n` | ޏ gnaviyani | | `c` | ޝ sheenu |
| `h` | ޙ | `g` | ޣ | `z` | ޡ | `x` | ޘ |
| `b` | ޞ | `q` | ޤ | `w` | ޢ | `r` | ޜ |
| `y` | ޠ | `j` | ޛ | `k` | ޚ | `m` | ޟ |

## Option layer — Arabic

Holding ++option++ types a full Arabic keyboard, re-encoded from Apple's Dhivehi-QWERTY Option layer.
Letters are placed phonetically, and the **harakat sit on the same keys as the Thaana fili**, so vowel-mark
muscle memory carries straight over:

| key | mark | | key | mark | | key | mark |
|--|--|--|--|--|--|--|--|
| `a` | َ fatha | | `u` | ُ damma | | `i` | ِ kasra |
| `q` | ْ sukun | | `p` | ّ shadda | | | |

Letters:

| key | | key | | key | | key | |
|--|--|--|--|--|--|--|--|
| `w` | ا | `e` | ء | `o` | أ | `r` | ر |
| `t` | ت | `y` | ي | `s` | س | `d` | د |
| `f` | ف | `g` | غ | `h` | ه | `j` | ج |
| `k` | ك | `l` | ل | `z` | ظ | `x` | ط |
| `c` | ث | `v` | و | `b` | ب | `n` | ن |
| `m` | م | | | | | | |

**++shift+option++** adds the shifted letters (e.g. `s` ش, `h` ح, `k` خ, `j` ض, `n` ص), the **tanween**
(`a` ً, `u` ٌ, `i` ٍ), **hamza forms** (`o` آ, `e` ئ, `v` ؤ), and the **religious ligatures** — `﷽` on
++shift+option+p++, `ﷺ` on ++shift+option+minus++, `ﷻ` on ++shift+option+equal++.

## Nuances

- **`ޱ` NAA is on `x`** (base layer) — the one Thaana letter SIL omits, given a real key rather than a
  long-press. Apple's Dhivehi-QWERTY does the same.
- **`ﷲ` (Allah ligature) on ++shift+f++** — a one-shot insert of the Arabic presentation form (NFC-stable;
  common muscle-memory placement). (`﷽` bismillah now lives on the Arabic layer at ++shift+option+p++; the
  backtick is a plain `` ` ``.)
- **Optional long-vowel double-tap.** Tap a short-vowel key twice to get its long form (`a a → ާ`,
  `i i → ީ`, …) instead of reaching for Shift. One toggle for all five; **off by default**. It matches the
  fili *actually before the cursor* (the engine reads the document — see [the keys path](../architecture.md#the-keys-path)), so it stays correct even when you click back into a word.
- **RTL-correct punctuation** on the base layer: `,` → ، , `;` → ؛ , `/`(shift) → ؟ .
- **Bracket-key flip** (inherited, on): the right-hand bracket key emits the logical *open* codepoint,
  matching RTL typing instinct — the stored character stays logically correct.
- The **rufiyaa sign** `⃂` is on ++shift+4++ (the key emits `$`; the inherited `rufiyaa` transform folds
  it to `⃂` — toggle it off for a literal `$`). Long-pressing ++shift+4++ offers the literal `$` / `€`.
