# @tharikey/engine

The WebAssembly build of the **ThariKey** engine — a small, fast, data-driven **Thaana (Dhivehi)**
transliteration & keyboard engine. Lexicon-free, no runtime data, ships as one `.wasm` (~110 KB
brotli).

```sh
npm install @tharikey/engine
```

> Built with `wasm-pack --target bundler` — use it with a bundler (Vite, webpack, Rollup, Next, …).

## Usage

```js
import { transliterate, reverse, listSchemes, Session } from '@tharikey/engine';

// One-shot text transforms (text schemes, e.g. Malé Latin)
transliterate('male-latin', 'raajje');        // 'ރާއްޖެ'
reverse('male-latin', 'ދިވެހި');               // 'dhivehi'

// All bundled schemes (id, name, kind: 'keys' | 'text')
listSchemes();

// Stateful keyboard session (keys schemes — what an IME feeds key-by-key)
const s = new Session('phonetic');
for (const k of ['b', 'a', 's']) s.feed(k, 'base');
s.output();                                     // 'ބަސް'
```

The full API (scheme metadata, per-key output, the custom-scheme registry, and the context-aware
`peek`/`resolve` reach-back) mirrors the engine's other bindings.

## Docs

- Documentation: <https://tharikey.com/docs>
- Source: <https://github.com/tharikey/tharikey-engine>

MIT
