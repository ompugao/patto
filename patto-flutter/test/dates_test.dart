import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/core/dates.dart';

void main() {
  test('a date is written with zero-padded month and day', () {
    expect(isoDate(DateTime(2026, 3, 7)), '2026-03-07');
    expect(isoDate(DateTime(2026, 12, 25, 23, 59)), '2026-12-25');
  });
}
