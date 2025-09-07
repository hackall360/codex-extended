## MUL

MUL is a small, typed language that Codex uses as an intermediate representation. Programs are expressed with modules, functions, structs, and explicit types, letting Codex translate code across ecosystems and perform deeper analysis.

For the full grammar and AST description see the [MUL specification](./mul-spec.md).

### Supported languages

Codex currently ships adapters that convert between MUL and:

- Python
- JavaScript
- Rust
- Go

Use `codex mul --from <language> --to <language>` to translate between these targets.

### Tooling adapters

Language support and analysis capabilities are implemented through adapters in the `codex-rs/mul` crate. Adapters implement the `MulAdapter` trait to convert source code to and from the `MulProgram` representation. Additional tooling layers build on top of this, including:

- **Parser** – turns source text into a typed AST.
- **Code generator** – emits source from a `MulProgram`.
- **Analyzer** – walks the AST/IR to compute metrics or surface diagnostics.
- **Refactor engine** – applies structural edits and pretty‑prints the result.

These components enable consistent cross‑language transformations and can be extended with new adapters for additional languages or tooling backends.
