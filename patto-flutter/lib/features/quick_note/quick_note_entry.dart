import 'package:intl/intl.dart';

import '../../core/settings.dart';

/// The note a quick note is appended to, under the given settings.
///
/// `DateFormat` never throws: an unmatched quote yields nothing and unknown
/// letters are substituted, so a pattern that gives no usable name falls back
/// to the default one.
String quickNoteTargetName(Settings settings, DateTime now) {
  switch (settings.quickNoteTarget) {
    case QuickNoteTarget.daily:
      final name = dailyNoteName(settings.quickNoteDateFormat, now);
      return isValidQuickNoteName(name)
          ? name
          : dailyNoteName(Settings.defaultQuickNoteDateFormat, now);
    case QuickNoteTarget.single:
      return isValidQuickNoteName(settings.quickNoteName)
          ? settings.quickNoteName
          : Settings.defaultQuickNoteName;
  }
}

String dailyNoteName(String pattern, DateTime now) =>
    DateFormat(pattern).format(now);

/// `#` separates a note from an anchor in `[note#anchor]`, so a name holding
/// one could never be linked to.
bool isValidQuickNoteName(String name) =>
    name.trim().isNotEmpty && !name.contains('#');

/// Lay the captured text out as one outline item: the first line at the top
/// level, optionally stamped with the time, and every further line nested one
/// level under it. Blank lines are dropped. Empty text gives an empty string.
String formatQuickNoteEntry(
  String text, {
  required bool timePrefix,
  required DateTime now,
}) {
  final lines = text
      .replaceAll('\r\n', '\n')
      .split('\n')
      .map((line) => line.trimRight())
      .where((line) => line.trim().isNotEmpty)
      .toList();
  if (lines.isEmpty) return '';

  final buffer = StringBuffer();
  if (timePrefix) {
    buffer.write(DateFormat('HH:mm').format(now));
    buffer.write(' ');
  }
  buffer.write(lines.first.trimLeft());
  for (final line in lines.skip(1)) {
    buffer.write('\n\t');
    buffer.write(line);
  }
  return buffer.toString();
}

/// Turn what another app shared into the text the quick note starts with. A
/// shared link with a subject becomes a patto link, `[url title]`.
String draftFromShared({required String text, String? subject}) {
  final trimmedText = text.trim();
  final trimmedSubject = subject?.trim() ?? '';
  if (trimmedSubject.isEmpty || trimmedSubject == trimmedText) {
    return trimmedText;
  }
  if (_isUrl(trimmedText)) {
    final title = trimmedSubject.replaceAll('[', '(').replaceAll(']', ')');
    return '[$trimmedText $title]';
  }
  return '$trimmedSubject\n$trimmedText';
}

bool _isUrl(String text) {
  if (text.contains(RegExp(r'\s'))) return false;
  final uri = Uri.tryParse(text);
  return uri != null &&
      (uri.scheme == 'http' || uri.scheme == 'https') &&
      uri.host.isNotEmpty;
}
