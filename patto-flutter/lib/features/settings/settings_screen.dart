import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers.dart';
import '../../core/settings.dart';
import '../../src/rust/api/events.dart';
import '../../src/rust/api/git.dart';
import '../../src/rust/frb_api.dart' as rust;

class SettingsScreen extends ConsumerStatefulWidget {
  const SettingsScreen({super.key, this.onboarding = false});

  /// In onboarding the screen is the whole app until a clone succeeds.
  final bool onboarding;

  static Future<void> open(BuildContext context) {
    return Navigator.of(context).push(
      MaterialPageRoute<void>(builder: (_) => const SettingsScreen()),
    );
  }

  @override
  ConsumerState<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends ConsumerState<SettingsScreen> {
  final _repoUrl = TextEditingController();
  final _branch = TextEditingController();
  final _username = TextEditingController();
  final _token = TextEditingController();
  final _authorName = TextEditingController();
  final _authorEmail = TextEditingController();

  bool _filled = false;
  bool _cloning = false;
  String? _phase;
  String? _error;

  @override
  void dispose() {
    for (final c in [
      _repoUrl,
      _branch,
      _username,
      _token,
      _authorName,
      _authorEmail,
    ]) {
      c.dispose();
    }
    super.dispose();
  }

  void _fill(Settings settings) {
    if (_filled) return;
    _filled = true;
    _repoUrl.text = settings.repoUrl;
    _branch.text = settings.branch;
    _username.text = settings.username;
    _token.text = settings.token;
    _authorName.text = settings.authorName;
    _authorEmail.text = settings.authorEmail;
  }

  Settings _collect(Settings base) => base.copyWith(
    repoUrl: _repoUrl.text.trim(),
    branch: _branch.text.trim(),
    username: _username.text.trim(),
    token: _token.text.trim(),
    authorName: _authorName.text.trim(),
    authorEmail: _authorEmail.text.trim(),
  );

  Future<void> _save(Settings base) async {
    await ref.read(settingsProvider.notifier).save(_collect(base));
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('Saved')),
    );
  }

  Future<void> _clone(Settings base) async {
    final settings = _collect(base);
    if (settings.repoUrl.isEmpty) {
      setState(() => _error = 'Enter the repository URL.');
      return;
    }

    await ref.read(settingsProvider.notifier).save(settings);
    final workspace = await ref.read(workspaceProvider.future);
    if (!mounted) return;

    if (workspace.exists) {
      final replace = await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          title: const Text('Replace local notes?'),
          content: const Text(
            'Cloning deletes the notes already on this device. '
            'Anything not pushed will be lost.',
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(context, true),
              child: const Text('Replace'),
            ),
          ],
        ),
      );
      if (replace != true) return;
      await workspace.clear();
    }

    setState(() {
      _cloning = true;
      _error = null;
      _phase = 'Connecting';
    });

    try {
      final stream = rust.gitClone(
        url: settings.repoUrl,
        root: workspace.root,
        branch: settings.branch.isEmpty ? null : settings.branch,
        creds: GitCreds(username: settings.username, token: settings.token),
      );

      var cloned = false;
      await for (final event in stream) {
        if (!mounted) return;
        switch (event) {
          case CloneEvent_Progress(:final progress):
            setState(
              () => _phase = 'Receiving ${progress.current}/${progress.total}',
            );
          case CloneEvent_Done():
            cloned = true;
          case CloneEvent_Failed(:final failure):
            setState(() {
              _cloning = false;
              _error = failure.message;
            });
            return;
        }
      }

      if (!mounted) return;
      setState(() => _cloning = false);
      if (!cloned) return;

      ref.read(notesRevisionProvider.notifier).value++;
      unawaited(ref.read(indexProvider.notifier).rebuild(workspace.root));

      if (widget.onboarding) {
        // The bootstrap gate re-reads the workspace and shows the note list.
        ref.invalidate(workspaceProvider);
      } else {
        Navigator.of(context).pop();
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _cloning = false;
          _error = e.toString();
        });
      }
    }
  }

  Future<void> _clearLocalData() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Delete local notes?'),
        content: const Text('Anything not pushed will be lost.'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
    if (confirmed != true) return;

    final workspace = await ref.read(workspaceProvider.future);
    await workspace.clear();
    ref.read(notesRevisionProvider.notifier).value++;
    ref.invalidate(workspaceProvider);
  }

  @override
  Widget build(BuildContext context) {
    final settings = ref.watch(settingsProvider);

    return Scaffold(
      appBar: AppBar(
        title: Text(widget.onboarding ? 'Set up Patto Notes' : 'Settings'),
        automaticallyImplyLeading: !widget.onboarding,
      ),
      body: settings.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text('$e')),
        data: (data) {
          _fill(data);
          return ListView(
            padding: const EdgeInsets.all(16),
            children: [
              if (widget.onboarding)
                const Padding(
                  padding: EdgeInsets.only(bottom: 16),
                  child: Text(
                    'Patto Notes keeps your notes in a git repository. '
                    'Enter the repository to clone it onto this device.',
                  ),
                ),
              _Section('Repository'),
              TextField(
                controller: _repoUrl,
                decoration: const InputDecoration(
                  labelText: 'HTTPS URL',
                  hintText: 'https://github.com/you/notes.git',
                ),
                keyboardType: TextInputType.url,
              ),
              TextField(
                controller: _branch,
                decoration: const InputDecoration(
                  labelText: 'Branch',
                  hintText: 'default branch',
                ),
              ),
              TextField(
                controller: _username,
                decoration: const InputDecoration(labelText: 'Username'),
              ),
              TextField(
                controller: _token,
                decoration: const InputDecoration(
                  labelText: 'Access token',
                  helperText: 'Stored in the device keystore',
                ),
                obscureText: true,
              ),
              const SizedBox(height: 20),
              _Section('Commits'),
              TextField(
                controller: _authorName,
                decoration: const InputDecoration(labelText: 'Author name'),
              ),
              TextField(
                controller: _authorEmail,
                decoration: const InputDecoration(labelText: 'Author email'),
                keyboardType: TextInputType.emailAddress,
              ),
              const SizedBox(height: 20),
              _Section('Appearance'),
              SegmentedButton<ThemeMode>(
                showSelectedIcon: false,
                segments: const [
                  ButtonSegment(value: ThemeMode.system, label: Text('System')),
                  ButtonSegment(value: ThemeMode.light, label: Text('Light')),
                  ButtonSegment(value: ThemeMode.dark, label: Text('Dark')),
                ],
                selected: {data.themeMode},
                onSelectionChanged: (s) => ref
                    .read(settingsProvider.notifier)
                    .save(_collect(data).copyWith(themeMode: s.first)),
              ),
              const SizedBox(height: 24),
              if (_cloning) ...[
                const LinearProgressIndicator(),
                const SizedBox(height: 8),
                Text(_phase ?? ''),
                const SizedBox(height: 16),
              ],
              if (_error != null)
                Container(
                  width: double.infinity,
                  margin: const EdgeInsets.only(bottom: 16),
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: Theme.of(context).colorScheme.errorContainer,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Text(_error!),
                ),
              Row(
                children: [
                  Expanded(
                    child: OutlinedButton(
                      onPressed: _cloning ? null : () => _save(data),
                      child: const Text('Save'),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: FilledButton(
                      onPressed: _cloning ? null : () => _clone(data),
                      child: const Text('Clone'),
                    ),
                  ),
                ],
              ),
              if (!widget.onboarding) ...[
                const SizedBox(height: 24),
                TextButton(
                  onPressed: _clearLocalData,
                  child: const Text('Delete local notes'),
                ),
              ],
            ],
          );
        },
      ),
    );
  }
}

class _Section extends StatelessWidget {
  const _Section(this.title);

  final String title;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Text(title, style: Theme.of(context).textTheme.titleSmall),
    );
  }
}
