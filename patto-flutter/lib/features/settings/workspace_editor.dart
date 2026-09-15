import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers.dart';
import '../../core/workspace.dart';
import '../../src/rust/api/events.dart';
import '../../src/rust/api/git.dart';
import '../../src/rust/frb_api.dart' as rust;

/// Add or edit one workspace, and clone it.
///
/// The same screen is the app's first run: with no workspace configured there
/// is nothing else to show.
class WorkspaceEditorScreen extends ConsumerStatefulWidget {
  const WorkspaceEditorScreen({
    super.key,
    this.existing,
    this.onboarding = false,
  });

  /// Null when adding a workspace.
  final Workspace? existing;

  final bool onboarding;

  static Future<void> open(BuildContext context, {Workspace? existing}) {
    return Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => WorkspaceEditorScreen(existing: existing),
      ),
    );
  }

  @override
  ConsumerState<WorkspaceEditorScreen> createState() =>
      _WorkspaceEditorScreenState();
}

class _WorkspaceEditorScreenState extends ConsumerState<WorkspaceEditorScreen> {
  final _name = TextEditingController();
  final _repoUrl = TextEditingController();
  final _branch = TextEditingController();
  final _username = TextEditingController();
  final _token = TextEditingController();

  bool _cloning = false;
  String? _phase;
  String? _error;

  Workspace? get _existing => widget.existing;

  @override
  void initState() {
    super.initState();
    final existing = _existing;
    if (existing != null) {
      _name.text = existing.name;
      _repoUrl.text = existing.repoUrl;
      _branch.text = existing.branch;
      _username.text = existing.username;
      _token.text = existing.token;
    }
  }

  @override
  void dispose() {
    for (final c in [_name, _repoUrl, _branch, _username, _token]) {
      c.dispose();
    }
    super.dispose();
  }

  String? _newId;

  /// Identity for a workspace being added, generated once so saving and then
  /// cloning does not create two of them.
  String get _id => _existing?.id ?? (_newId ??= Workspace.newId());

  /// The workspace as the form currently describes it, keeping the identity and
  /// folder of the one being edited.
  Workspace _collect() {
    final repoUrl = _repoUrl.text.trim();
    final name = _name.text.trim().isEmpty
        ? Workspace.nameFromUrl(repoUrl)
        : _name.text.trim();

    return Workspace(
      id: _id,
      // A new workspace's folder is its id; an existing one keeps the folder it
      // was cloned into.
      dirName: _existing?.dirName ?? _id,
      name: name,
      repoUrl: repoUrl,
      branch: _branch.text.trim(),
      username: _username.text.trim(),
      token: _token.text.trim(),
    );
  }

  Future<String?> _rootFor(Workspace workspace) async {
    final baseDir = await ref.read(workspaceBaseDirProvider.future);
    return WorkspaceStorage.rootFor(baseDir, workspace);
  }

  Future<void> _save() async {
    final workspace = _collect();
    await ref.read(settingsProvider.notifier).saveWorkspace(workspace);

    // Create the folder so a workspace with no remote is usable straight away
    // as a local-only set of notes.
    final root = await _rootFor(workspace);
    if (root != null) {
      await Directory(root).create(recursive: true);
    }

    ref.invalidate(workspaceProvider);
    if (!mounted) return;
    Navigator.of(context).pop();
  }

  Future<void> _clone() async {
    final workspace = _collect();
    if (workspace.repoUrl.isEmpty) {
      setState(() => _error = 'Enter the repository URL.');
      return;
    }

    await ref.read(settingsProvider.notifier).saveWorkspace(workspace);
    final root = await _rootFor(workspace);
    if (root == null || !mounted) return;

    final existingClone = await _confirmReplace(root);
    if (existingClone == false || !mounted) return;

    setState(() {
      _cloning = true;
      _error = null;
      _phase = 'Connecting';
    });

    try {
      final stream = rust.gitClone(
        url: workspace.repoUrl,
        root: root,
        branch: workspace.branch.isEmpty ? null : workspace.branch,
        creds: GitCreds(
          username: workspace.username,
          token: workspace.token,
        ),
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

      // Show the workspace that was just cloned.
      await ref.read(settingsProvider.notifier).setActiveWorkspace(workspace.id);
      ref.read(indexProvider.notifier).forget(root);
      ref.read(notesRevisionProvider.notifier).value++;
      ref.invalidate(workspaceProvider);

      if (!widget.onboarding && mounted) {
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

  /// Returns false when the user declines to replace an existing clone.
  Future<bool> _confirmReplace(String root) async {
    final workspace = ActiveWorkspace(config: _collect(), root: root);
    if (!workspace.exists) return true;

    final replace = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Replace these notes?'),
        content: const Text(
          'Cloning deletes the notes already in this workspace. '
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
    if (replace != true) return false;

    await workspace.clear();
    return true;
  }

  @override
  Widget build(BuildContext context) {
    final title = widget.onboarding
        ? 'Set up Patto Notes'
        : (_existing == null ? 'Add workspace' : 'Edit workspace');

    return Scaffold(
      appBar: AppBar(
        title: Text(title),
        automaticallyImplyLeading: !widget.onboarding,
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          if (widget.onboarding)
            const Padding(
              padding: EdgeInsets.only(bottom: 16),
              child: Text(
                'Patto Notes keeps your notes in a git repository. Enter the '
                'repository to clone it onto this device. You can add more '
                'workspaces later.',
              ),
            ),
          TextField(
            controller: _name,
            decoration: const InputDecoration(
              labelText: 'Name',
              helperText: 'Defaults to the repository name',
            ),
          ),
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
              if (!widget.onboarding) ...[
                Expanded(
                  child: OutlinedButton(
                    onPressed: _cloning ? null : _save,
                    child: const Text('Save'),
                  ),
                ),
                const SizedBox(width: 12),
              ],
              Expanded(
                child: FilledButton(
                  onPressed: _cloning ? null : _clone,
                  child: const Text('Clone'),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
