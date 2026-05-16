# codefold (Node.js)

[![npm](https://img.shields.io/npm/v/codefold)](https://www.npmjs.com/package/codefold)

Node.js bindings for [codefold](https://github.com/maxenceleguery/codefold) —
structural code reader for LLM agents.

## Install

```sh
npm install codefold
```

Prebuilt binaries are shipped for:

- Linux x86_64 (glibc) / aarch64 (glibc)
- macOS x86_64 / arm64
- Windows x64

npm picks the right sub-package automatically based on your platform.

## Use

```js
import { read } from "codefold";

const r = read("src/auth.py", "signatures");
console.log(r.content);
console.log(`~${r.tokensEst} tokens, ${r.symbols.length} symbols, ${r.language}`);

// With focus: keep `login` and `verifyToken` at full body, the rest as signatures.
const r2 = read("src/auth.py", "signatures", ["login", "verifyToken"]);
```

## API

```ts
function read(
  path: string,
  level?: "full" | "signatures" | "public" | "bodies", // default "signatures"
  focus?: string[],
): FoldResult;

interface FoldResult {
  content: string;
  symbols: Symbol[];
  hiddenRanges: { start: number; end: number }[];
  language: string;
  tokensEst: number;
}

interface Symbol {
  name: string;
  kind: "function" | "method" | "class" | "import";
  byteStart: number;
  byteEnd: number;
  lineStart: number;
  lineEnd: number;
}
```

## Build from source

Requires a Rust toolchain.

```sh
cd bindings/node
npm install
npm run build       # builds the native binary + JS bindings
npm test
```

## License

MIT
