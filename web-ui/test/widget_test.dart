import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/material.dart';

import 'package:web_ui/main.dart';
import 'package:web_ui/services/websocket_service.dart';

void main() {
  testWidgets('App builds with navigation', (WidgetTester tester) async {
    tester.binding.window.physicalSizeTestValue = const Size(400, 800);
    addTearDown(() => tester.binding.window.clearPhysicalSizeTestValue());

    await tester.pumpWidget(MyApp(service: WebSocketService()));
    expect(find.byType(BottomNavigationBar), findsOneWidget);
    expect(find.text('Sessions'), findsOneWidget);
    expect(find.text('Chat'), findsOneWidget);
  });
}
