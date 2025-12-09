import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Secure storage provider
final secureStorageProvider = Provider<FlutterSecureStorage>((ref) {
  return const FlutterSecureStorage(
    aOptions: AndroidOptions(
      encryptedSharedPreferences: true,
    ),
  );
});

/// Keys for secure storage
class StorageKeys {
  StorageKeys._();

  static const gitConfig = 'patto_git_config';
  static const themeMode = 'patto_theme_mode';
}
