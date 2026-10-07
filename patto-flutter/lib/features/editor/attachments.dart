import 'dart:io';
import 'dart:typed_data';

import '../../core/workspace.dart';

final _dirSegment = RegExp(
  r'^[A-Za-z0-9_\-.ー\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}'
  r'\p{Script=Hangul}\p{Script=Bopomofo}]+$',
  unicode: true,
);

/// The attachment folder as typed, reduced to a relative path the grammar can
/// spell inside `./...`; empty means the default, and null that it cannot be
/// used.
String? normalizeAttachmentsDir(String text) {
  final trimmed = text.trim().replaceAll(RegExp(r'^/+|/+$'), '');
  if (trimmed.isEmpty) return defaultAttachmentsDir;
  final segments = trimmed.split('/');
  for (final segment in segments) {
    if (segment == '.' || segment == '..' || !_dirSegment.hasMatch(segment)) {
      return null;
    }
  }
  return segments.join('/');
}

const _imageExtensions = {'png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg'};

const _mimeExtensions = {
  'image/png': 'png',
  'image/jpeg': 'jpg',
  'image/gif': 'gif',
  'image/webp': 'webp',
  'image/bmp': 'bmp',
  'image/svg+xml': 'svg',
  'image/heic': 'heic',
  'image/heif': 'heif',
  'image/avif': 'avif',
  'image/tiff': 'tiff',
};

/// Everything the grammar's `local_file` rule refuses in a path segment. Its
/// CJK classes are Unicode scripts, so `・` or `゛` (script Common) must go
/// even though they sit between the kana.
final _unsafeChar = RegExp(
  r'[^A-Za-z0-9_\-ー\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}'
  r'\p{Script=Hangul}\p{Script=Bopomofo}]',
  unicode: true,
);

/// A file name the patto grammar accepts inside `./...`: ASCII letters,
/// digits, `_`, `-` or CJK in the stem, and an extension of letters only,
/// which is why `mp4` loses its digit and a name without one gets `.bin`.
String sanitizeAttachmentName(String name) {
  final dot = name.lastIndexOf('.');
  var stem = dot > 0 ? name.substring(0, dot) : name;
  var ext = dot > 0 ? name.substring(dot + 1) : '';
  stem = stem.replaceAll(_unsafeChar, '_').replaceAll(RegExp('_+'), '_');
  stem = stem.replaceAll(RegExp(r'^_+|_+$'), '');
  if (stem.isEmpty) stem = 'file';
  ext = ext.toLowerCase().replaceAll(RegExp('[^a-z]'), '');
  if (ext.isEmpty) ext = 'bin';
  return '$stem.$ext';
}

String _stamp(DateTime now) {
  String two(int n) => n.toString().padLeft(2, '0');
  return '${now.year}${two(now.month)}${two(now.day)}-'
      '${two(now.hour)}${two(now.minute)}${two(now.second)}';
}

/// Name for an image that arrived without one, such as a pasted screenshot.
/// The format is read off the bytes, then off [mimeType], then taken as PNG.
String pastedImageName(
  String noteRelPath,
  Uint8List bytes,
  DateTime now, {
  String? mimeType,
}) {
  final file = noteRelPath.split('/').last;
  final stem = file.endsWith('.pn') ? file.substring(0, file.length - 3) : file;
  final ext =
      imageExtensionOf(bytes) ??
      _mimeExtensions[mimeType?.toLowerCase()] ??
      'png';
  return sanitizeAttachmentName('$stem-${_stamp(now)}.$ext');
}

/// The image format by its leading bytes, or null for anything else.
String? imageExtensionOf(Uint8List bytes) {
  bool starts(List<int> magic, [int at = 0]) {
    if (bytes.length < at + magic.length) return false;
    for (var i = 0; i < magic.length; i++) {
      if (bytes[at + i] != magic[i]) return false;
    }
    return true;
  }

  if (starts(const [0x89, 0x50, 0x4e, 0x47])) return 'png';
  if (starts(const [0xff, 0xd8, 0xff])) return 'jpg';
  if (starts(const [0x47, 0x49, 0x46, 0x38])) return 'gif';
  if (starts(const [0x52, 0x49, 0x46, 0x46]) &&
      starts(const [0x57, 0x45, 0x42, 0x50], 8)) {
    return 'webp';
  }
  if (starts(const [0x42, 0x4d])) return 'bmp';
  return null;
}

