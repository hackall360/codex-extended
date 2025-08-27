# Web UI

The Codex Web UI is an experimental Flutter front end that connects to the CLI over WebSockets.

## Start the WebSocket CLI

Set your API key and launch the CLI in WebSocket mode:

```bash
export OPENAI_API_KEY="your-api-key-here"
codex web --addr 127.0.0.1:8080
```

This starts a server on `ws://127.0.0.1:8080/ws`. Use `--addr` to change the host or port.

## Build and serve the Flutter app

From the repository root:

```bash
cd web-ui
flutter build web
flutter run -d web-server --web-port 8081
```

`flutter build web` creates a static bundle under `build/web`. `flutter run` serves it locally so you can visit the printed URL in your browser. The app expects to reach the CLI at the WebSocket URL shown above.
