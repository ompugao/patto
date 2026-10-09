/// `[` followed by a partial name, excluding the block and decoration forms.
final _linkTrigger = RegExp(r'\[([^\[\]\s@*/`$_-]*)$');

/// The wiki link being typed on [line] when the caret is at [offset]: where
/// its `[` is and the name so far. Null outside a link, and for a URL, which
/// no note is named after.
({int start, String query})? pendingLink(String line, int offset) {
  final before = line.substring(0, offset.clamp(0, line.length));
  final match = _linkTrigger.firstMatch(before);
  if (match == null || match.group(1)!.startsWith('http')) return null;
  return (start: match.start, query: match.group(1)!);
}

/// Where a completed link replaces up to. The toolbar's `[]` leaves its
/// closing `]` after the caret; take it along rather than leave it dangling
/// after the link.
int linkReplaceEnd(String line, int caret) =>
    caret < line.length && line[caret] == ']' ? caret + 1 : caret;
