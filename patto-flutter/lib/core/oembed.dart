/// Pure helpers for the small amount of HTML that oEmbed responses carry.
library;

final _tag = RegExp(r'<[^>]+>');
final _lineBreak = RegExp(r'<br\s*/?>', caseSensitive: false);
final _paragraph = RegExp(
  r'<p\b[^>]*>(.*?)</p>',
  caseSensitive: false,
  dotAll: true,
);
final _iframeSrc = RegExp(
  r'''<iframe\b[^>]*\bsrc\s*=\s*["']([^"']+)["']''',
  caseSensitive: false,
);
final _entity = RegExp(r'&(#x[0-9a-fA-F]+|#[0-9]+|[a-zA-Z]+);');
final _blankRuns = RegExp(r'[ \t]+');
final _blankLines = RegExp(r'\n\s*\n+');

const _namedEntities = {
  'amp': '&',
  'lt': '<',
  'gt': '>',
  'quot': '"',
  'apos': "'",
  'nbsp': ' ',
  'mdash': '—',
  'ndash': '–',
  'hellip': '…',
  'lsquo': '‘',
  'rsquo': '’',
  'ldquo': '“',
  'rdquo': '”',
};

String decodeEntities(String text) {
  return text.replaceAllMapped(_entity, (m) {
    final body = m[1]!;
    if (body.startsWith('#x') || body.startsWith('#X')) {
      final code = int.tryParse(body.substring(2), radix: 16);
      return code == null ? m[0]! : String.fromCharCode(code);
    }
    if (body.startsWith('#')) {
      final code = int.tryParse(body.substring(1));
      return code == null ? m[0]! : String.fromCharCode(code);
    }
    return _namedEntities[body] ?? m[0]!;
  });
}

/// The readable text of an HTML fragment: tags dropped, `<br>` kept as a line
/// break, entities decoded and runs of blanks collapsed.
String stripHtml(String html) {
  final text = decodeEntities(
    html.replaceAll(_lineBreak, '\n').replaceAll(_tag, ''),
  );
  return text
      .replaceAll(_blankRuns, ' ')
      .replaceAll(_blankLines, '\n')
      .split('\n')
      .map((line) => line.trim())
      .join('\n')
      .trim();
}

/// The tweet itself out of Twitter's oEmbed `html`, which wraps it in a
/// `<p>` followed by the author and date.
String tweetTextFromHtml(String html) {
  final paragraph = _paragraph.firstMatch(html);
  return stripHtml(paragraph?.group(1) ?? html);
}

/// The `src` of the first `<iframe>` in an oEmbed `html` snippet.
String? iframeSrc(String html) {
  final src = _iframeSrc.firstMatch(html)?.group(1);
  if (src == null) return null;
  return decodeEntities(src.startsWith('//') ? 'https:$src' : src);
}
