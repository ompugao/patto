import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../../core/providers.dart';
import '../../src/rust/api/conflict.dart';
import '../../src/rust/api/error.dart';
import '../../src/rust/api/events.dart';
import '../../src/rust/api/git.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../notes/widgets/note_image.dart';
import 'conflict_note_screen.dart';
import 'conflict_state.dart';

/// Every note a paused sync is waiting on, and the two ways out: settle them
/// here, or merge the side branch on the desktop.
class ConflictListScreen extends ConsumerStatefulWidget {
  const ConflictListScreen({super.key});

  static Future<void> open(BuildContext context) {
    return Navigator.of(context).push(
      MaterialPageRoute<void>(builder: (_) => const ConflictListScreen()),
    );
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
      final drafts = ref.read(conflictDraftsProvider);
      final resolutions = <Resolution>[];
      for (final file in pending.files) {
        final detail = await rust.conflictDetail(
          root: workspace.root,
          relPath: file.path,
        );
        final draft = drafts[file.path];
        if (draft == null || !draft.matches(detail) || !draft.done) {
          throw StateError('${file.path} changed since it was reviewed');
        }
        resolutions.add(
          Resolution(
            path: file.path,
            oursId: detail.oursId,
            theirsId: detail.theirsId,
            content: mergedContent(detail, draft),
          ),
        );
      }

      final authorName = settings.authorName.isEmpty
          ? 'Patto'
          : settings.authorName;
      final authorEmail = settings.authorEmail.isEmpty
          ? 'patto@localhost'
          : settings.authorEmail;
      final creds = GitCreds(
        username: workspace.config.username,
        token: workspace.config.token,
      );
      final stream = rust.gitResolve(
        root: workspace.root,
        attachmentsDir: workspace.config.attachmentsDir,
        authorName: authorName,
        authorEmail: authorEmail,
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
            ScaffoldMessenger.of(context).showSnackBar(
              const SnackBar(content: Text('Merged and synced')),
            );
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
                    authorName: authorName,
                    authorEmail: authorEmail,
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

  void _finishOnDesktop(PendingConflict pending) {
    final branch = pending.sideBranch;
    showModalBottomSheet<void>(
      context: context,
      showDragHandle: true,
      builder: (context) {
        final theme = Theme.of(context);
        final commands = 'git fetch origin\ngit merge origin/$branch';
        return SafeArea(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(20, 0, 20, 20),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Finish on the desktop', style: theme.textTheme.titleLarge),
                const SizedBox(height: 8),
                Text(
                  'Your edits are already on the remote, on the branch '
                  '$branch. Merge it on the desktop and push; the next sync '
                  'here picks the result up and removes the branch.',
                ),
                const SizedBox(height: 12),
                Container(
                  width: double.infinity,
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: SelectableText(
                    commands,
                    style: const TextStyle(fontFamily: 'monospace'),
                  ),
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    TextButton.icon(
                      icon: const Icon(Icons.copy),
                      label: const Text('Copy commands'),
                      onPressed: () {
                        Clipboard.setData(ClipboardData(text: commands));
                        Navigator.pop(context);
                      },
                    ),
                    const Spacer(),
                    FilledButton(
                      onPressed: () => Navigator.pop(context),
                      child: const Text('OK'),
                    ),
                  ],
                ),
              ],
            ),
          ),
        );
      },
    );
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
    final theme = Theme.of(context);
    final drafts = ref.watch(conflictDraftsProvider);
    final allDone =
        pending.files.isNotEmpty &&
        pending.files.every((f) => drafts[f.path]?.done ?? false);
    final when = DateFormat.MMMd().add_Hm().format(
      DateTime.fromMillisecondsSinceEpoch(pending.remote.timeMs),
    );

    return Column(
      children: [
        Expanded(
          child: ListView(
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
                const _Empty(
                  'No notes clash any more. Sync again to finish.',
                ),
              for (final file in pending.files)
                _FileTile(file: file, draft: drafts[file.path]),
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
          ),
        ),
        if (_merging) ...[
          const LinearProgressIndicator(),
          Padding(
            padding: const EdgeInsets.only(top: 4),
            child: Text(_phase ?? '', style: theme.textTheme.bodySmall),
          ),
        ],
        if (_error != null)
          Container(
            width: double.infinity,
            margin: const EdgeInsets.fromLTRB(16, 8, 16, 0),
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: theme.colorScheme.errorContainer,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Text(
              _error!,
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
                  onPressed: () => _finishOnDesktop(pending),
                  child: const Text('Finish on desktop'),
                ),
                const Spacer(),
                FilledButton.icon(
                  icon: const Icon(Icons.merge),
                  label: const Text('Merge & sync'),
                  onPressed: allDone && !_merging
                      ? () => _merge(pending)
                      : null,
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }
}

class _FileTile extends StatelessWidget {
  const _FileTile({required this.file, required this.draft});

  final ConflictFile file;
  final NoteDraft? draft;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final done = draft?.done ?? false;
    final started = !done && (draft?.choices.isNotEmpty ?? false);

    final summary = switch (file.kind) {
      ConflictKind.deletedByThem => 'Remote deleted it · you changed '
          '${file.oursChanged} lines',
      ConflictKind.deletedByUs => 'You deleted it · remote changed '
          '${file.theirsChanged} lines',
      ConflictKind.bothAdded => 'Created on both sides',
      ConflictKind.bothModified => 'You: ${file.oursChanged} lines · '
          'Remote: ${file.theirsChanged} lines',
    };
    final clashes = switch (file.conflicts) {
      0 => 'merges cleanly, just confirm',
      1 => '1 conflict',
      final n => '$n conflicts',
    };

    return ListTile(
      leading: Icon(
        done
            ? Icons.check_circle
            : started
            ? Icons.timelapse
            : Icons.warning_amber_rounded,
        color: done ? theme.colorScheme.primary : Colors.amber.shade800,
      ),
      title: Text(rust.relPathToNoteName(relPath: file.path)),
      subtitle: Text(
        isWholeNote(file.kind) ? summary : '$summary\n$clashes',
      ),
      isThreeLine: !isWholeNote(file.kind),
      trailing: const Icon(Icons.chevron_right),
      onTap: () => ConflictNoteScreen.open(context, file.path),
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
