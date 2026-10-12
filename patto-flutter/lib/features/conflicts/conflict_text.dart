import '../../src/rust/api/conflict.dart';
import 'conflict_state.dart';

/// Tabs as spaces, so indentation reads the same as in the note.
String displayLine(String line) => line.replaceAll('\t', '    ');

/// The part of [a] left once the prefix and suffix it shares with [b] are
/// taken off.
({int start, int end}) changedRange(String a, String b) {
  var prefix = 0;
  while (prefix < a.length && prefix < b.length && a[prefix] == b[prefix]) {
    prefix++;
  }
  var suffix = 0;
  while (suffix < a.length - prefix &&
      suffix < b.length - prefix &&
      a[a.length - 1 - suffix] == b[b.length - 1 - suffix]) {
    suffix++;
  }
  return (start: prefix, end: a.length - suffix);
}

/// What each side did to a clashing note and, unless one side deleted it,
/// how many places need a choice.
String conflictFileSubtitle(ConflictFile file) {
  final summary = switch (file.kind) {
    ConflictKind.deletedByThem =>
      'Remote deleted it · you changed '
          '${file.oursChanged} lines',
    ConflictKind.deletedByUs =>
      'You deleted it · remote changed '
          '${file.theirsChanged} lines',
    ConflictKind.bothAdded => 'Created on both sides',
    ConflictKind.bothModified =>
      'You: ${file.oursChanged} lines · '
          'Remote: ${file.theirsChanged} lines',
  };
  if (isWholeNote(file.kind)) return summary;
  final clashes = switch (file.conflicts) {
    0 => 'merges cleanly, just confirm',
    1 => '1 conflict',
    final n => '$n conflicts',
  };
  return '$summary\n$clashes';
}
