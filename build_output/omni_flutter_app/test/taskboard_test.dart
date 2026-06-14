import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:omni_flutter_app/taskboard.dart';

/// Helper to set a wide viewport for complex layouts.
void _setWideViewport(WidgetTester tester) {
  tester.view.physicalSize = const Size(2400, 1600);
  tester.view.devicePixelRatio = 1.0;
}

void main() {
  testWidgets('TaskBoard renders all panels', (tester) async {
    _setWideViewport(tester);
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(body: Center(child: TaskBoard())),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Task Assignment Dashboard'), findsOneWidget);
    expect(find.text('Team Members'), findsOneWidget);
    expect(find.text('Alice Chen'), findsOneWidget);
    expect(find.text('Bob Wang'), findsOneWidget);
    expect(find.text('Charlie Li'), findsOneWidget);
    expect(find.text('Task Board'), findsOneWidget);
    expect(find.text('Statistics'), findsOneWidget);
    expect(find.text('Total tasks: 4'), findsOneWidget);
  });

  testWidgets('TaskBoard assign T1 to Alice', (tester) async {
    _setWideViewport(tester);
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(body: Center(child: TaskBoard())),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.widgetWithText(ElevatedButton, 'Assign T1 to Alice'));
    await tester.pumpAndSettle();

    expect(find.textContaining('Assignee: Alice'), findsOneWidget);
  });

  testWidgets('TaskBoard capacity limit', (tester) async {
    _setWideViewport(tester);
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(body: Center(child: TaskBoard())),
      ),
    );
    await tester.pumpAndSettle();

    final assignBtn = find.widgetWithText(ElevatedButton, 'Assign T1 to Alice');

    // 3 clicks = Alice at capacity
    for (int i = 0; i < 3; i++) {
      await tester.tap(assignBtn);
      await tester.pumpAndSettle();
    }

    // 4th click = error
    await tester.tap(assignBtn);
    await tester.pumpAndSettle();

    expect(find.text('Alice is at capacity!'), findsOneWidget);
  });

  testWidgets('TaskBoard filter changes', (tester) async {
    _setWideViewport(tester);
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(body: Center(child: TaskBoard())),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.widgetWithText(OutlinedButton, 'High Priority'));
    await tester.pumpAndSettle();
    expect(find.text('Filter: high'), findsOneWidget);

    await tester.tap(find.widgetWithText(ElevatedButton, 'All'));
    await tester.pumpAndSettle();
    expect(find.text('Filter: all'), findsOneWidget);
  });

  testWidgets('TaskBoard sync toggle', (tester) async {
    _setWideViewport(tester);
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(body: Center(child: TaskBoard())),
      ),
    );
    await tester.pumpAndSettle();

    final syncBtn = find.widgetWithText(OutlinedButton, 'Sync to Backend');
    await tester.tap(syncBtn);
    await tester.pumpAndSettle();
    expect(find.text('Saving: true'), findsOneWidget);

    // Second click: error
    await tester.tap(syncBtn);
    await tester.pumpAndSettle();
    expect(find.text('Sync failed: network error'), findsOneWidget);
  });
}
