import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';

/// One notes repository the app knows about.
///
/// The token is held here while the app runs but is stored separately, in the
/// device keystore, keyed by [id].
@immutable
class Workspace {
  const Workspace({
    required this.id,
    required this.name,
    required this.dirName,
    this.repoUrl = '',
    this.branch = '',
    this.username = '',
    this.token = '',
  });

  /// Stable identifier. Also the keystore key for the token, so it must not
  /// change once a workspace exists.
  final String id;

  /// What the user calls this workspace.
  final String name;

  /// Folder under the app's workspaces directory. Kept separate from [id] so an
  /// install upgraded from the single-workspace version keeps the clone it has.
  final String dirName;

  final String repoUrl;

  /// Empty means whatever the remote's default branch is.
  final String branch;
  final String username;
  final String token;

  bool get hasRemote => repoUrl.trim().isNotEmpty;

  Workspace copyWith({
    String? name,
    String? repoUrl,
    String? branch,
    String? username,
    String? token,
  }) {
    return Workspace(
      id: id,
      dirName: dirName,
      name: name ?? this.name,
      repoUrl: repoUrl ?? this.repoUrl,
      branch: branch ?? this.branch,
      username: username ?? this.username,
      token: token ?? this.token,
    );
  }

  /// Everything except the token, which lives in the keystore.
  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'dirName': dirName,
    'repoUrl': repoUrl,
    'branch': branch,
    'username': username,
  };

  static Workspace fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String;
    return Workspace(
      id: id,
      name: json['name'] as String? ?? id,
      dirName: json['dirName'] as String? ?? id,
      repoUrl: json['repoUrl'] as String? ?? '',
      branch: json['branch'] as String? ?? '',
      username: json['username'] as String? ?? '',
    );
  }

  /// A name for a workspace cloned from [repoUrl], e.g. `you/notes.git` becomes
  /// `notes`.
  static String nameFromUrl(String repoUrl) {
    final trimmed = repoUrl.trim().replaceAll(RegExp(r'[/\s]+$'), '');
    if (trimmed.isEmpty) return 'Notes';
    final last = trimmed.split('/').last;
    final withoutSuffix = last.endsWith('.git')
        ? last.substring(0, last.length - 4)
        : last;
    return withoutSuffix.isEmpty ? 'Notes' : withoutSuffix;
  }

  /// Identifiers are generated, never typed, so a simple timestamp suffices and
  /// keeps the folder name readable.
  static String newId() =>
      'ws-${DateTime.now().microsecondsSinceEpoch.toRadixString(36)}';
}

/// A workspace resolved to a place on disk.
@immutable
class ActiveWorkspace {
  const ActiveWorkspace({required this.config, required this.root});

  final Workspace config;

  /// Absolute path of the notes directory.
  final String root;

  bool get exists => Directory(root).existsSync();

  bool get isCloned => Directory('$root/.git').existsSync();

  Future<void> clear() async {
    final dir = Directory(root);
    if (await dir.exists()) {
      await dir.delete(recursive: true);
    }
  }

  /// Absolute path for a note path relative to the root.
  String absolute(String relPath) => '$root/$relPath';
}

/// Where workspaces live on the device.
class WorkspaceStorage {
  /// The folder an upgraded install already cloned into.
  static const legacyDirName = 'notes';

  static Future<String> baseDir() async {
    final dir = await getApplicationSupportDirectory();
    return dir.path;
  }

  static String rootFor(String baseDir, Workspace workspace) =>
      '$baseDir/${workspace.dirName}';
}
