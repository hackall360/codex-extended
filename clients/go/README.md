# Go Client

Minimal helper for calling the Codex HTTP API from Go.

```go
package main

import (
    "context"
    "fmt"
    codex "github.com/openai/codex/clients/go"
)

func main() {
    c := codex.NewClient("http://localhost:8080")
    out, _ := c.Mul(context.Background(), "module main {}", "mul", "rust")
    fmt.Println(out)
}
```

The `Mul` method accepts source text along with `from` and `to` adapter names and
returns the translated source.
