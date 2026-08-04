// Task 3.9 — smoke 7: page switcher keeps form input across tab switches.
//
// A component-ROOT @if whose branches are all component instances (the
// page-switch shape) emits an IndexedStack — every page stays MOUNTED, so
// state (typed form input) survives the switch. The old
// `cond ? child : SizedBox()` ternary unmounted the outgoing page on every
// switch and dropped its state.
//
// Hidden IndexedStack branches remain in the widget tree (not Offstage) but
// are NOT hit-testable — taps use `.hitTestable()` so they always land on
// the visible page.
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:flutter_test_app/PageSwitcherDemo.dart';

void main() {
  testWidgets('typed form input survives a page switch (IndexedStack)', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(800, 1200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(const MaterialApp(home: PageSwitcherDemo()));

    final homeField = find.widgetWithText(TextField, 'Home draft');
    expect(
      homeField,
      findsOneWidget,
      reason: 'the Home page must render its text field',
    );

    // Type into the Home page's input, then switch to About and back.
    await tester.enterText(homeField, 'draft note');
    await tester.pump();
    expect(
      find.text('draft note'),
      findsOneWidget,
      reason: 'typed text must be visible before switching',
    );

    await tester.tap(find.text('Go to About').hitTestable());
    await tester.pump();
    expect(
      find.text('Back to Home').hitTestable(),
      findsOneWidget,
      reason: 'the About page must be the visible page after the switch',
    );

    await tester.tap(find.text('Back to Home').hitTestable());
    await tester.pump();

    // The Home page stayed MOUNTED (IndexedStack) — the typed text is still
    // in the field. The old ternary remounted an empty field and this finds
    // nothing.
    expect(
      find.text('draft note'),
      findsOneWidget,
      reason: 'typed input must survive the tab switch — the outgoing page '
          'was unmounted on switch (ternary) and its state was lost',
    );
    expect(tester.takeException(), isNull);
  });
}
