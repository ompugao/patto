import 'package:flutter/material.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// Everything the user configures. The access token is the only secret and is
/// kept out of shared preferences.
@immutable
class Settings {
  const Settings({
    this.repoUrl = '',
    this.branch = '',
    this.username = '',
    this.token = '',
    this.authorName = '',
    this.authorEmail = '',
    this.themeMode = ThemeMode.system,
    this.fontScale = 1.0,
  });

  /// The smallest and largest note text the appearance setting offers.
  static const minFontScale = 0.8;
  static const maxFontScale = 1.8;

  final String repoUrl;

  /// Empty means "whatever the remote's default branch is".
  final String branch;
  final String username;
  final String token;
  final String authorName;
  final String authorEmail;
  final ThemeMode themeMode;

  /// Multiplier for note text, applied in the note view and the editor.
  final double fontScale;

  bool get hasRemote => repoUrl.trim().isNotEmpty;

  Settings copyWith({
    String? repoUrl,
    String? branch,
    String? username,
    String? token,
    String? authorName,
    String? authorEmail,
    ThemeMode? themeMode,
    double? fontScale,
  }) {
    return Settings(
      repoUrl: repoUrl ?? this.repoUrl,
      branch: branch ?? this.branch,
      username: username ?? this.username,
      token: token ?? this.token,
      authorName: authorName ?? this.authorName,
      authorEmail: authorEmail ?? this.authorEmail,
      themeMode: themeMode ?? this.themeMode,
      fontScale: fontScale ?? this.fontScale,
    );
  }
}

class SettingsStore {
  static const _tokenKey = 'patto.git.token';
  static const _secure = FlutterSecureStorage();

  Future<Settings> load() async {
    final prefs = await SharedPreferences.getInstance();
    String token = '';
    try {
      token = await _secure.read(key: _tokenKey) ?? '';
    } catch (_) {
      // A corrupt or unreadable keystore must not block startup; the user can
      // re-enter the token in settings.
      token = '';
    }

    return Settings(
      repoUrl: prefs.getString('repoUrl') ?? '',
      branch: prefs.getString('branch') ?? '',
      username: prefs.getString('username') ?? '',
      token: token,
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
    await prefs.setString('repoUrl', settings.repoUrl);
    await prefs.setString('branch', settings.branch);
    await prefs.setString('username', settings.username);
    await prefs.setString('authorName', settings.authorName);
    await prefs.setString('authorEmail', settings.authorEmail);
    await prefs.setString('themeMode', settings.themeMode.name);
    await prefs.setDouble('fontScale', settings.fontScale);

    try {
      if (settings.token.isEmpty) {
        await _secure.delete(key: _tokenKey);
      } else {
        await _secure.write(key: _tokenKey, value: settings.token);
      }
    } catch (_) {
      // Saving the token can fail on devices with a broken keystore; the rest
      // of the settings are still worth keeping.
    }
  }
}
