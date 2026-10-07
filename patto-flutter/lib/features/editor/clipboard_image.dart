import 'package:flutter/services.dart';

/// An image found on the clipboard.
class ClipboardImage {
  const ClipboardImage({required this.bytes, this.mimeType});

  final Uint8List bytes;
  final String? mimeType;

  static const _channel = MethodChannel('com.sifi.patto/clipboard');

  /// The image on the clipboard, which Flutter's own clipboard API cannot
  /// read. Backed by a small platform channel in the Android host; where the
  /// host lacks it, or fails, the clipboard is treated as holding no image.
  static Future<ClipboardImage?> read() async {
    try {
      final reply = await _channel.invokeMapMethod<String, Object?>(
        'readImage',
      );
      final bytes = reply?['bytes'];
      if (bytes is! Uint8List || bytes.isEmpty) return null;
      return ClipboardImage(
        bytes: bytes,
        mimeType: reply?['mimeType'] as String?,
      );
    } on MissingPluginException {
      return null;
    } on PlatformException {
      return null;
    }
  }
}
