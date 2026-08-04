// Task 3.2 — smoke 3: @for list rendering.
//
// Generated shape: `for (final item in items) KeyedSubtree(key:
// ValueKey(item.id), ...)` (ForDirectiveKeyDemo). Seeded items must
// render, and an add must re-render the list with the new item.
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
}
