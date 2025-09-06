# MUL Examples

This directory demonstrates translating a simple MUL program into multiple languages with the `codex` CLI.

The shared MUL source lives in `program.mul`:

```mul
module main {
  fn add(a: Int, b: Int) -> Int {
    return a + b;
  }
}
```

Use the `mul` subcommand and the `--from`/`--to` flags to convert between languages:

```bash
codex mul --from mul --to <language> < program.mul > <language>/main.<ext>
```

Examples:

- `codex mul --from mul --to python < program.mul > python/main.py`
- `codex mul --from mul --to javascript < program.mul > javascript/main.js`
- `codex mul --from mul --to rust < program.mul > rust/main.rs`

Each language directory also lists common build, test, lint, and run commands.
