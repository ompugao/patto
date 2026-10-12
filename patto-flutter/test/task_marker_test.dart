import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/notes/widgets/task_marker.dart';
import 'package:patto_flutter/src/rust/api/types.dart';

void main() {
  final today = DateTime(2026, 6, 15);

  Future<Color?> background(
    WidgetTester tester,
    String due, {
    DateKind kind = DateKind.date,
  }) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: DueChip(
            due: TaskDate(text: due, kind: kind),
            now: today,
          ),
        ),
      ),
    );
    final box = tester.widget<Container>(find.byType(Container));
    return (box.decoration! as BoxDecoration).color;
  }

  testWidgets('an overdue task is marked with the error colour', (
    tester,
  ) async {
    final scheme = ThemeData().colorScheme;
    expect(await background(tester, '2026-06-14'), scheme.errorContainer);
  });

  testWidgets('a task due within a week is amber', (tester) async {
    expect(await background(tester, '2026-06-15'), const Color(0xFFFFF0C2));
    expect(await background(tester, '2026-06-22'), const Color(0xFFFFF0C2));
  });

  testWidgets('a task due later, or with an unreadable date, is plain', (
    tester,
  ) async {
    final scheme = ThemeData().colorScheme;
    expect(
      await background(tester, '2026-06-23'),
      scheme.surfaceContainerHighest,
    );
    expect(
      await background(tester, '2020-01-01', kind: DateKind.unparsed),
      scheme.surfaceContainerHighest,
    );
  });

  testWidgets('the marker only reacts to taps when given a handler', (
    tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(body: TaskMarker(status: TaskStatus.todo)),
      ),
    );
    expect(find.byType(InkResponse), findsNothing);

    var taps = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: TaskMarker(status: TaskStatus.done, onTap: () => taps++),
        ),
      ),
    );
    await tester.tap(find.byIcon(Icons.check_circle));
    expect(taps, 1);
  });
}
