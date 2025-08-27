import 'package:flutter/material.dart';

import '../models/protocol.dart';
import '../services/websocket_service.dart';

class SessionListView extends StatefulWidget {
  const SessionListView({super.key, required this.service});

  final WebSocketService service;

  @override
  State<SessionListView> createState() => _SessionListViewState();
}

class _SessionListViewState extends State<SessionListView> {
  final sessions = <String>[];
  final _controller = TextEditingController();

  @override
  void initState() {
    super.initState();
    widget.service.messages.listen((msg) {
      if (msg is SessionOpened) {
        setState(() => sessions.add(msg.id));
      } else if (msg is SessionClosed) {
        setState(() => sessions.remove(msg.id));
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Expanded(
          child: ListView(
            children: sessions.map((s) => ListTile(title: Text(s))).toList(),
          ),
        ),
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: _controller,
                decoration: const InputDecoration(labelText: 'Session id'),
              ),
            ),
            IconButton(
              icon: const Icon(Icons.add),
              onPressed: () {
                widget.service.openSession(_controller.text);
                _controller.clear();
              },
            ),
            IconButton(
              icon: const Icon(Icons.save),
              onPressed: () {
                widget.service.requestExport(_controller.text);
              },
            ),
            IconButton(
              icon: const Icon(Icons.upload),
              onPressed: () {
                widget.service.importFromLocal(_controller.text);
              },
            ),
          ],
        ),
      ],
    );
  }
}
