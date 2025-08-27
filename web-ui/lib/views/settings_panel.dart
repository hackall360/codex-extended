import 'package:flutter/material.dart';

import '../models/protocol.dart';
import '../services/websocket_service.dart';

class SettingsPanel extends StatefulWidget {
  const SettingsPanel(
      {super.key,
      required this.service,
      required this.themeMode,
      required this.seedColor});

  final WebSocketService service;
  final ValueNotifier<ThemeMode> themeMode;
  final ValueNotifier<Color> seedColor;

  @override
  State<SettingsPanel> createState() => _SettingsPanelState();
}

class _SettingsPanelState extends State<SettingsPanel> {
  final fields = <String, TextEditingController>{};
  static const colorOptions = <String, Color>{
    'Blue': Colors.blue,
    'Green': Colors.green,
    'Red': Colors.red,
    'Purple': Colors.purple,
  };

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
    final List<Widget> children = [
      ValueListenableBuilder<ThemeMode>(
        valueListenable: widget.themeMode,
        builder: (context, mode, _) {
          return DropdownButtonFormField<ThemeMode>(
            value: mode,
            decoration: const InputDecoration(labelText: 'Theme'),
            onChanged: (m) {
              if (m != null) widget.themeMode.value = m;
            },
            items: ThemeMode.values
                .map((m) => DropdownMenuItem(value: m, child: Text(m.name)))
                .toList(),
          );
        },
      ),
      const SizedBox(height: 8),
      ValueListenableBuilder<Color>(
        valueListenable: widget.seedColor,
        builder: (context, color, _) {
          return DropdownButtonFormField<Color>(
            value: color,
            decoration: const InputDecoration(labelText: 'Accent color'),
            onChanged: (c) {
              if (c != null) widget.seedColor.value = c;
            },
            items: colorOptions.entries
                .map((e) => DropdownMenuItem(value: e.value, child: Text(e.key)))
                .toList(),
          );
        },
      ),
      const Divider(),
      ...fields.entries
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
    ];

    return ListView(padding: const EdgeInsets.all(8), children: children);
  }
}
