import 'package:re_editor/re_editor.dart';

import 'outline.dart';

/// re_editor's controller with patto's tab indentation. re_editor counts
/// only spaces as indentation; every Enter, soft or hardware, comes here.
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

    // On a line of nothing but tabs, Enter outdents it, as in an outliner.
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

    // Tabs after the caret move down with the text; repeat only those before.
    final tabs = depthOf(line).clamp(0, selection.startOffset);
    replaceSelection('\n${'\t' * tabs}');
    makeCursorVisible();
  }

  @override
  void applyIndent() => onIndent();

  @override
  void applyOutdent() => onOutdent();
}
