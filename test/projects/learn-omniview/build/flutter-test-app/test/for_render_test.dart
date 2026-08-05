// Task 3.2 — smoke 3: @for list rendering.
//
// Generated shape: `for (final item in items) KeyedSubtree(key:
// ValueKey(item.id), ...)` (ForDirectiveKeyDemo). Seeded items must
// render, and an add must re-render the list with the new item.
//
// Task 7.6 — smoke 3b: @for REORDER key stability (behavioral-parity
// scenario ③): "Move Last to Front" must reuse the keyed subtrees — the
// same Element instances keep their identity through the reorder (an
// unkeyed index-matched remount would produce NEW elements for the moved
// items).
//
// Surface: rows are ~300px each (banner + id/name), so the seeded list
// (~960px) and the post-add list (~1150px) overflow the default 600px
// surface — sized to fit (same rationale as F2, see README).
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:flutter_test_app/ForDirectiveKeyDemo.dart';

void main() {
  testWidgets('@for renders seeded items and reflects add', (tester) async {
    tester.view.physicalSize = const Size(800, 1600);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(const MaterialApp(home: ForDirectiveKeyDemo()));

    // 3 seeded fruits.
    expect(find.text('Apple'), findsOneWidget);
    expect(find.text('Banana'), findsOneWidget);
    expect(find.text('Cherry'), findsOneWidget);
    expect(find.text('Total:3· 下一 id:4'), findsOneWidget);

    // add → 4th item rendered, counters bumped.
    await tester.tap(find.text('Add Item'));
    await tester.pump();
    expect(find.text('Fruit-4'), findsOneWidget);
    expect(find.text('Total:4· 下一 id:5'), findsOneWidget);

    expect(tester.takeException(), isNull);
  });

  testWidgets('@for reorder keeps item identity via ValueKey', (tester) async {
    tester.view.physicalSize = const Size(800, 1600);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(const MaterialApp(home: ForDirectiveKeyDemo()));

    // Capture the Element identity of two items before the reorder.
    final cherryBefore = tester.element(find.text('Cherry'));
    final appleBefore = tester.element(find.text('Apple'));

    await tester.tap(find.text('Move Last to Front'));
    await tester.pump();

    // The reordered list renders the new order (Cherry above Apple).
    expect(
      tester.getTopLeft(find.text('Cherry')).dy,
      lessThan(tester.getTopLeft(find.text('Apple')).dy),
      reason: 'the reordered list must render the new order',
    );

    // Key-stability: with ValueKey(item.id) the KeyedSubtree elements are
    // MOVED, not recreated — the same Element instance keeps its identity
    // through the reorder. An unkeyed index-matched remount would produce
    // a NEW element for the moved items (this finds a different instance).
    expect(
      identical(tester.element(find.text('Cherry')), cherryBefore),
      isTrue,
      reason: 'the Cherry item must keep its Element identity across the '
          'reorder — ValueKey reuse; an unkeyed remount would recreate it',
    );
    expect(
      identical(tester.element(find.text('Apple')), appleBefore),
      isTrue,
      reason: 'the Apple item must keep its Element identity across the '
          'reorder — ValueKey reuse; an unkeyed remount would recreate it',
    );

    expect(tester.takeException(), isNull);
  });
}
