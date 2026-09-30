import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/editor/trackpad.dart';
import 'package:re_editor/re_editor.dart';

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
}
