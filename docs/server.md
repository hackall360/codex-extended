# Server and CLI usage

## Configuration defaults

Codex server reads `~/.codex/config.toml`. If the file is missing, these defaults are used:

| Key               | Default value           |
| ----------------- | ----------------------- |
| `backend_url`     | `http://localhost:8000` |
| `chat_model`      | `gpt-4o`                |
| `embedding_model` | `nomic-embed-text`      |
| `store`           | `memory`                |
| `data_path`       | `./data`                |
| `port`            | `0` (choose random)     |
| `resp_port`       | _unset_                 |

An example config lives at [`examples/config.toml`](../examples/config.toml).

## Model tier mapping

When calling the chat endpoint, the `tier` parameter selects a model derived from
`chat_model`:

| Tier     | Resolved model         |
| -------- | ---------------------- |
| `low`    | `${chat_model}-low`    |
| `medium` | `${chat_model}-medium` |
| `high`   | `${chat_model}`        |

## API and CLI

The server exposes a REST API:

- `POST /v1/embeddings` – body matches [`EmbeddingsRequest`](../examples/embed.json).
- `POST /v1/vector/upsert` – body matches [`UpsertRequest`](../examples/ingest.json).
- `POST /v1/rag/answer` – body matches [`RagRequest`](../examples/pipeline.json).
- `POST /v1/chat`
- `POST /v1/vector/query`
- `POST /v1/admin/compact`

The Node CLI in [`cli/src/index.js`](../cli/src/index.js) wraps these endpoints:

| Command                | Endpoint              |
| ---------------------- | --------------------- |
| `codex models list`    | `GET /models`         |
| `codex embed <text>`   | `POST /embed`         |
| `codex ingest <file>`  | `POST /ingest`        |
| `codex ask <question>` | `POST /ask`           |
| `codex admin compact`  | `POST /admin/compact` |

## Pulling Ollama models

Ensure required models are available before starting the server:

```bash
ollama pull gpt-4o
ollama pull gpt-4o-low
ollama pull gpt-4o-medium
ollama pull nomic-embed-text
```

## Running tests

```bash
# Rust crates
cargo test -p codex-ollama

# CLI tests
cd cli && npm test
```
