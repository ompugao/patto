import '../search/highlight.dart';

/// Index of the block that renders source line [row], given the row each
/// block starts on in source order: the last one starting at or before it,
/// since code blocks and tables span several lines.
int blockIndexForRow(List<int> blockRows, int row) {
  var low = 0;
  var high = blockRows.length;
  while (low < high) {
    final mid = (low + high) >> 1;
    if (blockRows[mid] <= row) {
      low = mid + 1;
    } else {
      high = mid;
    }
  }
  return (low - 1).clamp(0, blockRows.length - 1);
}

/// The blocks whose source lines contain [term], each once, in order.
List<int> findHitBlocks(
  List<String> sourceLines,
  List<int> blockRows,
  String term,
) {
  final hits = <int>[];
  for (var row = 0; row < sourceLines.length; row++) {
    if (!containsIgnoringCase(sourceLines[row], term)) continue;
    final index = blockIndexForRow(blockRows, row);
    // Rows ascend, so repeats of a multi-line block are adjacent.
    if (hits.isEmpty || hits.last != index) hits.add(index);
  }
  return hits;
}

/// Where find-in-note stands: the term, the blocks that match it and which
/// of them is current.
class NoteFind {
  NoteFind({this.term = '', this.hits = const [], this.current = 0})
    : hitSet = hits.toSet();

  final String term;
  final List<int> hits;
  final Set<int> hitSet;
  final int current;

  /// `3 / 7`, `No matches`, or nothing before a term is typed.
  String get countLabel => hits.isEmpty
      ? (term.isEmpty ? '' : 'No matches')
      : '${current + 1} / ${hits.length}';

  NoteFind withTerm(String term) =>
      NoteFind(term: term, hits: hits, current: current);

  /// New matches; [jump] restarts from the first, else the position is kept
  /// as far as the new list allows.
  NoteFind withHits(List<int> hits, {required bool jump}) => NoteFind(
    term: term,
    hits: hits,
    current: jump ? 0 : current.clamp(0, hits.isEmpty ? 0 : hits.length - 1),
  );

  NoteFind stepped(int delta) => NoteFind(
    term: term,
    hits: hits,
    current: (current + delta) % hits.length,
  );
}
