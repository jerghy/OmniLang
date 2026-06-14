import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:omni_flutter_app/shoppingcart.dart';

void main() {
  testWidgets('ShoppingCart renders initial state', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: Center(
            child: ShoppingCart(),
          ),
        ),
      ),
    );

    await tester.pumpAndSettle();

    // Verify key elements are present
    expect(find.text('Shopping Cart'), findsOneWidget);
    expect(find.text('Products'), findsOneWidget);
    expect(find.text('Apple - \$5'), findsOneWidget);
    expect(find.text('Banana - \$3'), findsOneWidget);
    expect(find.text('Orange - \$4'), findsOneWidget);
    expect(find.text('Your Cart'), findsOneWidget);
    expect(find.text('Items: 0'), findsOneWidget);
    expect(find.text('Total: \$0'), findsOneWidget);

    // All 4 buttons present
    expect(find.widgetWithText(ElevatedButton, 'Add to Cart'), findsNWidgets(3));
    expect(find.widgetWithText(OutlinedButton, 'Clear Cart'), findsOneWidget);
  });

  testWidgets('ShoppingCart click adds item', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: Center(
            child: ShoppingCart(),
          ),
        ),
      ),
    );

    await tester.pumpAndSettle();

    // Click first "Add to Cart" (Apple)
    final addButtons = find.widgetWithText(ElevatedButton, 'Add to Cart');
    await tester.tap(addButtons.first);
    await tester.pumpAndSettle();

    // Verify state update
    expect(find.text('Items: 1'), findsOneWidget);
    expect(find.text('Total: \$5'), findsOneWidget);
    expect(find.text('Apple (\$5)'), findsOneWidget);
  });

  testWidgets('ShoppingCart accumulates multiple items', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: Center(
            child: ShoppingCart(),
          ),
        ),
      ),
    );

    await tester.pumpAndSettle();

    // Add Apple + Banana
    final addButtons = find.widgetWithText(ElevatedButton, 'Add to Cart');
    await tester.tap(addButtons.first);
    await tester.pumpAndSettle();
    await tester.tap(addButtons.at(1));
    await tester.pumpAndSettle();

    expect(find.text('Items: 2'), findsOneWidget);
    expect(find.text('Total: \$8'), findsOneWidget);
    expect(find.text('Banana (\$3)'), findsOneWidget);
  });

  testWidgets('ShoppingCart clear works', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: Center(
            child: ShoppingCart(),
          ),
        ),
      ),
    );

    await tester.pumpAndSettle();

    // Add items then clear
    final addButtons = find.widgetWithText(ElevatedButton, 'Add to Cart');
    await tester.tap(addButtons.at(0));
    await tester.pumpAndSettle();
    await tester.tap(addButtons.at(2)); // Orange
    await tester.pumpAndSettle();

    expect(find.text('Items: 2'), findsOneWidget);

    // Clear
    await tester.tap(find.widgetWithText(OutlinedButton, 'Clear Cart'));
    await tester.pumpAndSettle();

    expect(find.text('Items: 0'), findsOneWidget);
    expect(find.text('Total: \$0'), findsOneWidget);
  });

  testWidgets('ShoppingCart post-clear re-add works', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: Center(
            child: ShoppingCart(),
          ),
        ),
      ),
    );

    await tester.pumpAndSettle();

    // Add, clear, re-add
    final addButtons = find.widgetWithText(ElevatedButton, 'Add to Cart');
    await tester.tap(addButtons.first);
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(OutlinedButton, 'Clear Cart'));
    await tester.pumpAndSettle();
    await tester.tap(addButtons.last); // Orange
    await tester.pumpAndSettle();

    // No stale closure — should be 1 item, not 2
    expect(find.text('Items: 1'), findsOneWidget);
    expect(find.text('Total: \$4'), findsOneWidget);
  });
}
