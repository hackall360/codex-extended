# MUL Specification

## Grammar

```ebnf
module     = "module" identifier "{" { declaration } "}"

; Declarations

declaration = function | struct | import
function    = "fn" identifier "(" [ parameters ] ")" [ "->" type ] block
struct      = "struct" identifier "{" { field } "}"
import      = "import" string

parameters  = parameter { "," parameter }
parameter   = identifier ":" type
field       = identifier ":" type

block       = "{" { statement } "}"
statement   = let_stmt | expr_stmt | return_stmt
let_stmt    = "let" identifier ":" type "=" expression ";"
expr_stmt   = expression ";"
return_stmt = "return" expression ";"

expression  = assignment
assignment  = identifier "=" expression
            | logic_or
logic_or    = logic_and { "||" logic_and }
logic_and   = equality { "&&" equality }
equality    = comparison { ("==" | "!=") comparison }
comparison  = term { ("<" | "<=" | ">" | ">=") term }
term        = factor { ("+" | "-") factor }
factor      = unary { ("*" | "/") unary }
unary       = ("!" | "-") unary | primary
primary     = number
            | string
            | identifier
            | "(" expression ")"
```

## Abstract Syntax Tree

MUL parses source into a typed abstract syntax tree. Key nodes include:

- `Module` – collection of declarations.
- `Function` – name, parameters, return type, and body.
- `Struct` – name and typed fields.
- `Block` – ordered statements.
- `Statement` – `Let`, `Expr`, or `Return`.
- `Expression` – assignment, binary, unary, and primary forms.
- `Type` – intrinsic types (`Int`, `Float`, `Bool`, `String`) and user-defined.

## Tooling Abstractions

- **Parser** – converts source text to AST using the grammar above.
- **Type Checker** – annotates the AST with types, resolving user-defined names.
- **IR Builder** – lowers the AST to an intermediate form for optimization.
- **Code Generator** – targets specific languages or bytecode backends.
- **Analyzer** – walks the AST/IR to compute metrics and detect patterns.
- **Refactor Engine** – applies AST transforms and pretty-prints the result.

## Translation Requirements

### Code Generation

- Preserve symbol names and module structure.
- Emit idiomatic constructs for the target language (e.g., `function` vs `fn`).
- Provide hooks for runtime or library imports.

### Analysis

- Expose a stable IR for external tooling.
- Track source spans to map diagnostics back to original files.
- Support custom passes that operate on AST or IR nodes.

### Refactoring

- Guarantee round-trip stability: formatting a program then parsing it again must yield the same AST.
- Allow structural edits such as renaming, extraction, and inlining.
- Produce minimal diffs when printing back to source.

## Round-trip Examples

### Python

```python
# Python source
def add(a: int, b: int) -> int:
    return a + b
```

```mul
module main {
  fn add(a: Int, b: Int) -> Int {
    return a + b;
  }
}
```

```python
# Back to Python
def add(a: int, b: int) -> int:
    return a + b
```

### JavaScript

```javascript
// JavaScript source
function add(a, b) {
  return a + b;
}
```

```mul
module main {
  fn add(a: Int, b: Int) -> Int {
    return a + b;
  }
}
```

```javascript
// Back to JavaScript
function add(a, b) {
  return a + b;
}
```

### Rust

```rust
// Rust source
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

```mul
module main {
  fn add(a: Int, b: Int) -> Int {
    return a + b;
  }
}
```

```rust
// Back to Rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

### Go

```go
// Go source
func add(a int, b int) int {
    return a + b
}
```

```mul
module main {
  fn add(a: Int, b: Int) -> Int {
    return a + b;
  }
}
```

```go
// Back to Go
func add(a int, b int) int {
    return a + b
}
```

### Java

```java
// Java source
int add(int a, int b) {
    return a + b;
}
```

```mul
module main {
  fn add(a: Int, b: Int) -> Int {
    return a + b;
  }
}
```

```java
// Back to Java
int add(int a, int b) {
    return a + b;
}
```
