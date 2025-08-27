import 'package:flutter/material.dart';

import '../models/protocol.dart';
import '../services/websocket_service.dart';

class ChatView extends StatefulWidget {
  const ChatView({super.key, required this.service});

  final WebSocketService service;

  @override
  State<ChatView> createState() => _ChatViewState();
}

class _ChatViewState extends State<ChatView> {
  final events = <String>[];
  final _controller = TextEditingController();
  String session = '';

  @override
  void initState() {
    super.initState();
    widget.service.messages.listen((msg) {
      if (msg is AgentEvent) {
        setState(() => events.add('${msg.session}: ${msg.data}'));
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Expanded(
          child: ListView(
            children: events.map((e) => Text(e)).toList(),
          ),
        ),
        TextField(
          controller: _controller,
          decoration: const InputDecoration(labelText: 'Message'),
          onSubmitted: (value) {
            widget.service.sendSubmission(session, value);
            _controller.clear();
          },
        ),
      ],
    );
  }
}
