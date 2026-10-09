import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/editor/link_completion.dart';

void main() {
  group('pendingLink', () {
    test('a bracket and a partial name is a link being typed', () {
      expect(pendingLink('see [proj', 9), (start: 4, query: 'proj'));
      expect(pendingLink('[', 1), (start: 0, query: ''));
    });

    test('only the text before the caret counts', () {
      expect(pendingLink('[proj] later', 3), (start: 0, query: 'pr'));
      expect(pendingLink('[ab', 99), (start: 0, query: 'ab'));
    });

    test('a closed link, a command or a decoration is not one', () {
      expect(pendingLink('[done]', 6), isNull);
      expect(pendingLink('[@code', 6), isNull);
      expect(pendingLink('[* bold', 7), isNull);
      expect(pendingLink('[`x', 3), isNull);
      expect(pendingLink('[a b', 4), isNull);
    });

    test('a web address is not a note name', () {
      expect(pendingLink('[https://x', 10), isNull);
    });
  });

  group('linkReplaceEnd', () {
    test('takes the closing bracket after the caret along', () {
      expect(linkReplaceEnd('[ab]', 3), 4);
    });

    test('stops at the caret otherwise', () {
      expect(linkReplaceEnd('[ab', 3), 3);
      expect(linkReplaceEnd('[ab x', 3), 3);
    });
  });
}
