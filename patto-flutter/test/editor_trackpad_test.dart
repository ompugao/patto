import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/editor/trackpad.dart';
import 'package:re_editor/re_editor.dart';

/// A trackpad on an editor showing [text], whose test font is 10px a
/// character with 20px lines.
Future<(CodeLineEditingController, CaretTrackpad)> _pump(
  WidgetTester tester,
  String text,
) async {
  final controller = CodeLineEditingController.fromText(text);
  CodeIndicatorValueNotifier? layout;
  final trackpad = CaretTrackpad(
    controller: controller,
    paragraphs: () => layout?.value?.paragraphs,
  );
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: CodeEditor(
          controller: controller,
          style: CodeEditorStyle(fontSize: 10, fontHeight: 2),
          indicatorBuilder: (context, editing, chunks, notifier) {
            layout = notifier;
            return const SizedBox(width: 1);
          },
        ),
      ),
    ),
  );
  await tester.pump();
  return (controller, trackpad);
}

/// re_editor's caret blink outlives the editor; stop it before the test ends.
Future<void> _tearDown(WidgetTester tester) async {
  FocusManager.instance.primaryFocus?.unfocus();
  await tester.pump(const Duration(seconds: 1));
  await tester.pumpWidget(const SizedBox());
  await tester.pump(const Duration(seconds: 1));
}

(int, int) _caret(CodeLineEditingController c) =>
    (c.selection.extentIndex, c.selection.extentOffset);

void main() {
  testWidgets('the caret follows the finger across characters and lines', (
    tester,
  ) async {
    final controller = CodeLineEditingController.fromText('abcdef\nxyz\n');
    CodeIndicatorValueNotifier? layout;
    final trackpad = CaretTrackpad(
      controller: controller,
      paragraphs: () => layout?.value?.paragraphs,
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: CodeEditor(
            controller: controller,
            style: CodeEditorStyle(fontSize: 10, fontHeight: 2),
            indicatorBuilder: (context, editing, chunks, notifier) {
              layout = notifier;
              return const SizedBox(width: 1);
            },
          ),
        ),
      ),
    );
    await tester.pump();
    controller.selection = const CodeLineSelection.collapsed(
      index: 0,
      offset: 1,
    );
    await tester.pump();

    // The test font is 10px a character; the line is 20px high.
    trackpad.start();
    trackpad.move(const Offset(21, 0));
    expect(controller.selection.extentOffset, 3);

    trackpad.move(const Offset(0, 21));
    await tester.pump();
    expect(controller.selection.extentIndex, 1);
    expect(controller.selection.extentOffset, 3);

    // Past the end of the short line, coming back starts from its end.
    trackpad.move(const Offset(200, 0));
    expect(controller.selection.extentOffset, 3);
    trackpad.move(const Offset(-10, 0));
    expect(controller.selection.extentOffset, 2);

    // re_editor's caret blink outlives the editor; stop it first.
    FocusManager.instance.primaryFocus?.unfocus();
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpWidget(const SizedBox());
    await tester.pump(const Duration(seconds: 1));
  });

  group('keeps the column across an empty line', () {
    Future<void> drag(
      WidgetTester tester,
      CaretTrackpad trackpad,
      List<Offset> steps,
    ) async {
      for (final step in steps) {
        trackpad.move(step);
        await tester.pump();
      }
    }

    testWidgets('moving straight down', (tester) async {
      final (c, trackpad) = await _pump(tester, 'abcdef\n\nabcdef');
      c.selection = const CodeLineSelection.collapsed(index: 0, offset: 4);
      await tester.pump();
      trackpad.start();

      await drag(tester, trackpad, [const Offset(0, 21)]);
      expect(_caret(c), (1, 0));
      await drag(tester, trackpad, [const Offset(0, 21)]);
      expect(_caret(c), (2, 4));
      await _tearDown(tester);
    });

    testWidgets('with the finger drifting sideways', (tester) async {
      final (c, trackpad) = await _pump(tester, 'abcdef\n\nabcdef');
      c.selection = const CodeLineSelection.collapsed(index: 0, offset: 4);
      await tester.pump();
      trackpad.start();

      await drag(tester, trackpad, [
        for (var i = 0; i < 6; i++) Offset(i.isEven ? -2 : 2, 7),
      ]);
      expect(_caret(c), (2, 4));
      await _tearDown(tester);
    });

    testWidgets('but not after dragging back on it', (tester) async {
      final (c, trackpad) = await _pump(tester, 'abcdef\n\nabcdef');
      c.selection = const CodeLineSelection.collapsed(index: 0, offset: 4);
      await tester.pump();
      trackpad.start();

      await drag(tester, trackpad, [
        const Offset(0, 21),
        const Offset(-30, 0),
        const Offset(0, 21),
      ]);
      expect(_caret(c), (2, 0));
      await _tearDown(tester);
    });
  });
}
