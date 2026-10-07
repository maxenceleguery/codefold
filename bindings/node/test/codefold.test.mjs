import { test } from "node:test";
import assert from "node:assert";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { read, readSource } from "../index.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.resolve(
  __dirname,
  "../../../crates/codefold-core/tests/fixtures",
);
const fix = (rel) => path.join(FIXTURES, rel);

test("full level returns verbatim content", () => {
  const r = read(fix("python/auth.py"), "full");
  assert.equal(r.language, "python");
  assert.ok(r.tokensEst > 0);
  assert.ok(r.content.includes("user = next("));
});

test("signatures level hides bodies", () => {
  const r = read(fix("python/auth.py"), "signatures");
  assert.ok(r.content.includes("def login"));
  assert.ok(!r.content.includes("user = next("));
  assert.ok(r.symbols.length > 0);
});

test("public level filters underscore-prefixed", () => {
  const r = read(fix("python/auth.py"), "public");
  assert.ok(r.content.includes("def login"));
  assert.ok(!r.content.includes("def _hash_password"));
});

test("focus parameter keeps named function body", () => {
  const r = read(fix("python/auth.py"), "signatures", ["login"]);
  assert.ok(r.content.includes("user = next("));
  assert.ok(!r.content.includes("secrets.compare_digest"));
});

test("typescript", () => {
  const r = read(fix("typescript/auth.ts"), "signatures");
  assert.equal(r.language, "typescript");
  assert.ok(r.content.includes("class TokenStore"));
});

test("rust", () => {
  const r = read(fix("rust/auth.rs"), "public");
  assert.equal(r.language, "rust");
  assert.ok(r.content.includes("pub fn login("));
  assert.ok(!r.content.includes("fn hash_password("));
});

test("go", () => {
  const r = read(fix("go/auth.go"), "public");
  assert.equal(r.language, "go");
  assert.ok(r.content.includes("func Login("));
  assert.ok(!r.content.includes("func hashPassword("));
});

test("default level is signatures", () => {
  const r = read(fix("python/auth.py"));
  assert.ok(r.content.includes("def login"));
  assert.ok(!r.content.includes("user = next("));
});

test("symbol fields", () => {
  const r = read(fix("python/auth.py"), "signatures");
  const login = r.symbols.find((s) => s.name === "login");
  assert.equal(login.kind, "function");
  assert.ok(login.lineStart > 0);

  const user = r.symbols.find((s) => s.name === "User");
  assert.equal(user.kind, "class");
});

test("unsupported extension throws", () => {
  assert.throws(
    () => read("/tmp/codefold-node-test.xyz", "signatures"),
    /unsupported language|no such file/i,
  );
});

test("invalid level throws", () => {
  assert.throws(
    () => read(fix("python/auth.py"), "quantum"),
    /unknown level/,
  );
});

test("readSource folds an in-memory string", () => {
  const r = readSource("def f(x):\n    return x + 1\n", "python");
  assert.equal(r.language, "python");
  assert.ok(r.content.includes("def f(x):"));
  assert.ok(!r.content.includes("return x + 1"));
});

test("readSource honors level, focus and extension aliases", () => {
  const src = "def a():\n    return 1\n\ndef b():\n    return 2\n";
  const r = readSource(src, "py", "signatures", ["b"]);
  assert.ok(r.content.includes("return 2"));
  assert.ok(!r.content.includes("return 1"));
});

test("readSource unknown language throws", () => {
  assert.throws(() => readSource("x", "cobol"), /cobol/);
});

test("type defs name symbols with an interface, not the JS primitive", async () => {
  const { readFile } = await import("node:fs/promises");
  const dts = await readFile(path.resolve(__dirname, "../index.d.ts"), "utf8");
  assert.ok(!/Array<symbol>/.test(dts), "symbols typed as primitive symbol");
});
