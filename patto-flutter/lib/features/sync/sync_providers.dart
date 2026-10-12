import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers.dart';
import '../../src/rust/api/git.dart';
import '../../src/rust/frb_api.dart' as rust;

final gitStatusProvider = FutureProvider.autoDispose<GitStatus?>((ref) async {
  final workspace = await ref.watch(workspaceProvider.future);
  ref.watch(notesRevisionProvider);
  if (workspace == null || !workspace.isCloned) return null;

  try {
    return await rust.gitStatus(
      root: workspace.root,
      attachmentsDir: workspace.config.attachmentsDir,
    );
  } catch (_) {
    return null;
  }
});
