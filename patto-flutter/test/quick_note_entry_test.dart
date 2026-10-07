import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/core/settings.dart';
import 'package:patto_flutter/features/quick_note/quick_note_entry.dart';

void main() {
  final now = DateTime(2026, 10, 7, 14, 5);

  group('target name', () {
    test('a daily note is named after today', () {
      expect(quickNoteTargetName(const Settings(), now), '2026-10-07');
    });

    test('the daily format is configurable', () {
      const settings = Settings(quickNoteDateFormat: 'yyyy/MM/dd');
      expect(quickNoteTargetName(settings, now), '2026/10/07');
    });

    test('a single note keeps its name', () {
      const settings = Settings(
        quickNoteTarget: QuickNoteTarget.single,
        quickNoteName: 'Inbox',
      );
      expect(quickNoteTargetName(settings, now), 'Inbox');
    });
  });

  group('entry', () {
    test('one line gets the time in front', () {
      expect(
        formatQuickNoteEntry('Call the dentist', timePrefix: true, now: now),
        '14:05 Call the dentist',
      );
    });

    test('the time can be left out', () {
      expect(
        formatQuickNoteEntry('Call the dentist', timePrefix: false, now: now),
        'Call the dentist',
      );
    });

    test('further lines nest under the first', () {
      expect(
        formatQuickNoteEntry(
          'Meeting\nask about budget\n\tand timeline\n',
          timePrefix: false,
          now: now,
        ),
        'Meeting\n\task about budget\n\t\tand timeline',
      );
    });

    test('blank lines and line endings are normalised', () {
      expect(
        formatQuickNoteEntry(
          '  first  \r\n\r\n\tsecond\r\n   \n',
          timePrefix: false,
          now: now,
        ),
        'first\n\t\tsecond',
      );
    });

    test('nothing to say gives nothing to append', () {
      expect(formatQuickNoteEntry(' \n\t\n', timePrefix: true, now: now), '');
    });
  });

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
