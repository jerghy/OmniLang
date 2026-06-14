
void main() {
  // Use wider surface for complex layout
  testWidgets('TaskBoard renders (wide viewport)', (tester) async {
    tester.view.physicalSize = const Size(2400, 1600);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() {
      tester.view.resetPhysicalSize();
      tester.view.resetDevicePixelRatio();
    });

    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(body: Center(child: TaskBoard())),
      ),
    );

    await tester.pumpAndSettle();

    expect(find.text('Task Assignment Dashboard'), findsOneWidget);
    expect(find.text('Alice Chen'), findsOneWidget);
    expect(find.text('Bob Wang'), findsOneWidget);
    expect(find.text('Charlie Li'), findsOneWidget);
    expect(find.text('Total tasks: 4'), findsOneWidget);
  });
}
