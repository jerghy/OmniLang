// Task 7.6 — smoke: motion @enter plays on mount (EasingFunctionsDemo).
//
// Behavioral-parity scenario ⑤ (mirror of the React WAAPI-running
// assertion in behavioral-parity.spec.ts B5): the @enter trigger must PLAY
// when the element enters the tree. The Flutter codegen emits one
// AnimationController per @enter element with `.forward()` in initState —
// every @enter card animates from its `from` state toward `to` on mount.
//
// Observable proxy: the first @enter card ("ease-in", easeInMove) is
// wrapped in `Transform.translate(offset: Offset(_easeInMoveTranslateX_1
// .value, 0.0))` with a 0 → 180 tween over 800ms. The evidence pair:
//   · right after mount the offset is at the `from` value (< 180);
//   · after the 800ms window it settles at exactly 180.0 (`to` value) —
//     a never-started controller would read 0.0 forever.
//
// Why this demo (and not KeyframeAnimationDemo / AnimPropertiesDemo):
// those demos also contain @enter cards with COLOR tweens, and the
// Flutter codegen emits plain `Tween<Color>` — Flutter's Tween.lerp
// asserts on Color mid-flight ("Cannot lerp between Color..."), a real
// codegen defect registered as a blocker in the task-70 report. The
// @enter-playback evidence must not be tangled with that defect, so the
// probe uses the color-free easing demo.
//
// Why fixed pumps instead of pumpAndSettle: the demo's Replay button
// rewinds/forwards the backOutMove controller on press, and other demos
// in this family use `loop` — fixed pumps keep the assertion scoped to
// the 800ms window.
//
// Surface: the demo is a tall card grid (~8k px natural height, same
// overflow rationale as theme_switcher_test) — sized to fit so the
// assertion stays about the @enter wiring, not the tracked layout gap.
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:flutter_test_app/EasingFunctionsDemo.dart';

void main() {
  testWidgets('@enter animations start on mount and complete at their `to` value', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1440, 16000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(const MaterialApp(home: EasingFunctionsDemo()));
    await tester.pump();

    final easeIn = find.text('ease-in');
    final transformOf = (Finder f) =>
        find.ancestor(of: f, matching: find.byType(Transform));
    expect(
      transformOf(easeIn),
      findsOneWidget,
      reason: 'the @enter card must render its Transform wrapper',
    );

    // t=0: the controller just started — the offset is at the `from`
    // value (0.0), NOT yet at the `to` value (180.0).
    final early = tester
        .widget<Transform>(transformOf(easeIn))
        .transform
        .getTranslation()
        .x;
    expect(
      early,
      lessThan(180.0),
      reason: 'the @enter animation must be mid-flight right after mount — '
          'a completed-at-mount read means the animation was skipped',
    );

    // After the 800ms window the controller has run to completion — the
    // offset settles at the `to` value (180.0). A controller that never
    // forwarded would still read 0.0 here.
    await tester.pump(const Duration(milliseconds: 1000));
    final settled = tester
        .widget<Transform>(transformOf(easeIn))
        .transform
        .getTranslation()
        .x;
    expect(
      settled,
      180.0,
      reason: 'the @enter animation must COMPLETE at the `to` value (180.0) '
          'within its duration — a never-played controller reads 0.0',
    );

    expect(tester.takeException(), isNull);
  });
}
