import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/editor/caret_keys.dart';
import 'package:re_editor/re_editor.dart';

Future<(CodeLineEditingController, FocusNode)> _pump(
  WidgetTester tester,
  String text,
) async {
  final controller = CodeLineEditingController.fromText(text);
  final focusNode = FocusNode();
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        appBar: AppBar(
          actions: [
            IconButton(icon: const Icon(Icons.check), onPressed: () {}),
          ],
        ),
        body: CaretKeys(
          controller: controller,
          child: CodeEditor(controller: controller, focusNode: focusNode),
        ),
      ),
    ),
  );
  await tester.pump();
  focusNode.requestFocus();
  await tester.pump();
  return (controller, focusNode);
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
  // re_editor reads the platform once, at the first build, so every test
  // runs as Android.
  final android = TargetPlatformVariant.only(TargetPlatform.android);

  testWidgets('arrow keys move the caret instead of the focus', (tester) async {
    final (c, focusNode) = await _pump(tester, 'abc\nxyz');
    c.selection = const CodeLineSelection.collapsed(index: 0, offset: 1);
    await tester.pump();

    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    expect(_caret(c), (0, 2));
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
    expect(_caret(c), (0, 1));
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    expect(_caret(c), (1, 1));
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowUp);
    expect(_caret(c), (0, 1));
    expect(FocusManager.instance.primaryFocus, same(focusNode));
    await _tearDown(tester);
  }, variant: android);

  testWidgets('the caret crosses line ends', (tester) async {
    final (c, _) = await _pump(tester, 'abc\nxyz');
    c.selection = const CodeLineSelection.collapsed(index: 0, offset: 3);
    await tester.pump();

    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    expect(_caret(c), (1, 0));
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
    expect(_caret(c), (0, 3));
    await _tearDown(tester);
  }, variant: android);

  testWidgets('shift with an arrow key extends the selection', (tester) async {
    final (c, _) = await _pump(tester, 'abc\nxyz');
    c.selection = const CodeLineSelection.collapsed(index: 0, offset: 1);
    await tester.pump();

    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    expect(c.selection.baseOffset, 1);
    expect(c.selection.extentOffset, 3);
    expect(c.selectedText, 'bc');
    await _tearDown(tester);
  }, variant: android);
}
