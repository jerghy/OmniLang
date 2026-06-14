import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:omni_flutter_app/counter.dart';

void main() {
  testWidgets('Counter renders initial state', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(body: Center(child: Counter())),
      ),
    );

    await tester.pumpAndSettle();

    expect(find.text('Count: 0'), findsOneWidget);
    expect(find.widgetWithText(ElevatedButton, '+'), findsOneWidget);
  });

  testWidgets('Counter increments', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(body: Center(child: Counter())),
      ),
    );

    await tester.pumpAndSettle();

    await tester.tap(find.widgetWithText(ElevatedButton, '+'));
    await tester.pumpAndSettle();
    expect(find.text('Count: 1'), findsOneWidget);

    await tester.tap(find.widgetWithText(ElevatedButton, '+'));
    await tester.pumpAndSettle();
    expect(find.text('Count: 2'), findsOneWidget);

    await tester.tap(find.widgetWithText(ElevatedButton, '+'));
    await tester.pumpAndSettle();
    expect(find.text('Count: 3'), findsOneWidget);
  });
}
