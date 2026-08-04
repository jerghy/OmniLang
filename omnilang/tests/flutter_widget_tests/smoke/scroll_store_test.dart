// Task 3.7 — smoke: [scroll] ListView + store reads (Riverpod 3.0 scope).
//
// The [scroll] @for emits ListView.builder; its itemBuilder executes at
// LAYOUT time — `ref.watch` inside it throws in Riverpod 3.0 ("watch
// outside the build phase"). The codegen hoists the item body's store reads
// to build locals (`final __omniN = ref.watch(...)`), and the itemBuilder
// references the locals by capture.
//
// This test mounts the generated ScrollStoreDemo and asserts:
// 1. the empty state renders (its store read runs inside the Builder
//    wrapper — a build-phase closure, legal);
// 2. seeded items render with the per-item total badge (proves the hoisted
//    local is captured by the itemBuilder);
// 3. `tester.takeException()` is null — a layout-phase watch would surface
//    here as an unhandled exception.
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:flutter_test_app/ScrollStoreDemo.dart';
import 'package:flutter_test_app/ScrollStore_store.dart';

void main() {
  testWidgets('[scroll] list + store reads render without Riverpod scope errors', (
    tester,
  ) async {
    await tester.pumpWidget(
      const ProviderScope(
        child: MaterialApp(home: Scaffold(body: ScrollStoreDemo())),
      ),
    );

    // Empty state first — its store read is a build-phase watch (the
    // @empty wrapper is a Builder, which builds during build).
    expect(find.textContaining('Empty list'), findsOneWidget);
    expect(tester.takeException(), isNull);

    // Seed via the container → store action → @for re-render.
    final container = ProviderScope.containerOf(
      tester.element(find.byType(ScrollStoreDemo)),
    );
    container.read(scrollStoreProvider.notifier).addItem('alpha');
    container.read(scrollStoreProvider.notifier).addItem('beta');
    await tester.pump();

    expect(find.text('alpha'), findsOneWidget);
    expect(find.text('beta'), findsOneWidget);
    // Per-item badge read — the hoisted build local captured by the
    // itemBuilder (one badge per rendered row).
    expect(find.text('2'), findsNWidgets(2));

    // No Riverpod scope exception (a layout-phase watch would throw here).
    expect(tester.takeException(), isNull);
  });
}
