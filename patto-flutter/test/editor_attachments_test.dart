import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/editor/attachments.dart';

void main() {
  group('names', () {
    test('keeps letters, digits, CJK, dashes and underscores', () {
      expect(sanitizeAttachmentName('Photo 2024 (1).PNG'), 'Photo_2024_1.png');
      expect(sanitizeAttachmentName('図面-最終.jpeg'), '図面-最終.jpeg');
      expect(sanitizeAttachmentName('a.b.tar.gz'), 'a_b_tar.gz');
      expect(sanitizeAttachmentName('???'), 'file.bin');
      expect(sanitizeAttachmentName('.hidden'), 'hidden.bin');
      expect(sanitizeAttachmentName('README'), 'README.bin');
      expect(sanitizeAttachmentName('clip.mp4'), 'clip.mp');
      expect(sanitizeAttachmentName('図面・最終゛.png'), '図面_最終.png');
      expect(sanitizeAttachmentName('café résumé.pdf'), 'caf_r_sum.pdf');
    });

    test('a pasted image is named after the note and the time', () {
      final png = Uint8List.fromList([0x89, 0x50, 0x4e, 0x47, 0, 0]);
      final jpg = Uint8List.fromList([0xff, 0xd8, 0xff, 0xe0]);
      final at = DateTime(2026, 10, 7, 9, 5, 3);
      final svg = Uint8List.fromList('<svg'.codeUnits);
      expect(
        pastedImageName('a.pn', svg, at, mimeType: 'image/svg+xml'),
        'a-20261007-090503.svg',
      );
      expect(pastedImageName('a.pn', svg, at), 'a-20261007-090503.png');
      expect(
        pastedImageName('daily/2026-10-07.pn', png, at),
        '2026-10-07-20261007-090503.png',
      );
      expect(
        pastedImageName('Trip plan.pn', jpg, at),
        'Trip_plan-20261007-090503.jpg',
      );
    });

    test('image formats are told by their leading bytes', () {
      Uint8List bytes(List<int> b) => Uint8List.fromList(b);
      expect(imageExtensionOf(bytes([0x47, 0x49, 0x46, 0x38, 0x39])), 'gif');
      expect(
        imageExtensionOf(
          bytes([0x52, 0x49, 0x46, 0x46, 1, 2, 3, 4, 0x57, 0x45, 0x42, 0x50]),
        ),
        'webp',
      );
      expect(imageExtensionOf(bytes([0x25, 0x50, 0x44, 0x46])), isNull);
      expect(imageExtensionOf(bytes([])), isNull);
    });
  });

  group('snippets', () {
    test('attachments by kind', () {
      expect(
        attachmentSnippet('attachments/a.png'),
        '[@img ./attachments/a.png]',
      );
      expect(
        attachmentSnippet('attachments/Paper_v2.pdf'),
        '[@embed ./attachments/Paper_v2.pdf Paper v2]',
      );
      expect(
        attachmentSnippet('attachments/data.csv'),
        '[./attachments/data.csv]',
      );
    });

    test('web addresses by what they point at', () {
      expect(
        urlSnippet('https://www.youtube.com/watch?v=x'),
        '[@embed https://www.youtube.com/watch?v=x]',
      );
      expect(urlSnippet('https://youtu.be/x'), '[@embed https://youtu.be/x]');
      expect(
        urlSnippet('https://x.com/a/status/1'),
        '[@embed https://x.com/a/status/1]',
      );
      expect(
        urlSnippet('https://example.com/p.PDF'),
        '[@embed https://example.com/p.PDF]',
      );
      expect(
        urlSnippet('https://example.com/i.jpg?s=1'),
        '[@img https://example.com/i.jpg?s=1]',
      );
      expect(urlSnippet('https://example.com/'), '[https://example.com/]');
      expect(
        urlSnippet('https://example.com/', title: ' Example [site]\n'),
        '[https://example.com/ Example site]',
      );
      expect(
        urlSnippet('https://notyoutube.com/x'),
        '[https://notyoutube.com/x]',
      );
      expect(
        urlSnippet('https://example.com/?tags[]=a|b "c"'),
        '[https://example.com/?tags%5B%5D=a%7Cb%20%22c%22]',
      );
      expect(
        urlSnippet('https://example.com/', title: 'Visit https://x.y/z now'),
        '[https://example.com/ Visit now]',
      );
      expect(
        urlSnippet('https://example.com/a?b=c&d=e#f'),
        '[https://example.com/a?b=c&d=e#f]',
      );
    });

    test('only a lone address counts as one', () {
      expect(isWebUrl(' https://example.com/a?b=c '), isTrue);
      expect(isWebUrl('see https://example.com'), isFalse);
      expect(isWebUrl('ftp://example.com'), isFalse);
    });
  });

  group('folder', () {
    test('is trimmed, defaulted when empty and refused when unspellable', () {
      expect(normalizeAttachmentsDir(''), 'attachments');
      expect(normalizeAttachmentsDir(' / '), 'attachments');
      expect(normalizeAttachmentsDir('/media/notes/'), 'media/notes');
      expect(normalizeAttachmentsDir('添付'), '添付');
      expect(normalizeAttachmentsDir('my files'), isNull);
      expect(normalizeAttachmentsDir('../up'), isNull);
      expect(normalizeAttachmentsDir('a/./b'), isNull);
      expect(normalizeAttachmentsDir('a//b'), isNull);
    });
  });

  group('saving', () {
    late Directory root;
    setUp(
      () async => root = await Directory.systemTemp.createTemp('patto-att'),
    );
    tearDown(() => root.delete(recursive: true));

    test('writes under attachments and numbers a clash', () async {
      final bytes = Uint8List.fromList([1, 2, 3]);
      expect(
        await saveAttachment(root.path, 'attachments', 'a b.png', bytes),
        'attachments/a_b.png',
      );
      expect(
        await saveAttachment(root.path, 'attachments', 'a_b.png', bytes),
        'attachments/a_b-2.png',
      );
      expect(
        await saveAttachment(root.path, 'attachments', 'a_b.png', bytes),
        'attachments/a_b-3.png',
      );
      expect(
        await File('${root.path}/attachments/a_b-3.png').readAsBytes(),
        bytes,
      );
    });
  });
}
