import 'dart:io';

import 'package:path_provider/path_provider.dart';

/// Where the notes clone lives on the device, and whether it is there yet.
class Workspace {
  Workspace(this.root);

  /// Absolute path of the notes directory.
  final String root;

  static Future<Workspace> resolve() async {
    final dir = await getApplicationSupportDirectory();
    return Workspace('${dir.path}/notes');
  }

  bool get isCloned => Directory('$root/.git').existsSync();

  bool get exists => Directory(root).existsSync();

  Future<void> clear() async {
    final dir = Directory(root);
    if (await dir.exists()) {
      await dir.delete(recursive: true);
    }
  }

  /// Absolute path for a note path relative to the root.
  String absolute(String relPath) => '$root/$relPath';
}
