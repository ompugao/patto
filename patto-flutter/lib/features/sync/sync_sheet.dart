import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers.dart';
import '../../src/rust/api/error.dart';
import '../../src/rust/api/events.dart';
import '../../src/rust/api/git.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../settings/settings_screen.dart';

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
        () => _error = 'Set this workspace\'s repository URL in settings first.',
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
        authorName: settings.authorName.isEmpty ? 'Patto' : settings.authorName,
        authorEmail: settings.authorEmail.isEmpty
            ? 'patto@localhost'
            : settings.authorEmail,
        creds: GitCreds(
          username: workspace.config.username,
          token: workspace.config.token,
        ),
      );

      await for (final event in stream) {
        if (!mounted) return;
        switch (event) {
          case SyncEvent_Progress(:final progress):
            setState(() => _phase = _phaseLabel(progress));
          case SyncEvent_Done(:final report):
            setState(() {
              _running = false;
              _report = report;
            });
            ref.read(notesRevisionProvider.notifier).value++;
          case SyncEvent_Failed(:final failure):
            setState(() {
              _running = false;
              _error = _advice(failure);
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

  /// Turn a failure into something the user can act on.
  String _advice(Failure failure) => switch (failure.gitKind) {
    GitErrorKind.auth =>
      'The server rejected the credentials. Check the username and token.',
    GitErrorKind.network => 'Could not reach the server. Check the connection.',
    GitErrorKind.certificate =>
      'The server certificate could not be verified.\n\n${failure.message}',
    GitErrorKind.noRemote => 'This clone has no "origin" remote.',
    GitErrorKind.notARepo =>
      'The notes folder is not a git repository. Clone again.',
    GitErrorKind.nonFastForward => 'The remote moved on while syncing. Try again.',
    GitErrorKind.conflict =>
      'The merge could not be resolved here. Resolve it on the desktop.',
    _ => failure.message,
  };

  String _phaseLabel(GitProgress p) => switch (p.phase) {
    GitPhase.connecting => 'Connecting',
    GitPhase.counting => 'Counting objects',
    GitPhase.receiving => 'Receiving ${p.current}/${p.total}',
    GitPhase.resolving => 'Resolving',
    GitPhase.checkingOut => 'Checking out',
    GitPhase.committing => 'Committing',
    GitPhase.merging => 'Merging',
    GitPhase.pushing => 'Pushing',
    GitPhase.done => 'Done',
  };

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
                  : Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text('Branch: ${s.branch}'),
                        Text(
                          '${s.dirty.length} changed, '
                          '${s.ahead} to push, ${s.behind} to pull',
                        ),
                        if (s.dirty.isNotEmpty)
                          Padding(
                            padding: const EdgeInsets.only(top: 6),
                            child: Text(
                              s.dirty.take(6).join('\n'),
                              style: theme.textTheme.bodySmall,
                            ),
                          ),
                      ],
                    ),
            ),
            const SizedBox(height: 16),
            if (_running) ...[
              const LinearProgressIndicator(),
              const SizedBox(height: 8),
              Text(_phase ?? '', style: theme.textTheme.bodySmall),
            ],
            if (_report != null)
              Text(
                [
                  if (_report!.committed) 'Committed',
                  switch (_report!.merge) {
                    MergeOutcome_UpToDate() => 'Already up to date',
                    MergeOutcome_FastForward() => 'Fast-forwarded',
                    MergeOutcome_Merged(:final autoResolved) =>
                      'Merged, kept local copy of ${autoResolved.length} file(s)',
                  },
                  if (_report!.pushed) 'Pushed',
                ].join(' · '),
                style: theme.textTheme.bodyMedium,
              ),
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
