import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/editor/outline_edits.dart';
import 'package:re_editor/re_editor.dart';

const _note = 'root\n\tchild a\n\t\tgrand\n\n\tchild b\nnext';

(CodeLineEditingController, OutlineEdits) _edits(
  String text, {
  required int line,
  int offset = 0,
}) {
  final controller = CodeLineEditingController.fromText(text);
  controller.selection = CodeLineSelection.collapsed(
    index: line,
    offset: offset,
  );
  return (controller, OutlineEdits(controller));
}

void main() {
  group('reindent', () {
    test('indenting a line takes its children along', () {
      final (c, edits) = _edits(_note, line: 1, offset: 3);
      edits.reindent(add: true);
      expect(c.text, 'root\n\t\tchild a\n\t\t\tgrand\n\n\tchild b\nnext');
      expect(c.selection.extentOffset, 4);
    });

    test('outdenting a nested block lifts it and its children', () {
      final (c, edits) = _edits(_note, line: 1, offset: 3);
      edits.reindent(add: false);
      expect(c.text, 'root\nchild a\n\tgrand\n\n\tchild b\nnext');
      expect(c.selection.extentOffset, 2);
    });

    test('outdenting a top-level block does nothing', () {
      final (c, edits) = _edits(_note, line: 0);
      edits.reindent(add: false);
      expect(c.text, _note);
    });

    test('empty lines inside the block stay empty', () {
      final (c, edits) = _edits('a\n\n\tb', line: 0);
      edits.reindent(add: true);
      expect(c.text, '\ta\n\n\t\tb');
    });

    test('a selection ending at the start of a line leaves that line', () {
      final (c, edits) = _edits('a\nb\nc', line: 0);
      c.selection = const CodeLineSelection(
        baseIndex: 0,
        baseOffset: 0,
        extentIndex: 1,
        extentOffset: 0,
      );
      edits.reindent(add: true);
      expect(c.text, '\ta\nb\nc');
    });

    test('is one undo step', () {
      final (c, edits) = _edits(_note, line: 0);
      edits.reindent(add: true);
      c.undo();
      expect(c.text, _note);
    });
  });

  group('moveBlock', () {
    test('moving down swaps the block with the sibling below it', () {
      final (c, edits) = _edits(_note, line: 1, offset: 2);
      edits.moveBlock(up: false);
      expect(c.text, 'root\n\tchild b\n\n\tchild a\n\t\tgrand\nnext');
      expect(c.selection.extentIndex, 3);
      expect(c.selection.extentOffset, 2);
    });

    test('moving up from the first sibling does nothing', () {
      final (c, edits) = _edits(_note, line: 1);
      edits.moveBlock(up: true);
      expect(c.text, _note);
    });

    test('blank lines between siblings stay where they are', () {
      final (c, edits) = _edits('a\n\nb', line: 2);
      edits.moveBlock(up: true);
      expect(c.text, 'b\n\na');
    });

    test('a blank line cannot be moved', () {
      final (c, edits) = _edits(_note, line: 3);
      edits.moveBlock(up: true);
      expect(c.text, _note);
    });
  });

  group('selectBlock', () {
    test('selects the line and its children', () {
      final (c, edits) = _edits(_note, line: 1, offset: 2);
      edits.selectBlock();
      expect(c.selection.startIndex, 1);
      expect(c.selection.endIndex, 2);
      expect(c.selectedText, '\tchild a\n\t\tgrand');
    });

    test('selecting again widens to the parent block', () {
      final (c, edits) = _edits(_note, line: 1);
      edits.selectBlock();
      edits.selectBlock();
      expect(c.selection.startIndex, 0);
      expect(c.selection.endIndex, 4);
      expect(c.selection.endOffset, '\tchild b'.length);
    });

    test('a top-level block stays selected on a third tap', () {
      final (c, edits) = _edits(_note, line: 0);
      edits.selectBlock();
      final once = c.selection;
      edits.selectBlock();
      expect(c.selection, once);
    });
  });

  test('jumpTo puts the caret after the indentation', () {
    final (c, edits) = _edits(_note, line: 0);
    edits.jumpTo(2);
    expect(c.selection, const CodeLineSelection.collapsed(index: 2, offset: 2));
  });

  group('insert', () {
    test('leaves the caret inside the brackets when asked', () {
      final (c, edits) = _edits('ab', line: 0, offset: 1);
      edits.insert('[]', 1);
      expect(c.text, 'a[]b');
      expect(
        c.selection,
        const CodeLineSelection.collapsed(index: 0, offset: 2),
      );
    });

    test('otherwise leaves the caret after the text', () {
      final (c, edits) = _edits('ab', line: 0, offset: 2);
      edits.insert('[@quote]');
      expect(c.text, 'ab[@quote]');
      expect(c.selection.extentOffset, 10);
    });
  });

  test('a code block makes its children verbatim', () {
    final (_, edits) = _edits('[@code py]\n\tx = 1\n\t\ty\nafter', line: 0);
    expect(edits.inVerbatimBlock(1), isTrue);
    expect(edits.inVerbatimBlock(2), isTrue);
    expect(edits.inVerbatimBlock(0), isFalse);
    expect(edits.inVerbatimBlock(3), isFalse);
    expect(edits.inVerbatimBlock(9), isFalse);
  });
}
