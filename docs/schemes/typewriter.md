---
title: "Typewriter"
---

`typewriter` · **keys** · the standard Maldivian typewriter arrangement — **filis on the left hand,
consonants on the right**, as on the mechanical machines.

The base and shift layers match **SIL Global's "Divehi Typewriter Basic"** (Keyman `basic_kbddiv2`, MIT)
byte-for-byte. ThariKey deviates only in the rufiyaa sign on Shift+4 (Keyman has `$`) and the RTL
bracket-flip convention. Inherits `thaana-common`.

## Base layer

Vowels & marks (left/top):

| key | glyph | name | | key | glyph | name |
|--|--|--|--|--|--|--|
| `q` | ޫ | ooboofili | | `f` | ަ | abafili |
| `w` | ޮ | obofili | | `g` | ެ | ebefili |
| `e` | ާ | aabaafili | | `a` | ި | ibifili |
| `r` | ީ | eebeefili | | `s` | ު | ubufili |
| `t` | ޭ | eybeyfili | | `d` | ް | sukun |
| `/` | ޯ | oaboafili | | `j` | އ | alifu |

Consonants (right):

| key | | key | | key | | key | |
|--|--|--|--|--|--|--|--|
| `y` | ގ | `u` | ރ | `i` | މ | `o` | ތ |
| `p` | ހ | `h` | ވ | `k` | ނ | `l` | ކ |
| `;` | ފ | `z` | ޒ | `x` | ޑ | `c` | ސ |
| `v` | ޔ | `b` | ޅ | `n` | ދ | `m` | ބ |
| `,` | ށ | `.` | ޓ | `[` | ލ | | |

## Shift layer

Thiki (Arabic-loan) consonants and symbols — e.g. `y` → ޤ , `p` → ޙ , `k` → ޘ , `l` → ޚ , `m` → ޝ ,
`n` → ޛ , `c` → ޏ , `v` → ޗ , `b` → ޟ , `.` → ޞ , `j` → ޢ , `h` → ޥ ; plus `f` → ، , `'` → ؛ , `/` → ؟ ,
and the rufiyaa sign `⃂` on ++shift+4++ (via the inherited `rufiyaa` transform — long-press for `$` / `€`).

## Nuances

- **No double-tap nicety** — long vowels already have their own base keys, so there is nothing to fold.
- **`'` emits the Allah ligature** ﷲ (U+FDF2), kept from the source layout.
- **`ޱ` NAA has no home** here; the Option layer is reserved for it and directional marks.
- Bracket keys live on `]` / `\`; the inherited bracket-flip swaps their output for RTL.
