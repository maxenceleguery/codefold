# @codefold/node

Node.js bindings for [codefold](https://github.com/maxenceleguery/codefold) —
structural code reader for LLM agents.

```js
import { read } from "@codefold/node";

const r = read("src/auth.py", "signatures");
console.log(r.content);
console.log(`~${r.tokensEst} tokens, ${r.symbols.length} symbols, ${r.language}`);

// With focus
const r2 = read("src/auth.py", "signatures", ["login", "verifyToken"]);
```

## Build from source

```sh
npm install
npm run build       # produces index.{js,d.ts} + native .node binary
npm test
```

Requires a Rust toolchain.

## License

MIT
