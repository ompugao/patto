import '../../core/settings.dart';

/// Why [name] cannot be the inbox note, or null when it can. An empty name
/// is fine: it means the default. [coreAccepts] is the Rust core's note-name
/// rule, which stays the authority where it can be called.
String? inboxNoteNameError(
  String name, {
  required bool Function(String name) coreAccepts,
}) {
  final trimmed = name.trim();
  if (trimmed.isEmpty) return null;
  if (trimmed.contains('#')) return 'A note name cannot contain #';
  if (!Settings.isValidInboxNoteName(trimmed)) return 'Not a valid note name';
  if (!coreAccepts(trimmed)) return 'Not a valid note name';
  return null;
}
