import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../../core/providers.dart';
import '../../src/rust/api/conflict.dart';
import '../../src/rust/api/error.dart';
import '../../src/rust/api/events.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../notes/widgets/note_image.dart';
import '../sync/git_identity.dart';
import 'conflict_state.dart';
import 'resolve_merge.dart';
import 'widgets/conflict_file_tile.dart';
import 'widgets/finish_on_desktop_sheet.dart';

/// Every note a paused sync is waiting on, and the two ways out: settle them
/// here, or merge the side branch on the desktop.
class ConflictListScreen extends ConsumerStatefulWidget {
  const ConflictListScreen({super.key});

  static Future<void> open(BuildContext context) {
    return Navigator.of(
      context,
    ).push(MaterialPageRoute<void>(builder: (_) => const ConflictListScreen()));
  }

  @override
  ConsumerState<ConflictListScreen> createState() => _ConflictListScreenState();
}

class _ConflictListScreenState extends ConsumerState<ConflictListScreen> {
  bool _merging = false;
  String? _phase;
  String? _error;

  Future<void> _merge(PendingConflict pending) async {
    final settings = await ref.read(settingsProvider.future);
    final workspace = await ref.read(workspaceProvider.future);
    if (workspace == null) return;

    setState(() {
      _merging = true;
      _error = null;
      _phase = 'Preparing';
    });

    try {
      final resolutions = await collectResolutions(
        pending,
        ref.read(conflictDraftsProvider),
        (relPath) =>
            rust.conflictDetail(root: workspace.root, relPath: relPath),
      );
      final creds = gitCredsFor(workspace.config);
      final stream = rust.gitResolve(
        root: workspace.root,
        attachmentsDir: workspace.config.attachmentsDir,
        authorName: settings.commitAuthorName,
        authorEmail: settings.commitAuthorEmail,
        creds: creds,
        resolutions: resolutions,
      );

      await for (final event in stream) {
        if (!mounted) return;
        switch (event) {
          case SyncEvent_Progress(:final progress):
            setState(() => _phase = progress.phase.name);
          case SyncEvent_Done(:final report):
            await ref.read(conflictDraftsProvider.notifier).clear();
            evictChangedImages(report.changedPaths);
            ref.read(notesRevisionProvider.notifier).value++;
            if (!mounted) return;
            ScaffoldMessenger.of(
              context,
            ).showSnackBar(const SnackBar(content: Text('Merged and synced')));
            Navigator.pop(context);
            return;
          case SyncEvent_Failed(:final failure):
            final stale = failure.gitKind == GitErrorKind.stale;
            if (stale) {
              // Catch up with the remote: this pauses again on its newest
              // commit, or finishes if the desktop merged in the meantime.
              setState(() => _phase = 'Refreshing');
              await rust
                  .gitSync(
                    root: workspace.root,
                    attachmentsDir: workspace.config.attachmentsDir,
                    authorName: settings.commitAuthorName,
                    authorEmail: settings.commitAuthorEmail,
                    creds: creds,
                  )
                  .drain<void>();
            }
            ref.read(notesRevisionProvider.notifier).value++;
            if (!mounted) return;
            setState(() {
              _merging = false;
              _error = stale
                  ? 'Something changed while you were resolving, so nothing '
                        'was merged. Notes that changed are marked again; '
                        'check them and try once more.'
                  : failure.message;
            });
        }
      }
    } catch (e) {
      ref.read(notesRevisionProvider.notifier).value++;
      if (mounted) {
        setState(() {
          _merging = false;
          _error = '$e';
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final pending = ref.watch(pendingConflictProvider);
    return Scaffold(
      appBar: AppBar(title: const Text('Waiting to merge')),
      body: pending.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text('$e')),
        data: (pending) => pending == null
            ? const _Empty(
                'Nothing is waiting to be merged. Your notes are in sync.',
              )
            : _body(context, pending),
      ),
    );
  }

  Widget _body(BuildContext context, PendingConflict pending) {
    final drafts = ref.watch(conflictDraftsProvider);
    final allDone =
        pending.files.isNotEmpty &&
        pending.files.every((f) => drafts[f.path]?.done ?? false);

    return Column(
      children: [
        Expanded(
          child: _ConflictOverview(pending: pending, drafts: drafts),
        ),
        _MergeFooter(
          merging: _merging,
          phase: _phase,
          error: _error,
          onFinishOnDesktop: () =>
              FinishOnDesktopSheet.show(context, pending.sideBranch),
          onMerge: allDone && !_merging ? () => _merge(pending) : null,
        ),
      ],
    );
  }
}

/// The remote commit the sync stopped at, the notes that clash and the ones
/// held back until they are merged.
class _ConflictOverview extends StatelessWidget {
  const _ConflictOverview({required this.pending, required this.drafts});

  final PendingConflict pending;
  final Map<String, NoteDraft> drafts;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final when = DateFormat.MMMd().add_Hm().format(
      DateTime.fromMillisecondsSinceEpoch(pending.remote.timeMs),
    );

    return ListView(
      padding: const EdgeInsets.only(bottom: 16),
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
          child: Text(
            'These notes changed both here and on another device. Your '
            'edits are safe on ${pending.sideBranch}. Choose what to '
            'keep here, or finish the merge on your desktop.',
            style: theme.textTheme.bodyMedium,
          ),
        ),
        ListTile(
          dense: true,
          leading: const Icon(Icons.cloud_outlined),
          title: Text(
            pending.remote.summary.isEmpty
                ? 'Remote changes'
                : pending.remote.summary,
            overflow: TextOverflow.ellipsis,
          ),
          subtitle: Text('${pending.remote.author} · $when'),
        ),
        const Divider(),
        if (pending.files.isEmpty)
          const _Empty('No notes clash any more. Sync again to finish.'),
        for (final file in pending.files)
          ConflictFileTile(file: file, draft: drafts[file.path]),
        if (pending.heldBack.isNotEmpty)
          ExpansionTile(
            leading: const Icon(Icons.pause_circle_outline),
            title: Text(
              '${pending.heldBack.length} other '
              '${pending.heldBack.length == 1 ? 'note' : 'notes'} '
              'changed on the remote',
            ),
            subtitle: const Text('They arrive once the merge is done.'),
            children: [
              for (final path in pending.heldBack)
                ListTile(
                  dense: true,
                  title: Text(rust.relPathToNoteName(relPath: path)),
                ),
            ],
          ),
      ],
    );
  }
}

