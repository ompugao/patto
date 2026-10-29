import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/notes/selection_range.dart';

void main() {
  bool allText(int _) => true;

  test('nothing selected gives no range', () {
    expect(selectedRange({}, null, 10, allText), isNull);
  });

  test('covers the unbroken run around the latest block', () {
    expect(selectedRange({3: 'c', 4: 'd', 5: 'e'}, 4, 10, allText), (3, 5));
  });

  test('leaves out an older selection separated by a gap', () {
    expect(selectedRange({0: 'old', 4: 'd', 5: 'e'}, 5, 10, allText), (4, 5));
  });

  test('blocks without text do not break the run', () {
    bool hasText(int i) => i != 4;
    expect(selectedRange({3: 'c', 5: 'e'}, 5, 10, hasText), (3, 5));
  });

  test('a run never ends on a block without text', () {
    bool hasText(int i) => i != 6;
    expect(selectedRange({4: 'd', 5: 'e'}, 5, 10, hasText), (4, 5));
  });

  test('falls back to the nearest block once the latest is deselected', () {
    expect(selectedRange({3: 'c', 4: 'd'}, 5, 10, allText), (3, 4));
  });
}
