import 'package:intl/intl.dart';

import '../../core/settings.dart';

/// The note a quick note is appended to, under the given settings.
String quickNoteTargetName(Settings settings, DateTime now) {
  switch (settings.quickNoteTarget) {
    case QuickNoteTarget.daily:
      try {
        return DateFormat(settings.quickNoteDateFormat).format(now);
      } on FormatException {
        return DateFormat(Settings.defaultQuickNoteDateFormat).format(now);
      }
    case QuickNoteTarget.single:
      return settings.quickNoteName;
  }
}

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
    return '[$trimmedText $trimmedSubject]';
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
