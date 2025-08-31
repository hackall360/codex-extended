import test from "node:test";
import assert from "node:assert";

import { getUpdatedPath } from "../bin/codex.js";

test("removes duplicate directories when updating PATH", () => {
  const pathSep = process.platform === "win32" ? ";" : ":";
  const original = process.env.PATH;
  try {
    process.env.PATH = ["b", "c"].join(pathSep);
    const result = getUpdatedPath(["a", "b"]);
    assert.strictEqual(result, ["a", "b", "c"].join(pathSep));
  } finally {
    process.env.PATH = original;
  }
});
