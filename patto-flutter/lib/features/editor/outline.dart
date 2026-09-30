import 'package:re_editor/re_editor.dart';

/// Outline structure of a patto note, read from its leading tabs.
///
/// A line's block is the line itself plus every following line that is
/// indented deeper. Blank lines belong to a block when the next non-blank line
/// still does, which is how the parser treats them inside code blocks too.
///
/// Everything here works on the lines the editor shows; a folded block counts
/// as its one visible line.

/// Whether [line] has nothing but indentation. The parser gives such a line no
/// depth of its own.
bool isBlank(String line) {
  for (var i = 0; i < line.length; i++) {
    if (line.codeUnitAt(i) != 0x09) return false;
  }
  return true;
}

/// Number of leading tabs.
int depthOf(String line) {
  var i = 0;
  while (i < line.length && line.codeUnitAt(i) == 0x09) {
    i++;
  }
  return i;
}

/// Exclusive end of the block that starts at [row].
int blockEnd(List<String> lines, int row) {
  final depth = depthOf(lines[row]);
  var end = row + 1;
  for (var i = row + 1; i < lines.length; i++) {
    if (isBlank(lines[i])) continue;
    if (depthOf(lines[i]) <= depth) break;
    end = i + 1;
  }
  return end;
}

/// The line [row] is nested under, or null at the top level.
int? parentOf(List<String> lines, int row) {
  if (isBlank(lines[row])) return null;
  final depth = depthOf(lines[row]);
  if (depth == 0) return null;
  for (var i = row - 1; i >= 0; i--) {
    if (isBlank(lines[i])) continue;
    if (depthOf(lines[i]) < depth) return i;
  }
  return null;
}

/// How many indent guides run through [row].
///
/// A blank line has no indentation of its own, so it takes the shallower of
/// its neighbours, and the guides carry on through it.
int guideDepth(List<String> lines, int row) {
  if (!isBlank(lines[row])) return depthOf(lines[row]);
  var above = 0;
  for (var i = row - 1; i >= 0; i--) {
    if (!isBlank(lines[i])) {
      above = depthOf(lines[i]);
      break;
    }
  }
  var below = 0;
  for (var i = row + 1; i < lines.length; i++) {
    if (!isBlank(lines[i])) {
      below = depthOf(lines[i]);
      break;
    }
  }
  return above < below ? above : below;
}

/// The indent column to emphasise while the caret is on [row], and the rows
/// it spans.
///
/// On a line with children that is the column of its own children; otherwise
/// it is the column of its siblings under the same parent.
({int column, int start, int end})? activeGuide(List<String> lines, int row) {
  if (row < 0 || row >= lines.length || isBlank(lines[row])) return null;
  final end = blockEnd(lines, row);
  if (end > row + 1) {
    return (column: depthOf(lines[row]), start: row + 1, end: end);
  }
  final parent = parentOf(lines, row);
  if (parent == null) return null;
  return (
    column: depthOf(lines[parent]),
    start: parent + 1,
    end: blockEnd(lines, parent),
  );
}

/// Start of the sibling block right above the block at [row], or null when
/// [row] is the first child of its parent.
int? previousSibling(List<String> lines, int row) {
  final depth = depthOf(lines[row]);
  for (var i = row - 1; i >= 0; i--) {
    if (isBlank(lines[i])) continue;
    final d = depthOf(lines[i]);
    if (d < depth) return null;
    if (d == depth) return i;
  }
  return null;
}

/// Start of the sibling block right below the block at [row], or null when
/// it is the last child of its parent.
int? nextSibling(List<String> lines, int row) {
  final depth = depthOf(lines[row]);
  for (var i = blockEnd(lines, row); i < lines.length; i++) {
    if (isBlank(lines[i])) continue;
    return depthOf(lines[i]) == depth ? i : null;
  }
  return null;
}

/// The row order after swapping the block at [row] with its neighbouring
/// sibling, and where [row] ends up; null when there is no sibling that way.
///
/// Blank lines between the two blocks stay between them.
({List<int> order, int row})? moveBlock(
  List<String> lines,
  int row, {
  required bool up,
}) {
  final int first;
  final int second;
  if (up) {
    final prev = previousSibling(lines, row);
    if (prev == null) return null;
    (first, second) = (prev, row);
  } else {
    final next = nextSibling(lines, row);
    if (next == null) return null;
    (first, second) = (row, next);
  }
  final firstEnd = blockEnd(lines, first);
  final secondEnd = blockEnd(lines, second);
  final order = [
    for (var i = 0; i < first; i++) i,
    for (var i = second; i < secondEnd; i++) i,
    for (var i = firstEnd; i < second; i++) i,
    for (var i = first; i < firstEnd; i++) i,
    for (var i = secondEnd; i < lines.length; i++) i,
  ];
  return (order: order, row: order.indexOf(row));
}

/// Folds every line that has children.
class PattoIndentChunkAnalyzer implements CodeChunkAnalyzer {
  const PattoIndentChunkAnalyzer();

  @override
  List<CodeChunk> run(CodeLines codeLines) {
    final lines = [
      for (var i = 0; i < codeLines.length; i++) codeLines[i].text,
    ];
    return [
      for (var i = 0; i < lines.length; i++)
        if (!isBlank(lines[i]))
          if (blockEnd(lines, i) case final end when end > i + 1)
            CodeChunk(i, end),
    ];
  }
}
