# Dart Client

Minimal Dart wrapper for the Codex HTTP API.

```dart
import 'package:codex_client/codex_client.dart';

void main() async {
  final client = CodexClient('http://localhost:8080');
  final out = await client.mul('module main {}', 'mul', 'rust');
  print(out);
}
```

`mul` translates source text using named adapters.
