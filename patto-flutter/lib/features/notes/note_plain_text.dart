import '../../src/rust/api/types.dart';

/// The text of a whole note as the viewer shows it, one block per line.
///
/// Copying a selection reads the text from the block widgets, but the list
/// only builds the blocks near the screen; a copy of the whole note needs it
/// from the blocks themselves.
String notePlainText(List<Block> blocks) =>
    [for (final block in blocks) ?blockPlainText(block)].join('\n');

/// The text [block] shows, or null when it shows none.
String? blockPlainText(Block block) {
  final text = switch (block.kind) {
    BlockKind_Line(:final spans) => _spans(spans),
    BlockKind_Blank() => '',
    BlockKind_Code(:final lines) => lines.join('\n'),
    BlockKind_Math(:final tex) => tex,
    BlockKind_Table(:final caption, :final rows) => [
      ?caption,
      for (final row in rows) row.cells.map((c) => _spans(c.spans)).join('\t'),
    ].join('\n'),
    BlockKind_Images(:final images) =>
      images.isEmpty || images.every((i) => i.alt == null)
          ? null
          : [for (final image in images) ?image.alt].join('\n'),
    BlockKind_Embed(:final embed) => embed.title ?? embed.url,
    BlockKind_Rule() => null,
  };
  if (block.kind is! BlockKind_Line || block.anchors.isEmpty) return text;
  return '$text\n${block.anchors.map((a) => '#$a').join(' ')}';
}

String _spans(List<NoteSpan> spans) => spans.map(_span).join();

String _span(NoteSpan span) => switch (span) {
  NoteSpan_Text(:final text) => text,
  NoteSpan_Decoration(:final children) => _spans(children),
  NoteSpan_WikiLink(:final name, :final anchor) =>
    name.isEmpty ? '#$anchor' : (anchor == null ? name : '$name#$anchor'),
  NoteSpan_Url(:final url, :final title) => title ?? url,
  NoteSpan_InlineCode(:final code) => code,
  NoteSpan_InlineMath(:final tex) => tex,
  NoteSpan_Image() => '',
  NoteSpan_Embed(:final embed) => embed.title ?? embed.url,
};
