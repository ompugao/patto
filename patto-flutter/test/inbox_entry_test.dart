import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/inbox/inbox_entry.dart';

void main() {
  group('shared text', () {
    test('a link with a subject becomes a patto link', () {
      expect(
        draftFromShared(
          text: 'https://example.com/post?id=1',
          subject: 'A good read',
        ),
        '[https://example.com/post?id=1 A good read]',
      );
    });

    test('brackets in the subject cannot break the link', () {
      expect(
        draftFromShared(text: 'https://example.com/', subject: 'Foo [draft]'),
        '[https://example.com/ Foo (draft)]',
      );
    });

    test('a link without a subject stays a link', () {
      expect(
        draftFromShared(text: 'https://example.com/'),
        'https://example.com/',
      );
    });

    test('plain text keeps its subject as the first line', () {
      expect(
        draftFromShared(text: 'body of the text', subject: 'Title'),
        'Title\nbody of the text',
      );
    });

    test('a subject equal to the text is not repeated', () {
      expect(draftFromShared(text: 'same', subject: 'same'), 'same');
    });

    test('text with spaces is never taken for a link', () {
      expect(
        draftFromShared(text: 'see https://example.com now', subject: 'S'),
        'S\nsee https://example.com now',
      );
    });
  });
}
