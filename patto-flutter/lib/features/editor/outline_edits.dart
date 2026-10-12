import 'dart:math' as math;

import 'package:re_editor/re_editor.dart';

import 'outline.dart';
import 'outline.dart' as outline show moveBlock;
import 'patto_spans.dart';

/// Edits that treat the note as an outline: a line together with the lines
/// nested under it. Soft keyboards have no Tab key, so the toolbar is the way
/// to nest, move and select blocks.
class OutlineEdits {
  OutlineEdits(this.controller);

  final CodeLineEditingController controller;

  CodeLines? _linesSource;
  List<String> _linesCache = const [];

  /// The visible lines' text; a folded block is its one visible line.
  List<String> get lines {
    final codeLines = controller.codeLines;
    if (!identical(codeLines, _linesSource)) {
      _linesSource = codeLines;
      _linesCache = [
        for (var i = 0; i < codeLines.length; i++) codeLines[i].text,
      ];
    }
    return _linesCache;
  }

  bool inVerbatimBlock(int row) {
    final lines = this.lines;
    if (row >= lines.length) return false;
    for (int? p = parentOf(lines, row); p != null; p = parentOf(lines, p)) {
      if (opensVerbatim(lines[p])) return true;
    }
    return false;
  }

  /// Indents or outdents the selected lines together with their children.
  void reindent({required bool add}) {
    final selection = controller.selection;
    final lines = this.lines;
    final start = selection.startIndex;
    var last = selection.endIndex;
    // A selection that ends at the start of a line does not take that line.
    if (last > start && selection.endOffset == 0) last--;
    var end = last + 1;
    for (var i = start; i <= last; i++) {
      if (!isBlank(lines[i])) end = math.max(end, blockEnd(lines, i));
    }
    // Outdenting a top-level block would only flatten its children.
    if (!add && depthOf(lines[start]) == 0) return;

    final codeLines = CodeLines.from(controller.codeLines);
    final shift = <int, int>{};
    for (var i = start; i < end; i++) {
      final text = lines[i];
      // Leave empty lines empty instead of filling them with tabs.
      if (text.isEmpty) continue;
      if (!add && !text.startsWith('\t')) continue;
      codeLines[i] = _shifted(codeLines[i], add: add);
      shift[i] = add ? 1 : -1;
    }
    if (shift.isEmpty) return;

    int offsetOn(int index, int offset) =>
        (offset + (shift[index] ?? 0)).clamp(0, codeLines[index].length);
    _edit(
      codeLines,
      selection.copyWith(
        baseOffset: offsetOn(selection.baseIndex, selection.baseOffset),
        extentOffset: offsetOn(selection.extentIndex, selection.extentOffset),
      ),
    );
  }

  /// [line] one level deeper or shallower, along with any lines folded into it.
  static CodeLine _shifted(CodeLine line, {required bool add}) {
    final text = line.text;
    return CodeLine(
      add
          ? (text.isEmpty ? text : '\t$text')
          : (text.startsWith('\t') ? text.substring(1) : text),
      [for (final chunk in line.chunks) _shifted(chunk, add: add)],
    );
  }

  /// Swaps the caret's block with the sibling block above or below it.
  void moveBlock({required bool up}) {
    final selection = controller.selection;
    final lines = this.lines;
    final row = selection.startIndex;
    if (isBlank(lines[row])) return;
    final moved = outline.moveBlock(lines, row, up: up);
    if (moved == null) return;

    final old = controller.codeLines;
    final codeLines = CodeLines.of([for (final i in moved.order) old[i]]);
    _edit(
      codeLines,
      selection.copyWith(
        baseIndex: moved.order.indexOf(selection.baseIndex),
        extentIndex: moved.order.indexOf(selection.extentIndex),
      ),
    );
    controller.makeCursorCenterIfInvisible();
  }

  /// Selects the caret's block; once it is selected, widens to its parent's.
  void selectBlock() {
    final selection = controller.selection;
    final lines = this.lines;
    var start = selection.startIndex;
    var end = isBlank(lines[start]) ? start + 1 : blockEnd(lines, start);

    bool covers(int s, int e) =>
        selection.startIndex == s &&
        selection.startOffset == 0 &&
        selection.endIndex == e - 1 &&
        selection.endOffset == lines[e - 1].length;
    if (covers(start, end)) {
      final parent = parentOf(lines, start);
      if (parent == null) return;
      start = parent;
      end = blockEnd(lines, parent);
    }
    controller.selection = CodeLineSelection(
      baseIndex: start,
      baseOffset: 0,
      extentIndex: end - 1,
      extentOffset: lines[end - 1].length,
    );
  }

  /// Puts the caret at the start of the text on [row].
  void jumpTo(int row) {
    controller.selection = CodeLineSelection.collapsed(
      index: row,
      offset: depthOf(lines[row]),
    );
    controller.makeCursorCenterIfInvisible();
  }

  /// Inserts [text] and leaves the caret [back] characters before its end,
  /// inside the brackets where there is something to type.
  void insert(String text, [int back = 0]) {
    controller.replaceSelection(text);
    if (back == 0) return;
    final caret = controller.selection.extent;
    controller.selection = CodeLineSelection.collapsed(
      index: caret.index,
      offset: caret.offset - back,
    );
  }

  /// Replaces the lines as a single undoable step.
  void _edit(CodeLines codeLines, CodeLineSelection selection) {
    controller.runRevocableOp(() {
      controller.value = controller.value.copyWith(
        codeLines: codeLines,
        selection: selection,
      );
    });
  }
}
