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