/// Progress and errors of the merge, over the two ways to finish.
class _MergeFooter extends StatelessWidget {
  const _MergeFooter({
    required this.merging,
    required this.phase,
    required this.error,
    required this.onFinishOnDesktop,
    required this.onMerge,
  });

  final bool merging;
  final String? phase;
  final String? error;
  final VoidCallback onFinishOnDesktop;
  final VoidCallback? onMerge;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        if (merging) ...[
          const LinearProgressIndicator(),
          Padding(
            padding: const EdgeInsets.only(top: 4),
            child: Text(phase ?? '', style: theme.textTheme.bodySmall),
          ),
        ],
        if (error != null)
          Container(
            width: double.infinity,
            margin: const EdgeInsets.fromLTRB(16, 8, 16, 0),
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: theme.colorScheme.errorContainer,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Text(
              error!,
              style: TextStyle(color: theme.colorScheme.onErrorContainer),
            ),
          ),
        SafeArea(
          top: false,
          child: Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 12),
            child: Row(
              children: [
                TextButton(
                  onPressed: onFinishOnDesktop,
                  child: const Text('Finish on desktop'),
                ),
                const Spacer(),
                FilledButton.icon(
                  icon: const Icon(Icons.merge),
                  label: const Text('Merge & sync'),
                  onPressed: onMerge,
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }
}

class _Empty extends StatelessWidget {
  const _Empty(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(32),
      child: Text(text, textAlign: TextAlign.center),
    );
  }
}
