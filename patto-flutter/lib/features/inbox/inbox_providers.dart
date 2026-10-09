import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers.dart';
import '../../core/settings.dart';
import '../../src/rust/api/types.dart';
import '../../src/rust/frb_api.dart' as rust;

/// Text waiting to be put into the inbox composer, from a share or the
/// launcher shortcut. The composer takes it and sets this back to null.
final inboxDraftProvider = valueProvider<String?>(null);

final inboxNoteNameProvider = Provider<String>((ref) {
  return ref.watch(settingsProvider).value?.inboxNoteName ??
      Settings.defaultInboxNoteName;
});

final inboxPostsProvider = FutureProvider<List<InboxPost>>((ref) async {
  final workspace = await ref.watch(workspaceProvider.future);
  ref.watch(notesRevisionProvider);
  final name = ref.watch(inboxNoteNameProvider);
  if (workspace == null) return const [];
  return rust.inboxPosts(root: workspace.root, name: name);
});
