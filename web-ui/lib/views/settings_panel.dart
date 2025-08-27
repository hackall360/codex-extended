import 'package:flutter/material.dart';

import '../models/protocol.dart';
import '../services/websocket_service.dart';

class SettingsPanel extends StatefulWidget {
  const SettingsPanel({super.key, required this.service});

  final WebSocketService service;

  @override
  State<SettingsPanel> createState() => _SettingsPanelState();
}

class _SettingsPanelState extends State<SettingsPanel> {
  final fields = <String, TextEditingController>{};

  @override
  void initState() {
    super.initState();
    widget.service.messages.listen((msg) {
      if (msg is ConfigForm) {
        setState(() {
          fields.clear();
          msg.fields.forEach((key, value) {
            fields[key] = TextEditingController(text: '$value');
          });
        });
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(8),
      children: fields.entries
          .map(
            (e) => Padding(
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: TextField(
                controller: e.value,
                decoration: InputDecoration(labelText: e.key),
              ),
            ),
          )
          .toList(),
    );
  }
}
