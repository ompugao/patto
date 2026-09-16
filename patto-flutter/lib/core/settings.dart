import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'workspace.dart';

/// Everything the user configures. Access tokens are the only secrets and are
/// kept out of shared preferences.
@immutable
class Settings {
  const Settings({
    this.workspaces = const [],
    this.activeWorkspaceId,
    this.authorName = '',
    this.authorEmail = '',
    this.themeMode = ThemeMode.system,
    this.fontScale = 1.0,
  });

  /// The smallest and largest note text the appearance setting offers.
  static const minFontScale = 0.8;
  static const maxFontScale = 1.8;

  final List<Workspace> workspaces;

  /// Null before the first workspace is added, or if the active one was
  /// deleted and none was chosen to replace it.
  final String? activeWorkspaceId;

  /// Commit identity, shared by every workspace.
  final String authorName;
  final String authorEmail;

  final ThemeMode themeMode;

  /// Multiplier for note text, applied in the note view and the editor.
  final double fontScale;

  Workspace? get active {
    for (final workspace in workspaces) {
      if (workspace.id == activeWorkspaceId) return workspace;
    }
    return workspaces.isEmpty ? null : workspaces.first;
  }

  Settings copyWith({
    List<Workspace>? workspaces,
    String? activeWorkspaceId,
    bool clearActiveWorkspace = false,
    String? authorName,
    String? authorEmail,
    ThemeMode? themeMode,
    double? fontScale,
  }) {
    return Settings(
      workspaces: workspaces ?? this.workspaces,
      activeWorkspaceId: clearActiveWorkspace
          ? null
          : (activeWorkspaceId ?? this.activeWorkspaceId),
      authorName: authorName ?? this.authorName,
      authorEmail: authorEmail ?? this.authorEmail,
      themeMode: themeMode ?? this.themeMode,
      fontScale: fontScale ?? this.fontScale,
    );
  }

  /// Add a workspace, or replace the one with the same id.
  Settings withWorkspace(Workspace workspace) {
    final next = [...workspaces];
    final at = next.indexWhere((w) => w.id == workspace.id);
    if (at >= 0) {
      next[at] = workspace;
    } else {
      next.add(workspace);
    }
    return copyWith(
      workspaces: next,
      activeWorkspaceId: activeWorkspaceId ?? workspace.id,
    );
  }

  Settings withoutWorkspace(String id) {
    final next = workspaces.where((w) => w.id != id).toList();
    if (activeWorkspaceId != id) {
      return copyWith(workspaces: next);
    }
    return Settings(
      workspaces: next,
      activeWorkspaceId: next.isEmpty ? null : next.first.id,
      authorName: authorName,
      authorEmail: authorEmail,
      themeMode: themeMode,
      fontScale: fontScale,
    );
  }
}

class SettingsStore {
  static const _secure = FlutterSecureStorage();

  static String _tokenKey(String workspaceId) => 'patto.git.token.$workspaceId';

  /// The key used before workspaces existed.
  static const _legacyTokenKey = 'patto.git.token';

  Future<Settings> load() async {
    final prefs = await SharedPreferences.getInstance();

    var workspaces = _readWorkspaces(prefs);
    var activeId = prefs.getString('activeWorkspaceId');

    if (workspaces.isEmpty) {
      final migrated = await _migrateSingleWorkspace(prefs);
      if (migrated != null) {
        workspaces = [migrated];
        activeId = migrated.id;
      }
    }

    workspaces = [
      for (final workspace in workspaces)
        workspace.copyWith(token: await _readToken(workspace.id)),
    ];

    return Settings(
      workspaces: workspaces,
      activeWorkspaceId: activeId,
      authorName: prefs.getString('authorName') ?? '',
      authorEmail: prefs.getString('authorEmail') ?? '',
      themeMode: ThemeMode.values.byName(
        prefs.getString('themeMode') ?? ThemeMode.system.name,
      ),
      fontScale: (prefs.getDouble('fontScale') ?? 1.0).clamp(
        Settings.minFontScale,
        Settings.maxFontScale,
      ),
    );
  }

  Future<void> save(Settings settings) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(
      'workspaces',
      jsonEncode([for (final w in settings.workspaces) w.toJson()]),
    );
    if (settings.activeWorkspaceId == null) {
      await prefs.remove('activeWorkspaceId');
    } else {
      await prefs.setString('activeWorkspaceId', settings.activeWorkspaceId!);
    }
    await prefs.setString('authorName', settings.authorName);
    await prefs.setString('authorEmail', settings.authorEmail);
    await prefs.setString('themeMode', settings.themeMode.name);
    await prefs.setDouble('fontScale', settings.fontScale);

    for (final workspace in settings.workspaces) {
      await _writeToken(workspace.id, workspace.token);
    }
  }

  /// Forget a workspace's token. The caller removes the workspace itself.
  Future<void> forgetToken(String workspaceId) async {
    try {
      await _secure.delete(key: _tokenKey(workspaceId));
    } on Exception {
      // Nothing to do: a token we cannot delete is one the user can overwrite.
    }
  }

  List<Workspace> _readWorkspaces(SharedPreferences prefs) {
    final raw = prefs.getString('workspaces');
    if (raw == null || raw.isEmpty) return const [];
    try {
      final decoded = jsonDecode(raw) as List<dynamic>;
      return [
        for (final entry in decoded)
          Workspace.fromJson(entry as Map<String, dynamic>),
      ];
    } on Exception {
      // Corrupt preferences must not stop the app from starting.
      return const [];
    }
  }

  /// Turn the settings written by the single-workspace version into the first
  /// workspace, keeping the clone already on disk.
  ///
  /// The new shape is written before the old keys are dropped: this runs once,
  /// and an install that lost both would look like a first run with the notes
  /// still sitting on disk.
  Future<Workspace?> _migrateSingleWorkspace(SharedPreferences prefs) async {
    final repoUrl = prefs.getString('repoUrl');
    if (repoUrl == null) return null;

    final workspace = Workspace(
      id: 'default',
      name: Workspace.nameFromUrl(repoUrl),
      dirName: WorkspaceStorage.legacyDirName,
      repoUrl: repoUrl,
      branch: prefs.getString('branch') ?? '',
      username: prefs.getString('username') ?? '',
    );

    await prefs.setString('workspaces', jsonEncode([workspace.toJson()]));
    await prefs.setString('activeWorkspaceId', workspace.id);

    try {
      final token = await _secure.read(key: _legacyTokenKey);
      if (token != null && token.isNotEmpty) {
        await _secure.write(key: _tokenKey(workspace.id), value: token);
        await _secure.delete(key: _legacyTokenKey);
      }
    } on Exception {
      // The user can re-enter the token; losing it must not block the upgrade.
    }

    for (final key in ['repoUrl', 'branch', 'username']) {
      await prefs.remove(key);
    }
    return workspace;
  }

  Future<String> _readToken(String workspaceId) async {
    try {
      return await _secure.read(key: _tokenKey(workspaceId)) ?? '';
    } on Exception {
      // A corrupt or unreadable keystore must not block startup; the user can
      // re-enter the token in settings.
      return '';
    }
  }

  Future<void> _writeToken(String workspaceId, String token) async {
    try {
      if (token.isEmpty) {
        await _secure.delete(key: _tokenKey(workspaceId));
      } else {
        await _secure.write(key: _tokenKey(workspaceId), value: token);
      }
    } on Exception {
      // Saving the token can fail on devices with a broken keystore; the rest
      // of the settings are still worth keeping.
    }
  }
}
