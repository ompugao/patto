import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/notes/widgets/find_bar.dart';

void main() {
  Future<List<int>> pump(
    WidgetTester tester, {
    required bool hasHits,
    String countLabel = '',
  }) async {
    final steps = <int>[];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          appBar: AppBar(
            bottom: FindBar(
              controller: TextEditingController(),
              countLabel: countLabel,
              hasHits: hasHits,
              onChanged: (_) {},
              onStep: steps.add,
            ),
          ),
        ),
      ),
    );
    return steps;
  }

  testWidgets('the arrows are disabled without matches', (tester) async {
    await pump(tester, hasHits: false, countLabel: 'No matches');
    expect(find.text('No matches'), findsOneWidget);
    final up = tester.widget<IconButton>(
      find.widgetWithIcon(IconButton, Icons.keyboard_arrow_up),
    );
    expect(up.onPressed, isNull);
  });

  testWidgets('the arrows and submit step through the matches', (tester) async {
    final steps = await pump(tester, hasHits: true, countLabel: '1 / 2');
    await tester.tap(find.byTooltip('Next match'));
    await tester.tap(find.byTooltip('Previous match'));
    await tester.enterText(find.byType(TextField), 'tea');
    await tester.testTextInput.receiveAction(TextInputAction.search);
    expect(steps, [1, -1, 1]);
  });
}
