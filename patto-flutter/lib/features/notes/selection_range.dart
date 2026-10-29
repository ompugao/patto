/// The blocks a text selection covers, as (first, last) block indices.
///
/// [selected] holds the selected text each block last reported. Blocks keep
/// their entry after scrolling out of the list, so it may also hold leftovers
/// of an older selection; only the unbroken run around [latest], the block
/// that last reported text, belongs to the current one. Blocks without
/// text never report, so they do not break the run.
(int, int)? selectedRange(
  Map<int, String> selected,
  int? latest,
  int blockCount,
  bool Function(int index) hasText,
) {
  if (latest == null || selected.isEmpty) return null;
  // A selection shrinking past [latest] leaves it without an entry.
  final anchor = selected.keys.reduce(
    (a, b) => (a - latest).abs() <= (b - latest).abs() ? a : b,
  );
  bool inRun(int i) => selected.containsKey(i) || !hasText(i);

  var first = anchor;
  for (var i = anchor - 1; i >= 0 && inRun(i); i--) {
    if (selected.containsKey(i)) first = i;
  }
  var last = anchor;
  for (var i = anchor + 1; i < blockCount && inRun(i); i++) {
    if (selected.containsKey(i)) last = i;
  }
  return (first, last);
}
