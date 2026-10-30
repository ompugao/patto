import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/notes/note_plain_text.dart';
import 'package:patto_flutter/src/rust/api/types.dart';

Block _block(BlockKind kind, {List<String> anchors = const []}) =>
    Block(row: 0, depth: 0, quoteDepth: 0, anchors: anchors, kind: kind);

EmbedRef _embed(String? title) => EmbedRef(
  url: 'https://example.com',
  title: title,
  kind: const EmbedKind.other(),
  isLocal: false,
);

void main() {
  test('a line shows its spans as the viewer labels them', () {
    final line = _block(
      BlockKind.line(
        spans: [
          const NoteSpan.text(text: 'see '),
          const NoteSpan.decoration(
            fontsize: 1,
            italic: false,
            underline: false,
            deleted: false,
            children: [NoteSpan.wikiLink(name: 'other', anchor: 'sec')],
          ),
          const NoteSpan.text(text: ', '),
          const NoteSpan.wikiLink(name: '', anchor: 'top'),
          const NoteSpan.text(text: ', '),
          const NoteSpan.url(url: 'https://example.com', title: 'site'),
          const NoteSpan.text(text: ', '),
          const NoteSpan.inlineCode(code: 'x = 1'),
          const NoteSpan.text(text: ', '),
          const NoteSpan.inlineMath(tex: 'a^2'),
          NoteSpan.embed(embed: _embed(null)),
        ],
      ),
    );

    expect(
      blockPlainText(line),
      'see other#sec, #top, site, x = 1, a^2https://example.com',
    );
  });

  test('a line lists its anchors on the next line', () {
    final line = _block(
      const BlockKind.line(spans: [NoteSpan.text(text: 'heading')]),
      anchors: ['a', 'b'],
    );

    expect(blockPlainText(line), 'heading\n#a #b');
  });

  test('a table puts tabs between cells and newlines between rows', () {
    NoteTableRow row(List<String> cells) => NoteTableRow(
      cells: [
        for (final c in cells) NoteTableCell(spans: [NoteSpan.text(text: c)]),
      ],
    );
    final table = _block(
      BlockKind.table(
        caption: 'cap',
        rows: [
          row(['a', 'b']),
          row(['c', 'd']),
        ],
      ),
    );

    expect(blockPlainText(table), 'cap\na\tb\nc\td');
  });

  test('blocks without text are left out of the note', () {
    final note = notePlainText([
      _block(const BlockKind.line(spans: [NoteSpan.text(text: 'one')])),
      _block(const BlockKind.blank()),
      _block(const BlockKind.rule()),
      _block(const BlockKind.code(lang: 'dart', lines: ['a', 'b'])),
      _block(
        const BlockKind.images(
          images: [ImageRef(src: 'x.png', alt: null, isLocal: true)],
        ),
      ),
      _block(BlockKind.embed(embed: _embed('Example'))),
      _block(const BlockKind.math(tex: r'\sum')),
    ]);

    expect(note, 'one\n\na\nb\nExample\n\\sum');
  });
}
