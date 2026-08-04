// Task 3.2 — smoke 4: store write → @for re-render (TodoStore.add()).
//
// DEFERRED (task 3.3): the typed-input path — enterText into AddTodo's
// TextField → bind:value write-back → handleAdd → store.add() — is NOT
// exercised yet: bind:value write-back is currently broken on the Flutter
// target (generated AddTodo's TextField has no controller/onChanged
// wiring). Those assertions land with 3.3 (the marker is this comment;
// 3.3 un-comments the enterText-based variant).
//
// Here the store is driven directly through the ProviderScope container;
// the reactive @for re-render (visibleItems + totalCount badge) is the
// assertion — the same store.add() code path the 3.1 fix repaired.
//
// Scaffold wrap: generated AddTodo (inside TodoList) contains a TextField
// and the generated tree has no Material/Scaffold ancestor — a bare
// TextField then explodes to ~100k px tall in the test environment
// (known finding F1, see README; Material wrapper → 48px). The Scaffold
// here provides that Material shell so the store behavior is what gets
// asserted; F1 is tracked separately (3.3 territory).
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:flutter_test_app/TodoList.dart';
import 'package:flutter_test_app/TodoStore_store.dart';

void main() {
  testWidgets('store.add() re-renders the @for list via provider', (
    tester,
  ) async {
    await tester.pumpWidget(
      const ProviderScope(
        child: MaterialApp(home: Scaffold(body: TodoList())),
      ),
    );

    // Empty state first.
    expect(find.text('还没有待办事项，在上方添加吧'), findsOneWidget);

    final container = ProviderScope.containerOf(
      tester.element(find.byType(TodoList)),
    );
    container.read(todoStoreProvider.notifier).add('widget test todo');
    await tester.pump();

    expect(find.text('widget test todo'), findsOneWidget);
    expect(find.text('1'), findsOneWidget); // totalCount badge

    expect(tester.takeException(), isNull);
  });
}
