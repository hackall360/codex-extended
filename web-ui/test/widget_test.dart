import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/material.dart';

import 'package:web_ui/main.dart';
import 'package:web_ui/services/websocket_service.dart';

void main() {
  testWidgets('App builds with tabs', (WidgetTester tester) async {
    await tester.pumpWidget(MyApp(service: WebSocketService()));
    expect(find.byType(TabBar), findsOneWidget);
    expect(find.text('Sessions'), findsOneWidget);
    expect(find.text('Chat'), findsOneWidget);
  });
}
