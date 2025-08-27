import 'dart:async';
import 'dart:convert';
import 'dart:html' as html;

import 'package:web_socket_channel/web_socket_channel.dart';

import '../models/protocol.dart';

class WebSocketService {
  WebSocketChannel? _channel;
  final _messages = StreamController<ProtocolMessage>.broadcast();
  final _history = <String, List<String>>{};

  Stream<ProtocolMessage> get messages => _messages.stream;

  void connect(String url) {
    _channel = WebSocketChannel.connect(Uri.parse(url));
    _channel!.stream.listen((data) {
      final json = jsonDecode(data as String) as Map<String, dynamic>;
      final msg = ProtocolMessage.fromJson(json);
      if (msg is AgentEvent) {
        _history.putIfAbsent(msg.session, () => []).add(msg.data);
      } else if (msg is SessionExported) {
        html.window.localStorage['session_${msg.id}'] = jsonEncode(msg.history);
      }
      _messages.add(msg);
    });
  }

  void disconnect() {
    _channel?.sink.close();
  }

  void openSession(String id) {
    _send({'type': 'open_session', 'id': id});
  }

  void closeSession(String id) {
    _send({'type': 'close_session', 'id': id});
  }

  void sendSubmission(String session, String text) {
    _send({'type': 'submission', 'session': session, 'text': text});
  }

  void requestExport(String id) {
    _send({'type': 'export_session', 'id': id});
  }

  void importFromLocal(String id) {
    final data = html.window.localStorage['session_$id'];
    if (data != null) {
      final history = jsonDecode(data) as List<dynamic>;
      _send({'type': 'import_session', 'id': id, 'history': history});
    }
  }

  void _send(Map<String, dynamic> data) {
    _channel?.sink.add(jsonEncode(data));
  }
}
