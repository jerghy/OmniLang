// Task 3.2 — smoke 2: @if conditional rendering.
//
// Generated shape: `isOpen ? Container(...) : SizedBox()` (Modal). The
// subtree must appear on open and disappear on close.
//
// Surface: the open modal column (~770px) overflows the default 600px
// test surface, which also pushed the Confirm button off-screen (its tap
// missed). Sized to fit so the assertion stays about the @if toggle.
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:flutter_test_app/Modal.dart';

void main() {
  testWidgets('@if toggles the modal subtree in and out', (tester) async {
    tester.view.physicalSize = const Size(800, 1200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(const MaterialApp(home: Modal()));

    // @if false → subtree absent.
    expect(find.text('Modal Title'), findsNothing);
    expect(find.text('Modal content goes here.'), findsNothing);

    // open → @if true → subtree present.
    await tester.tap(find.text('Open Modal'));
    await tester.pump();
    expect(find.text('Modal Title'), findsOneWidget);
    expect(find.text('Modal content goes here.'), findsOneWidget);

    // close via Confirm → @if false again.
    await tester.tap(find.text('Confirm'));
    await tester.pump();
    expect(find.text('Modal Title'), findsNothing);

    expect(tester.takeException(), isNull);
  });
}
