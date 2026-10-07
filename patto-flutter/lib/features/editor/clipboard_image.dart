import 'package:flutter/services.dart';

/// The image on the clipboard, which Flutter's own clipboard API cannot read.
///
/// Backed by a small platform channel in the Android host; where the host
/// does not implement it the clipboard is treated as holding no image.
class ClipboardImage {
  ClipboardImage._();

  static const _channel = MethodChannel('com.sifi.patto/clipboard');

  static Future<Uint8List?> read() async {
    try {
      return await _channel.invokeMethod<Uint8List>('readImage');
    } on MissingPluginException {
      return null;
    }
  }
}
