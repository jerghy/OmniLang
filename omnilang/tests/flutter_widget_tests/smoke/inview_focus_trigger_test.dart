// Task 3.4 — smoke 5: @in-view / @focus triggers actually play their motions.
//
// Pre-fix generated shape (flutter HIGH): the @in-view element emitted a
// BARE AnimatedBuilder whose controller never started (OmniInViewWidget was
// imported but never instantiated) and @focus had ZERO listeners — both
// animations silently never played.
//
// 1. In-view: MotionTriggersDemo's slideInView element sits below the
//    viewport (1200px spacer above the demo inside a scroll view);
//    dragging the scroll view brings it into view → OmniInViewWidget's
//    scroll listener fires onInView → controller.forward() → the Opacity
//    wrapper (gated on the per-element active flag) appears and completes
//    at 1.0 (no fill override keeps the flag set after completion).
// 2. Focus: focusing the demo's TextField fires the Focus wrapper's
//    onFocusChange(true) → slideInView plays (mirrors React's focusin).
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:flutter_test_app/MotionTriggersDemo.dart';

void main() {
  testWidgets('scrolling the @in-view element into the viewport plays the animation', (
    tester,
  ) async {
    await tester.pumpWidget(
      const ProviderScope(
        child: MaterialApp(
          home: SingleChildScrollView(
            child: Column(
              children: [
                SizedBox(height: 1200),
                MotionTriggersDemo(),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    // Below the fold: the motion wrapper is not active yet (no Opacity
    // ancestor on the slide-in element).
    final slideIn = find.text('Slide in (entering viewport)');
    final opacityOf = (Finder f) =>
        find.ancestor(of: f, matching: find.byType(Opacity));
    expect(opacityOf(slideIn), findsNothing);

    // Scroll the element into the viewport → onInView → controller.forward().
    await tester.drag(
      find.byType(SingleChildScrollView),
      const Offset(0, -1200),
    );
    await tester.pumpAndSettle();

    // The animated Opacity wrapper appeared and the motion completed at 1.0.
    final opacity = tester.widget<Opacity>(opacityOf(slideIn));
    expect(opacity.opacity, 1.0);

    expect(tester.takeException(), isNull);
  });

  testWidgets('focusing the @focus input plays the motion', (tester) async {
    await tester.pumpWidget(
      const ProviderScope(child: MaterialApp(home: MotionTriggersDemo())),
    );
    await tester.pump();

    final field = find.byType(TextField);
    final opacityOf = (Finder f) =>
        find.ancestor(of: f, matching: find.byType(Opacity));
    expect(opacityOf(field), findsNothing);

    // Focus the TextField → the Focus wrapper's onFocusChange(true) fires
    // (the wrapper's region reports focus when a descendant node focuses).
    await tester.showKeyboard(field);
    await tester.pump();
    await tester.pumpAndSettle();

    final opacity = tester.widget<Opacity>(opacityOf(field));
    expect(opacity.opacity, 1.0);

    expect(tester.takeException(), isNull);
  });
}
