// Task 3.5 — smoke 5: FormPage textarea + disabled mapping.
//
// Two runtime behaviors the review's flutter HIGH flagged as silent drops:
// 1. `<textarea>` rendered `Semantics(label: 'Message input',
//    child: Container())` — an invisible field. The fix maps it to
//    `TextField(maxLines: 4)`: the Message field must be findable (visible)
//    and typeable.
// 2. `disabled` was dropped with zero warnings — the "Disabled field"
//    rendered as an ENABLED TextField. The fix maps the boolean attribute to
//    `enabled: false`: the field must report `enabled == false` (the
//    TextField contract for "not editable") and stay inert on tap.
//
// The message field carries bind:value + @onChange. Task C2b (#9) merged
// the write-back INTO the user's onChanged handler, so typing now flows
// back into the signal — the trimmed-message display mirrors the typed
// text (the pre-C2b behavior dropped the write-back silently).
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:flutter_test_app/FormPage.dart';

void main() {
  // FormPage is a tall form (~950px); the default 800×600 test surface
  // overflows it (the page shell / Scaffold wrapping is the 3.9 task's
  // territory). A taller viewport keeps the test focused on the field
  // mappings instead of the layout gap.
  Future<void> pumpTallForm(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      const ProviderScope(child: MaterialApp(home: FormPage())),
    );
  }

  testWidgets('FormPage textarea maps to a visible multiline TextField', (
    tester,
  ) async {
    await pumpTallForm(tester);

    final msgField = find.widgetWithText(TextField, 'Your message (optional)');
    expect(
      msgField,
      findsOneWidget,
      reason: 'the textarea must render as a real TextField (it rendered as '
          'an empty Container before — invisible)',
    );
    expect(
      tester.widget<TextField>(msgField).maxLines,
      4,
      reason: 'the multiline intent must survive (maxLines: 4)',
    );

    await tester.enterText(msgField, 'hello from widget test');
    await tester.pump();

    // The field shows the typed text AND the two-way binding flows it back
    // into the `message` signal — the trimmed-message display mirrors it
    // (C2b #9: the write-back merges into the user's @onChange).
    expect(
      find.text('hello from widget test'),
      findsNWidgets(2),
      reason: 'typed text must be visible in the field and mirrored by the '
          'trimmed-message display (write-back merge)',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('FormPage disabled input renders enabled:false (not editable)', (
    tester,
  ) async {
    await pumpTallForm(tester);

    final disabledField = find.widgetWithText(
      TextField,
      'This field is disabled',
    );
    expect(
      disabledField,
      findsOneWidget,
      reason: 'the disabled input must render as a TextField',
    );
    expect(
      tester.widget<TextField>(disabledField).enabled,
      isFalse,
      reason: 'disabled was silently dropped — the field rendered ENABLED '
          'before the fix',
    );

    // Tapping a disabled field must stay inert (no keyboard, no exception).
    await tester.tap(disabledField);
    await tester.pump();
    expect(tester.widget<TextField>(disabledField).enabled, isFalse);
    expect(tester.takeException(), isNull);
  });
}
