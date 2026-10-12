import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers.dart';
import '../../src/rust/api/events.dart';
import '../../src/rust/api/git.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../conflicts/conflict_state.dart';
import '../notes/widgets/note_image.dart';
import '../settings/settings_screen.dart';
import 'git_identity.dart';
import 'sync_messages.dart';
import 'sync_providers.dart';
import 'widgets/paused_card.dart';

/// Shows what is uncommitted, and runs commit, pull and push.
class SyncSheet extends ConsumerStatefulWidget {
  const SyncSheet({super.key});

  static Future<void> show(BuildContext context) {
    return showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      builder: (_) => const SyncSheet(),
    );
  }

  @override
  ConsumerState<SyncSheet> createState() => _SyncSheetState();
}

class _SyncSheetState extends ConsumerState<SyncSheet> {
  bool _running = false;
  String? _phase;
  String? _error;
  SyncReport? _report;

  Future<void> _sync() async {
    final settings = await ref.read(settingsProvider.future);
    final workspace = await ref.read(workspaceProvider.future);

    if (workspace == null || !workspace.config.hasRemote) {
      setState(
        () =>
            _error = 'Set this workspace\'s repository URL in settings first.',
      );
      return;
    }

    setState(() {
      _running = true;
      _error = null;
      _report = null;
      _phase = 'Starting';
    });

    try {
      final stream = rust.gitSync(
        root: workspace.root,
        attachmentsDir: workspace.config.attachmentsDir,
        authorName: settings.commitAuthorName,
        authorEmail: settings.commitAuthorEmail,
        creds: gitCredsFor(workspace.config),
      );

      await for (final event in stream) {
        if (!mounted) return;
        switch (event) {
          case SyncEvent_Progress(:final progress):
            setState(() => _phase = syncPhaseLabel(progress));
          case SyncEvent_Done(:final report):
            setState(() {
              _running = false;
              _report = report;
            });
            // Merged on the desktop: choices made here are moot.
            if (report.conflictCleared) {
              await ref.read(conflictDraftsProvider.notifier).clear();
            }
            evictChangedImages(report.changedPaths);
            ref.read(notesRevisionProvider.notifier).value++;
          case SyncEvent_Failed(:final failure):
            setState(() {
              _running = false;
              _error = syncAdvice(failure);
            });
        }
      }

      if (mounted) setState(() => _running = false);
    } catch (e) {
      if (mounted) {
        setState(() {
          _running = false;
          _error = e.toString();
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final status = ref.watch(gitStatusProvider);

    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Flexible(
                  child: Text(
                    ref.watch(workspaceProvider).value?.config.name ?? 'Sync',
                    style: theme.textTheme.titleLarge,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                const Spacer(),
                IconButton(
                  icon: const Icon(Icons.settings),
                  tooltip: 'Settings',
                  onPressed: () {
                    Navigator.pop(context);
                    SettingsScreen.open(context);
                  },
                ),
              ],
            ),
            const SizedBox(height: 8),
            status.when(
              loading: () => const LinearProgressIndicator(minHeight: 2),
              error: (e, _) => Text('$e'),
              data: (s) => s == null
                  ? const Text('No repository cloned yet.')
                  : _StatusSummary(status: s),
            ),
            const SizedBox(height: 16),
            if (_running) ...[
              const LinearProgressIndicator(),
              const SizedBox(height: 8),
              Text(_phase ?? '', style: theme.textTheme.bodySmall),
            ],
            if (_report case SyncReport(
              merge: MergeOutcome_Conflicted(:final sideBranch, :final paths),
            ))
              PausedCard(sideBranch: sideBranch, count: paths.length)
            else if (_report != null)
              Text(
                syncReportSummary(_report!),
                style: theme.textTheme.bodyMedium,
              )
            else if (status.value?.conflictPending ?? false)
              const PausedCard(),
            if (_error != null)
              Container(
                width: double.infinity,
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
            const SizedBox(height: 16),
            SizedBox(
              width: double.infinity,
              child: FilledButton.icon(
                onPressed: _running ? null : _sync,
                icon: const Icon(Icons.sync),
                label: const Text('Sync now'),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// The branch, the counts and the first few changed files.
class _StatusSummary extends StatelessWidget {
  const _StatusSummary({required this.status});

  final GitStatus status;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('Branch: ${status.branch}'),
        Text(
          '${status.dirty.length} changed, '
          '${status.ahead} to push, ${status.behind} to pull',
        ),
        if (status.dirty.isNotEmpty)
          Padding(
            padding: const EdgeInsets.only(top: 6),
            child: Text(
              status.dirty.take(6).join('\n'),
              style: theme.textTheme.bodySmall,
            ),
          ),
      ],
    );
  }
}
