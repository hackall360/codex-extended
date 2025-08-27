import 'package:flutter/material.dart';

import 'services/websocket_service.dart';
import 'views/chat_view.dart';
import 'views/session_list.dart';
import 'views/workspace_picker.dart';
import 'views/settings_panel.dart';

void main() {
  runApp(MyApp());
}

class MyApp extends StatelessWidget {
  MyApp({super.key, WebSocketService? service})
      : service =
            service ?? (WebSocketService()..connect('ws://localhost:8080/ws'));

  final WebSocketService service;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Codex UI',
      theme: ThemeData(useMaterial3: true),
      home: HomePage(service: service),
    );
  }
}

class HomePage extends StatelessWidget {
  const HomePage({super.key, required this.service});

  final WebSocketService service;

  @override
  Widget build(BuildContext context) {
    return DefaultTabController(
      length: 4,
      child: Scaffold(
        appBar: AppBar(
          title: const Text('Codex UI'),
          bottom: const TabBar(
            tabs: [
              Tab(text: 'Sessions'),
              Tab(text: 'Chat'),
              Tab(text: 'Workspace'),
              Tab(text: 'Settings'),
            ],
          ),
        ),
        body: TabBarView(
          children: [
            SessionListView(service: service),
            ChatView(service: service),
            const WorkspacePickerView(),
            SettingsPanel(service: service),
          ],
        ),
      ),
    );
  }
}