String extensionOf(String path) {
  final file = path.split('/').last;
  final dot = file.lastIndexOf('.');
  return dot > 0 ? file.substring(dot + 1).toLowerCase() : '';
}

bool isImageName(String path) => _imageExtensions.contains(extensionOf(path));

/// Writes [bytes] under the attachments folder and returns the path relative
/// to [root]; an existing file of that name is kept and the new one numbered.
Future<String> saveAttachment(
  String root,
  String attachmentsDir,
  String name,
  Uint8List bytes,
) async {
  final dir = Directory('$root/$attachmentsDir');
  await dir.create(recursive: true);
  final relPath = await _freePath(
    dir,
    attachmentsDir,
    sanitizeAttachmentName(name),
  );
  await File('$root/$relPath').writeAsBytes(bytes, flush: true);
  return relPath;
}

Future<String> _freePath(
  Directory dir,
  String attachmentsDir,
  String name,
) async {
  final dot = name.lastIndexOf('.');
  final stem = dot > 0 ? name.substring(0, dot) : name;
  final ext = dot > 0 ? name.substring(dot) : '';
  var candidate = name;
  for (var n = 2; await File('${dir.path}/$candidate').exists(); n++) {
    candidate = '$stem-$n$ext';
  }
  return '$attachmentsDir/$candidate';
}

/// The note text that shows an attachment at [relPath]: an image, an embedded
/// PDF, or a link to any other file.
String attachmentSnippet(String relPath) {
  final ext = extensionOf(relPath);
  if (_imageExtensions.contains(ext)) return '[@img ./$relPath]';
  if (ext == 'pdf') return '[@embed ./$relPath ${_titleOf(relPath)}]';
  return '[./$relPath]';
}

String _titleOf(String relPath) {
  final file = relPath.split('/').last;
  final dot = file.lastIndexOf('.');
  return (dot > 0 ? file.substring(0, dot) : file).replaceAll('_', ' ');
}

final _httpUrl = RegExp(r'^https?://\S+$');

/// Whether the clipboard text is a single web address and nothing else.
bool isWebUrl(String text) => _httpUrl.hasMatch(text.trim());

/// The characters the grammar's `URL` rule reads after the scheme; anything
/// else, such as `[`, `|` or a space, would end the address early.
final _urlChar = RegExp(
  r"[A-Za-z0-9/:#%$&?@!()~.=+*\-_,';ー\p{Script=Han}\p{Script=Hiragana}"
  r'\p{Script=Katakana}\p{Script=Hangul}\p{Script=Bopomofo}]',
  unicode: true,
);

/// [url] with every character the grammar would choke on percent-encoded.
String noteSafeUrl(String url) {
  final out = StringBuffer();
  for (final rune in url.runes) {
    final ch = String.fromCharCode(rune);
    out.write(_urlChar.hasMatch(ch) ? ch : Uri.encodeComponent(ch));
  }
  return out.toString();
}

final _embedHosts = RegExp(
  r'^(www\.|m\.|mobile\.)?('
  r'youtube\.com|youtu\.be|twitter\.com|x\.com|speakerdeck\.com|'
  r'slideshare\.net|photos\.app\.goo\.gl|photos\.google\.com)$',
);

final _urlInTitle = RegExp(r'[A-Za-z][A-Za-z0-9+.\-]*://\S*');

/// The note text for a pasted address: an embed for the sites the viewer can
/// show inline, an image for one that points at a picture, and otherwise a
/// link carrying the page [title] when one was found.
String urlSnippet(String url, {String? title}) {
  final uri = Uri.tryParse(url);
  final path = uri?.path ?? '';
  final safe = noteSafeUrl(url);
  if (uri != null && _embedHosts.hasMatch(uri.host)) return '[@embed $safe]';
  if (path.toLowerCase().endsWith('.pdf')) return '[@embed $safe]';
  if (isImageName(path)) return '[@img $safe]';
  // A title may not hold brackets or an address, which the grammar would
  // read as the link's end or a second link.
  final cleaned = title
      ?.replaceAll(_urlInTitle, ' ')
      .replaceAll(RegExp(r'[\[\]\s]+'), ' ')
      .trim();
  if (cleaned == null || cleaned.isEmpty) return '[$safe]';
  return '[$safe $cleaned]';
}
