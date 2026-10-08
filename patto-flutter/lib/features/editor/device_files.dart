import 'dart:io';

import 'package:flutter/services.dart';

/// An image found on the clipboard.
class ClipboardImage {
  const ClipboardImage({required this.bytes, this.mimeType});

  final Uint8List bytes;
  final String? mimeType;
}

/// A file the user picked, copied by the host into the app's cache.
class PickedFile {
  const PickedFile({required this.file, required this.name});

  final File file;
  final String name;
}

/// What the Android host does for the editor that Flutter cannot: read an
/// image off the clipboard, and open the system file dialog without a plugin
/// (the usual one pulls in a native module that wants another NDK).
///
/// Where the host lacks the channel, or fails, each call answers as if there
/// were nothing: no image, no file.
class DeviceFiles {
  DeviceFiles._();

  static const _channel = MethodChannel('com.sifi.patto/device_files');

  static Future<ClipboardImage?> clipboardImage() async {
    final reply = await _call('readClipboardImage');
    final bytes = reply?['bytes'];
    if (bytes is! Uint8List || bytes.isEmpty) return null;
    return ClipboardImage(
      bytes: bytes,
      mimeType: reply?['mimeType'] as String?,
    );
  }

  static Future<PickedFile?> pickFile() async {
    final reply = await _call('pickFile');
    final path = reply?['path'];
    final name = reply?['name'];
    if (path is! String || name is! String) return null;
    return PickedFile(file: File(path), name: name);
  }

  static Future<Map<String, Object?>?> _call(String method) async {
    try {
      return await _channel.invokeMapMethod<String, Object?>(method);
    } on MissingPluginException {
      return null;
    } on PlatformException {
      return null;
    }
  }
}
