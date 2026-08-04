// Task 3.2 — smoke 1: the generated app shell mounts without exception.
//
// Entry class is `OmniApp` (emitted by the codegen app-shell in
// main.dart), not the demo component name — the harness aligns with the
// generated class. The shell is a ConsumerWidget and requires a
// ProviderScope above it (themeModeProvider and every store provider
// resolve from that container).
//
// Surface: the default widget-test surface (800x600) overflows on mount —
// the generated app has NO scroll container on the tab content (known
// finding F2, see README: Overview alone is ~4.9k logical px tall). The
// test therefore sizes the surface to the app's natural layout so the
// mount assertion stays about WIRING (ProviderScope, stores, tab
// builders) rather than the tracked layout gap.
import 'package:flutter/material.dart' show Size;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:flutter_test_app/main.dart';

void main() {
  testWidgets('app mounts without exception', (tester) async {
    tester.view.physicalSize = const Size(1440, 6000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(const ProviderScope(child: OmniApp()));
    await tester.pump();

    expect(tester.takeException(), isNull);
    expect(find.byType(OmniApp), findsOneWidget);
  });
}
