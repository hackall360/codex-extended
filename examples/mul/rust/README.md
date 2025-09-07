# Rust

Generate Rust from MUL:

```bash
codex mul --from mul --to rust < ../program.mul > main.rs
```

The generated `main.rs`:

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

Tooling commands:

- Build: `cargo build`
- Test: `cargo test`
- Lint: `cargo clippy`
- Run: `cargo run`
