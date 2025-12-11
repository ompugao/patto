import 'dart:convert';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:path_provider/path_provider.dart';

import '../../../core/utils/secure_storage.dart';
import '../domain/git_config.dart';
import '../../../rust_bridge/api/git_api.dart' as bridge;

/// Git state
class GitState {
  final GitConfig? config;
  final bool isConfigured;
  final bool isCloned;
  final bool isLoading;
  final String? error;
  final GitProgress? progress;

  const GitState({
    this.config,
    this.isConfigured = false,
    this.isCloned = false,
    this.isLoading = false,
    this.error,
    this.progress,
  });

  GitState copyWith({
    GitConfig? config,
    bool? isConfigured,
    bool? isCloned,
    bool? isLoading,
    String? error,
    GitProgress? progress,
    bool clearError = false,
    bool clearProgress = false,
  }) {
    return GitState(
      config: config ?? this.config,
      isConfigured: isConfigured ?? this.isConfigured,
      isCloned: isCloned ?? this.isCloned,
      isLoading: isLoading ?? this.isLoading,
      error: clearError ? null : (error ?? this.error),
      progress: clearProgress ? null : (progress ?? this.progress),
    );
  }
}

/// Git provider
final gitProvider = StateNotifierProvider<GitNotifier, GitState>((ref) {
  final storage = ref.watch(secureStorageProvider);
  return GitNotifier(storage);
});

/// Git state notifier
class GitNotifier extends StateNotifier<GitState> {
  final FlutterSecureStorage _storage;
  late String _repoDir;

  GitNotifier(this._storage) : super(const GitState()) {
    _init();
  }

  Future<void> _init() async {
    final appDir = await getApplicationDocumentsDirectory();
    _repoDir = '${appDir.path}/patto-notes';
    await loadConfig();
  }

  String get repoDir => _repoDir;

  /// Load git configuration from secure storage
  Future<void> loadConfig() async {
    try {
      final configJson = await _storage.read(key: StorageKeys.gitConfig);

      if (configJson != null) {
        final config = GitConfig.fromJson(jsonDecode(configJson));
        final isCloned = await _checkIsCloned();

        state = state.copyWith(
          config: config,
          isConfigured: true,
          isCloned: isCloned,
        );
      }
    } catch (e) {
      state = state.copyWith(error: 'Failed to load config: $e');
    }
  }

  /// Save git configuration
  Future<void> saveConfig(GitConfig config) async {
    try {
      await _storage.write(
        key: StorageKeys.gitConfig,
        value: jsonEncode(config.toJson()),
      );

      state = state.copyWith(
        config: config,
        isConfigured: true,
        clearError: true,
      );
    } catch (e) {
      state = state.copyWith(error: 'Failed to save config: $e');
    }
  }

  /// Clone the repository
  Future<void> clone() async {
    if (state.config == null) {
      state = state.copyWith(error: 'No configuration set');
      return;
    }

    state = state.copyWith(isLoading: true, clearError: true);

    try {
      state = state.copyWith(
        progress: const GitProgress(phase: 'Cloning', current: 0, total: 1),
      );

      final dir = Directory(_repoDir);
      if (await dir.exists()) {
        await dir.delete(recursive: true);
      }

      final result = await bridge.cloneRepository(
        url: state.config!.repoUrl,
        path: _repoDir,
        branch: state.config!.branch,
        username: state.config!.username,
        password: state.config!.password,
      );

      if (!result.success) {
        state = state.copyWith(
          isLoading: false,
          error: result.error ?? result.message ?? 'Clone failed',
          clearProgress: true,
        );
        return;
      }

      state = state.copyWith(
        isLoading: false,
        isCloned: true,
        clearProgress: true,
      );
    } catch (e) {
      state = state.copyWith(
        isLoading: false,
        error: 'Clone failed: $e',
        clearProgress: true,
      );
    }
  }

  /// Pull latest changes
  Future<void> pull() async {
    if (!state.isCloned) {
      state = state.copyWith(error: 'Repository not cloned');
      return;
    }
    if (state.config == null) {
      state = state.copyWith(error: 'No configuration set');
      return;
    }

    state = state.copyWith(isLoading: true, clearError: true);

    try {
      state = state.copyWith(
        progress: const GitProgress(phase: 'Pulling', current: 0, total: 1),
      );

      final result = await bridge.pullRepository(
        path: _repoDir,
        branch: state.config!.branch,
        username: state.config!.username,
        password: state.config!.password,
      );

      if (!result.success) {
        state = state.copyWith(
          isLoading: false,
          error: result.error ?? result.message ?? 'Pull failed',
          clearProgress: true,
        );
        return;
      }

      state = state.copyWith(
        isLoading: false,
        clearProgress: true,
      );
    } catch (e) {
      state = state.copyWith(
        isLoading: false,
        error: 'Pull failed: $e',
        clearProgress: true,
      );
    }
  }

  /// Clear local repository data
  Future<void> clearLocalData() async {
    try {
      final result = await bridge.deleteRepository(path: _repoDir);
      if (!result.success) {
        final dir = Directory(_repoDir);
        if (await dir.exists()) {
          await dir.delete(recursive: true);
        }
      }

      state = state.copyWith(
          isCloned: false, clearError: true, clearProgress: true);
    } catch (e) {
      state = state.copyWith(error: 'Failed to clear data: $e');
    }
  }

  /// Clear error
  void clearError() {
    state = state.copyWith(clearError: true);
  }

  /// Check if repository is cloned
  Future<bool> _checkIsCloned() async {
    try {
      return bridge.isGitRepository(path: _repoDir);
    } catch (_) {
      return false;
    }
  }
}
