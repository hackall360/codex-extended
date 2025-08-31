import test from "node:test";
import assert from "node:assert";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";
import { mkdtempSync, symlinkSync, unlinkSync, rmdirSync } from "node:fs";
import { spawnSync } from "node:child_process";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

test("executes main when invoked via symlink", () => {
  const tmpDir = mkdtempSync(path.join(tmpdir(), "codex-cli-test-"));
  const linkPath = path.join(tmpDir, "codex");
  symlinkSync(path.join(__dirname, "../bin/codex.js"), linkPath);
  const result = spawnSync(process.execPath, [linkPath], { encoding: "utf8" });
  try {
    assert.strictEqual(result.status, 1);
  } finally {
    unlinkSync(linkPath);
    rmdirSync(tmpDir);
  }
});
