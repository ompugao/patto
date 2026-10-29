/// The source text of block [index]: from the row it starts on up to the row
/// before the next block, without trailing blank lines. A code block, table
/// or quote spans several rows; a line's children are blocks of their own.
String blockSource(List<String> sourceLines, List<int> blockRows, int index) {
  final start = blockRows[index].clamp(0, sourceLines.length);
  final end = index + 1 < blockRows.length
      ? blockRows[index + 1]
      : sourceLines.length;
  final lines = sourceLines.sublist(
    start,
    end.clamp(start, sourceLines.length),
  );
  while (lines.length > 1 && lines.last.trim().isEmpty) {
    lines.removeLast();
  }
  return lines.join('\n');
}
