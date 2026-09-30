import 'package:re_editor/re_editor.dart';

import 'outline.dart';

/// re_editor's controller with patto's tab indentation.
///
/// re_editor only counts spaces as indentation, so on its own a new line
/// under a tab-indented one falls back to the top level, and its Tab key
/// inserts spaces, which patto does not read as nesting. Every Enter, from
/// a soft keyboard or a hardware one, goes through [applyNewLine] here.
class PattoEditingController extends CodeLineEditingControllerDelegate {
  PattoEditingController({
    required super.delegate,
    required this.onIndent,
    required this.onOutdent,
  });

  final void Function() onIndent;
  final void Function() onOutdent;

  @override
  void applyNewLine() {
    final selection = this.selection;
    final line = codeLines[selection.startIndex].text;

    // Enter on a line that is nothing but indentation climbs out a level, as
    // in an outliner, rather than leaving an empty line behind.
    if (selection.isCollapsed &&
        line.isNotEmpty &&
        isBlank(line) &&
        selection.startOffset == line.length) {
      final lines = CodeLines.from(codeLines);
      lines[selection.startIndex] = CodeLine(line.substring(1));
      runRevocableOp(() {
        value = value.copyWith(
          codeLines: lines,
          selection: CodeLineSelection.collapsed(
            index: selection.startIndex,
            offset: line.length - 1,
          ),
        );
      });
      return;
    }

    // Tabs after the caret stay with the text that moves down, so only the
    // ones before it are repeated.
    final tabs = depthOf(line).clamp(0, selection.startOffset);
    replaceSelection('\n${'\t' * tabs}');
    makeCursorVisible();
  }

  @override
  void applyIndent() => onIndent();

  @override
  void applyOutdent() => onOutdent();
}
