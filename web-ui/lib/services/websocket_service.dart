import 'dart:async';
import 'dart:convert';

import 'package:web_socket_channel/web_socket_channel.dart';

import '../models/protocol.dart';

class WebSocketService {
  WebSocketChannel? _channel;
  final _messages = StreamController<ProtocolMessage>.broadcast();

  Stream<ProtocolMessage> get messages => _messages.stream;

  void connect(String url) {
    _channel = WebSocketChannel.connect(Uri.parse(url));
    _channel!.stream.listen((data) {
      final json = jsonDecode(data as String) as Map<String, dynamic>;
      _messages.add(ProtocolMessage.fromJson(json));
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

  void _send(Map<String, dynamic> data) {
    _channel?.sink.add(jsonEncode(data));
  }
}
