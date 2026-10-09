import 'package:intl/intl.dart';

/// Turn what another app shared into the text the composer starts with. A
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

/// The composer text once [draft] arrives: on its own line after whatever
/// was already typed. An empty draft, the launcher shortcut, changes nothing.
String appendDraft(String current, String draft) => draft.isEmpty
    ? current
    : current.isEmpty
    ? draft
    : '$current\n$draft';

/// How a `yyyy-MM-dd` post date is headed, relative to [now].
String inboxDateLabel(String date, DateTime now) {
  final day = DateTime.tryParse(date);
  if (day == null) return date;
  // Calendar days are compared field by field: a Duration across a DST
  // change is not a whole number of days.
  final yesterday = DateTime(now.year, now.month, now.day - 1);
  if (_sameDay(day, now)) return 'Today';
  if (_sameDay(day, yesterday)) return 'Yesterday';
  return DateFormat('EEE, d MMM yyyy').format(day);
}

bool _sameDay(DateTime a, DateTime b) =>
    a.year == b.year && a.month == b.month && a.day == b.day;
