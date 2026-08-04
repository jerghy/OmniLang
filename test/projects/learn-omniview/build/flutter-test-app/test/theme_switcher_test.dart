// Task 3.6 — theme card tap → ThemeData switches (4 themes reachable).
//
// Before this task the theme cards were dead on Flutter:
//  1. `@onPress` on a `div` (→ Container) had no whitelisted callback —
//     the handler was dropped with a warning, so selectTheme was never
//     called (flutter HIGH);
//  2. the app shell wired only `themeMode` (light/dark) — sepia and
//     highContrast were unreachable dead builders.
// Now tap-family events on generic elements wrap the element in a
// GestureDetector, and >2 declared themes wire through `themeDataProvider`
// (MaterialApp(theme: ref.watch(themeDataProvider))). Tapping the Sepia
// card must apply the sepia ThemeData to the running MaterialApp.
import 'package:flutter/material.dart' show Size, Theme;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:flutter_test_app/design.dart';
import 'package:flutter_test_app/main.dart';

void main() {
  testWidgets('tapping the Sepia theme card applies the sepia ThemeData', (
    tester,
  ) async {
    // The Design tab stacks ThemeSwitcher + ThemeSourceDemo + token panels +
    // CSS demos — the natural column height is ~17k px (measured overflow on
    // a 6000px surface); size the surface to the app's natural layout so the
    // mount/tap assertions stay about WIRING, not the tracked layout gap.
    tester.view.physicalSize = const Size(1440, 24000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(const ProviderScope(child: OmniApp()));
    await tester.pump();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(OmniApp)),
    );

    // Initial theme: light (the store default).
    expect(
      container.read(themeDataProvider),
      buildLearnDesignLightTheme(),
      reason: 'light must be the initial theme',
    );

    // Open the Design tab — ThemeSwitcher lives there (tab content builds
    // only when the tab is active).
    await tester.tap(find.text('💅 Design'));
    await tester.pump();
    expect(find.text('Sepia'), findsOneWidget);

    // The Sepia card is tappable (GestureDetector wrap, task 3.6) and the
    // tap switches the applied ThemeData — all 4 declared themes reachable.
    await tester.tap(find.text('Sepia'));
    await tester.pump();
    expect(
      container.read(themeDataProvider),
      buildLearnDesignSepiaTheme(),
      reason: 'sepia must be selectable — no longer collapsed to light',
    );

    // The shell actually wired the selected theme into MaterialApp.
    // MaterialApp animates theme changes (AnimatedTheme, ~200ms) — settle
    // the animation before reading Theme.of.
    await tester.pump(const Duration(milliseconds: 400));
    expect(
      Theme.of(tester.element(find.text('Sepia'))).colorScheme.primary,
      LearnDesignTokens.primarySepia,
      reason: 'MaterialApp must present the sepia color scheme',
    );

    // The Dark card switches back to the dark builder too.
    await tester.tap(find.text('Dark'));
    await tester.pump();
    expect(
      container.read(themeDataProvider),
      buildLearnDesignDarkTheme(),
      reason: 'dark must be selectable after sepia',
    );

    expect(tester.takeException(), isNull);
  });
}
