# Dives Akuru

`dives-akuru` · **keys** · a keyboard layout for the historic **Dives Akuru** script.

Dives Akuru (*ދިވެސް އަކުރު*, "island letters") is the Brahmic-derived script the Maldives used before
Thaana — written left-to-right, with consonants carrying an inherent vowel and dependent vowel signs,
like other Brahmic scripts. It was encoded in **Unicode 13.0** at **U+11900–U+1195F** (the Supplementary
Multilingual Plane).

This scheme emits Dives Akuru codepoints directly. It is re-encoded from the key/codepoint facts of
Keyman's `dives_akuru_inscript` keyboard (© SIL Global, MIT-licensed).

!!! info "Not a Thaana → Dives transliteration"
    This is a **layout** — it maps physical keys to Dives Akuru letters, the same way the Phonetic and
    Typewriter schemes map keys to Thaana. It does not convert Thaana into Dives Akuru. (That would need
    conjunct formation and pre-base vowel reordering — a separate, harder problem.)

!!! warning "You need a Dives Akuru font"
    The engine emits **logical-order codepoints**; turning them into correctly-shaped text — conjuncts,
    reordered pre-base vowels — is the font and text shaper's job. Install a Dives Akuru font such as
    **Noto Serif Dives Akuru**; macOS ships none, so without it you'll see tofu (□).

## Layout (InScript)

InScript convention: the **base** layer carries consonants and *dependent* vowel signs; **Shift**
carries the *independent* vowels and extra letters.

A sample of the base layer (key → letter):

| key | letter | | key | letter |
|--|--|--|--|--|
| `k` | KA | | `a` | vowel sign O |
| `i` | GA | | `e` | vowel sign AA |
| `c` | MA | | `f` | vowel sign I |
| `o` | DA | | `g` | vowel sign U |
| `p` | JA | | `r` | vowel sign II |
| `u` | HA | | `s` | vowel sign E |
| `y` | BA | | `w` | vowel sign AI |
| `v` | NA | | `q` | halanta |
| `j` | RA | | `d` | virama |

Shift adds the independent vowels (`g` → U, `r` → II, `s` → E …) and aspirated/extra consonants
(`k` → KHA, `i` → GHA, `l` → THA, `o` → DHA …), plus the medial RA, anusvara, nukta, and the prefixed
nasal sign.

Digits `0`–`9` map to the Dives Akuru digits; `.` is the double danda.
