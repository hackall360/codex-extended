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
            service ?? (WebSocketService()..connect('ws://localhost:8080/ws')),
        themeMode = ValueNotifier(ThemeMode.system),
        seedColor = ValueNotifier(Colors.blue);

  final WebSocketService service;
  final ValueNotifier<ThemeMode> themeMode;
  final ValueNotifier<Color> seedColor;

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<ThemeMode>(
      valueListenable: themeMode,
      builder: (context, mode, _) {
        return ValueListenableBuilder<Color>(
          valueListenable: seedColor,
          builder: (context, color, __) {
            return MaterialApp(
              title: 'Codex UI',
              theme: ThemeData(
                colorSchemeSeed: color,
                useMaterial3: true,
              ),
              darkTheme: ThemeData(
                colorSchemeSeed: color,
                brightness: Brightness.dark,
                useMaterial3: true,
              ),
              themeMode: mode,
              home: HomePage(
                service: service,
                themeMode: themeMode,
                seedColor: seedColor,
              ),
            );
          },
        );
      },
    );
  }
}

class HomePage extends StatefulWidget {
  const HomePage(
      {super.key,
      required this.service,
      required this.themeMode,
      required this.seedColor});

  final WebSocketService service;
  final ValueNotifier<ThemeMode> themeMode;
  final ValueNotifier<Color> seedColor;

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  int index = 0;

  late final pages = <Widget>[
    SessionListView(service: widget.service),
    ChatView(service: widget.service),
    const WorkspacePickerView(),
    SettingsPanel(
      service: widget.service,
      themeMode: widget.themeMode,
      seedColor: widget.seedColor,
    ),
  ];

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxWidth >= 800) {
          return Scaffold(
            body: Row(
              children: [
                NavigationRail(
                  selectedIndex: index,
                  onDestinationSelected: (i) => setState(() => index = i),
                  labelType: NavigationRailLabelType.all,
                  destinations: const [
                    NavigationRailDestination(
                        icon: Icon(Icons.list), label: Text('Sessions')),
                    NavigationRailDestination(
                        icon: Icon(Icons.chat), label: Text('Chat')),
                    NavigationRailDestination(
                        icon: Icon(Icons.folder_open),
                        label: Text('Workspace')),
                    NavigationRailDestination(
                        icon: Icon(Icons.settings), label: Text('Settings')),
                  ],
                ),
                Expanded(child: pages[index]),
              ],
            ),
          );
        } else {
          return Scaffold(
            appBar: AppBar(title: const Text('Codex UI')),
            body: pages[index],
            bottomNavigationBar: BottomNavigationBar(
              currentIndex: index,
              onTap: (i) => setState(() => index = i),
              items: const [
                BottomNavigationBarItem(
                    icon: Icon(Icons.list), label: 'Sessions'),
                BottomNavigationBarItem(
                    icon: Icon(Icons.chat), label: 'Chat'),
                BottomNavigationBarItem(
                    icon: Icon(Icons.folder_open), label: 'Workspace'),
                BottomNavigationBarItem(
                    icon: Icon(Icons.settings), label: 'Settings'),
              ],
            ),
          );
        }
      },
    );
  }
}
