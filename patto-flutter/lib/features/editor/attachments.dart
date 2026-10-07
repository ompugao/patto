import 'dart:io';
import 'dart:typed_data';

/// Files inserted into a note live here, under the workspace root, so a
/// `./attachments/...` path resolves the same from every note.
const attachmentsDir = 'attachments';

const _imageExtensions = {'png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg'};

final _unsafeChar = RegExp(
  r'[^A-Za-z0-9_\-'
  r'々぀-ヿ㐀-䶿一-鿿가-힯豈-﫿]',
);

/// A file name the patto grammar accepts inside `./...`: ASCII letters,
/// digits, `_`, `-` or CJK in the stem, letters in the extension.
String sanitizeAttachmentName(String name) {
  final dot = name.lastIndexOf('.');
  var stem = dot > 0 ? name.substring(0, dot) : name;
  var ext = dot > 0 ? name.substring(dot + 1) : '';
  stem = stem.replaceAll(_unsafeChar, '_').replaceAll(RegExp('_+'), '_');
  stem = stem.replaceAll(RegExp(r'^_+|_+$'), '');
  if (stem.isEmpty) stem = 'file';
  ext = ext.toLowerCase().replaceAll(RegExp('[^a-z]'), '');
  return ext.isEmpty ? stem : '$stem.$ext';
}

String _stamp(DateTime now) {
  String two(int n) => n.toString().padLeft(2, '0');
  return '${now.year}${two(now.month)}${two(now.day)}-'
      '${two(now.hour)}${two(now.minute)}${two(now.second)}';
}

/// Name for an image that arrived without one, such as a pasted screenshot.
String pastedImageName(String noteRelPath, Uint8List bytes, DateTime now) {
  final file = noteRelPath.split('/').last;
  final stem = file.endsWith('.pn') ? file.substring(0, file.length - 3) : file;
  final ext = imageExtensionOf(bytes) ?? 'png';
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
Future<String> saveAttachment(String root, String name, Uint8List bytes) async {
  final dir = Directory('$root/$attachmentsDir');
  await dir.create(recursive: true);
  final relPath = await _freePath(dir, sanitizeAttachmentName(name));
  await File('$root/$relPath').writeAsBytes(bytes, flush: true);
  return relPath;
}

Future<String> _freePath(Directory dir, String name) async {
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

final _embedHosts = RegExp(
  r'^(www\.|m\.|mobile\.)?('
  r'youtube\.com|youtu\.be|twitter\.com|x\.com|speakerdeck\.com|'
  r'slideshare\.net|photos\.app\.goo\.gl|photos\.google\.com)$',
);

/// The note text for a pasted address: an embed for the sites the viewer can
/// show inline, an image for one that points at a picture, and otherwise a
/// link carrying the page [title] when one was found.
String urlSnippet(String url, {String? title}) {
  final uri = Uri.tryParse(url);
  final path = uri?.path ?? '';
  if (uri != null && _embedHosts.hasMatch(uri.host)) return '[@embed $url]';
  if (path.toLowerCase().endsWith('.pdf')) return '[@embed $url]';
  if (isImageName(path)) return '[@img $url]';
  final cleaned = title?.replaceAll(RegExp(r'[\[\]\s]+'), ' ').trim();
  if (cleaned == null || cleaned.isEmpty) return '[$url]';
  return '[$url $cleaned]';
}
