// Task 3.2 + 3.3 — smoke 4: store write → @for re-render (TodoStore.add()).
//
// Two paths exercise the same store.add() → @for re-render:
// 1. container-driven: the ProviderScope container calls add() directly;
// 2. typed-input (task 3.3, the 3.2 DEFERRED marker now landed): enterText
//    into AddTodo's TextField → bind:value write-back (the onChanged
//    setState signal assignment this task wires) → tap 添加 → handleAdd →
//    onAdd event → store.add().
//
// F1 (3.2 review finding, fixed in 3.3): the generated root of an
// input-bearing component is wrapped in Material by the codegen, so the
// TextField renders at a sane height WITHOUT an external Scaffold — the
// old generated tree had no Material ancestor and the bare TextField
// exploded to ~100k px tall in the test environment (a Material wrapper
// restores the 48 px input). The typed-input test pumps TodoList directly
// under MaterialApp to prove the codegen-side shell.
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:flutter_test_app/TodoItem.dart';
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

  testWidgets('enterText writes back via bind:value then adds the todo', (
    tester,
  ) async {
    // F1: no external Scaffold — the codegen wraps the input-bearing
    // component root in Material, so the TextField must render at a sane
    // height (~48 px), not ~100k px.
    await tester.pumpWidget(
      const ProviderScope(child: MaterialApp(home: TodoList())),
    );

    expect(find.text('还没有待办事项，在上方添加吧'), findsOneWidget);

    final textField = find.byType(TextField);
    final size = tester.getSize(textField);
    expect(
      size.height,
      lessThan(200),
      reason:
          'F1: TextField exploded to ${size.height} px — the generated root '
          'has no Material ancestor',
    );

    // bind:value write-back: the typed text must flow into the `input`
    // signal (onChanged → setState assignment)...
    await tester.enterText(textField, 'typed todo');
    await tester.pump();

    // ...and the 添加 button must now act on it: handleAdd emits onAdd →
    // store.add() → @for re-render. (The TextField itself still displays
    // 'typed todo' — the non-ref read-back direction is not wired — so the
    // item assertion is scoped to the TodoItem subtree.)
    await tester.tap(find.text('添加'));
    await tester.pump();

    expect(find.widgetWithText(TodoItem, 'typed todo'), findsOneWidget);
    expect(find.text('1'), findsOneWidget); // totalCount badge

    expect(tester.takeException(), isNull);
  });
}
