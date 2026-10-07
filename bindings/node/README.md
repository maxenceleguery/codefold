# @maxenceleguery/codefold (Node.js)

[![npm](https://img.shields.io/npm/v/@maxenceleguery/codefold)](https://www.npmjs.com/package/@maxenceleguery/codefold)

Node.js bindings for [codefold](https://github.com/maxenceleguery/codefold),
a structural code reader for LLM agents. Languages: Python, TypeScript
(incl. TSX/JSX), Rust, Go, Markdown.

The unscoped name `codefold` was blocked by npm as too similar to an existing
`code-fold` package, so this lives under the maintainer's user scope, as do the
per-platform native sub-packages (`@maxenceleguery/codefold-linux-x64-gnu`, etc.).

## Install

```sh
npm install @maxenceleguery/codefold
```

Prebuilt binaries are shipped for:

- Linux x86_64 (glibc) / aarch64 (glibc)
- macOS x86_64 / arm64
- Windows x64

npm picks the right sub-package automatically based on your platform.

## Use

```js
import { read, readSource } from "@maxenceleguery/codefold";

const r = read("src/auth.py", "signatures");
console.log(r.content);
console.log(`~${r.tokensEst} tokens, ${r.symbols.length} symbols, ${r.language}`);

// With focus: keep `login` and `verifyToken` at full body, the rest as signatures.
const r2 = read("src/auth.py", "signatures", ["login", "verifyToken"]);

// Fold code you already have in memory (an editor buffer, a diff, stdin).
const r3 = readSource(source, "typescript", "public");
```

## API

```ts
function read(
  path: string,
  level?: "full" | "signatures" | "public" | "bodies", // default "signatures"
  focus?: string[],
): FoldResult;

function readSource(
  source: string,
  language: string, // "python" | "py" | "typescript" | "ts" | "tsx" | "jsx" | "rust" | "rs" | "go" | "markdown" | "md"
  level?: "full" | "signatures" | "public" | "bodies",
  focus?: string[],
): FoldResult;

interface FoldResult {
  content: string;
  symbols: CodeSymbol[];
  hiddenRanges: { start: number; end: number }[];
  language: string;
  tokensEst: number;
}

interface CodeSymbol {
  name: string;
  kind: "function" | "method" | "class" | "import" | "section";
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
