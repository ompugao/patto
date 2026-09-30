import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/editor/patto_editing_controller.dart';
import 'package:re_editor/re_editor.dart';

PattoEditingController _controller(String text, int line, int offset) {
  final c = PattoEditingController(
    delegate: CodeLineEditingController.fromText(text),
    onIndent: () {},
    onOutdent: () {},
  );
  c.selection = CodeLineSelection.collapsed(index: line, offset: offset);
  return c;
}

void main() {
  test('a new line keeps the tab indent', () {
    final c = _controller('root\n\t\tdeep', 1, 6);
    c.applyNewLine();
    expect(c.text, 'root\n\t\tdeep\n\t\t');
    expect(c.selection, const CodeLineSelection.collapsed(index: 2, offset: 2));
  });

  test('splitting inside the indentation does not double it', () {
    final c = _controller('\t\tdeep', 0, 1);
    c.applyNewLine();
    expect(c.text, '\t\n\t\tdeep');
  });

  test('enter on an indentation-only line outdents it', () {
    final c = _controller('a\n\t\t', 1, 2);
    c.applyNewLine();
    expect(c.text, 'a\n\t');
    expect(c.selection, const CodeLineSelection.collapsed(index: 1, offset: 1));
  });

  test('enter on an empty top-level line still breaks the line', () {
    final c = _controller('a\n', 1, 0);
    c.applyNewLine();
    expect(c.text, 'a\n\n');
  });
}
