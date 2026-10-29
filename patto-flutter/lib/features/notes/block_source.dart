/// The source text of blocks [first] to [last] (default: just [first]): from
/// the row [first] starts on up to the row before the block after [last],
/// without trailing blank lines. A code block, table or quote spans several
/// rows; a line's children are blocks of their own.
String blockSource(
  List<String> sourceLines,
  List<int> blockRows,
  int first, [
  int? last,
]) {
  final next = (last ?? first) + 1;
  final start = blockRows[first].clamp(0, sourceLines.length);
  final end = next < blockRows.length ? blockRows[next] : sourceLines.length;
  final lines = sourceLines.sublist(
    start,
    end.clamp(start, sourceLines.length),
  );
  while (lines.length > 1 && lines.last.trim().isEmpty) {
    lines.removeLast();
  }
  return lines.join('\n');
}
