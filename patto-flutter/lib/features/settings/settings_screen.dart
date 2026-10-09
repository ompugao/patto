import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers.dart';
import '../../core/settings.dart';
import '../../core/workspace.dart';
import '../../src/rust/frb_api.dart' as rust;
import 'inbox_note_name.dart';
import 'widgets/font_size_setting.dart';
import 'widgets/settings_widgets.dart';
import 'workspace_editor.dart';

class SettingsScreen extends ConsumerStatefulWidget {
  const SettingsScreen({super.key});

  static Future<void> open(BuildContext context) {
    return Navigator.of(context)
        .push(MaterialPageRoute<void>(builder: (_) => const SettingsScreen()));
  }

  @override
  ConsumerState<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends ConsumerState<SettingsScreen> {
  final _authorName = TextEditingController();
  final _authorEmail = TextEditingController();
  final _inboxNoteName = TextEditingController();
  bool _filled = false;

  @override
  void dispose() {
    _authorName.dispose();
    _authorEmail.dispose();
    _inboxNoteName.dispose();
    super.dispose();
  }

  void _fill(Settings settings) {
    if (_filled) return;
    _filled = true;
    _authorName.text = settings.authorName;
    _authorEmail.text = settings.authorEmail;
    _inboxNoteName.text = settings.inboxNoteName;
  }

  /// An empty inbox field means the default; an invalid one keeps what was
  /// stored, so toggling another setting never silently replaces it.
  Settings _collect(Settings base) => base.copyWith(
    authorName: _authorName.text.trim(),
    authorEmail: _authorEmail.text.trim(),
    inboxNoteName: _inboxNameError == null
        ? _orDefault(_inboxNoteName.text, Settings.defaultInboxNoteName)
        : base.inboxNoteName,
  );

  static String _orDefault(String value, String fallback) =>
      value.trim().isEmpty ? fallback : value.trim();

  String? get _inboxNameError =>
      inboxNoteNameError(_inboxNoteName.text, coreAccepts: _coreAcceptsName);

  static bool _coreAcceptsName(String name) {
    try {
      rust.noteNameToRelPath(name: name);
      return true;
    } on Exception {
      return false;
    }
  }

  Future<void> _saveFields(Settings base) async {
    final error = _inboxNameError;
    if (error != null) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('Not saved. Inbox note: $error')));
      return;
    }
    await ref.read(settingsProvider.notifier).save(_collect(base));
    if (!mounted) return;
    ScaffoldMessenger.of(context)
        .showSnackBar(const SnackBar(content: Text('Saved')));
  }

  Future<void> _switchTo(Workspace workspace) async {
    await ref.read(settingsProvider.notifier).setActiveWorkspace(workspace.id);
  }

  Future<void> _delete(Workspace workspace) async {
    final confirmed = await _confirmRemove(context, workspace);
    if (confirmed != true) return;

    final baseDir = await ref.read(workspaceBaseDirProvider.future);
    final root = WorkspaceStorage.rootFor(baseDir, workspace);
    await ActiveWorkspace(config: workspace, root: root).clear();

    ref.read(indexProvider.notifier).forget(root);
    await ref.read(settingsProvider.notifier).removeWorkspace(workspace.id);
    ref.read(notesRevisionProvider.notifier).value++;
  }

  @override
  Widget build(BuildContext context) {
    final settings = ref.watch(settingsProvider);

    return Scaffold(
      appBar: AppBar(title: const Text('Settings')),
      body: settings.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text('$e')),
        data: (data) {
          _fill(data);
          final active = data.active;

          return ListView(
            padding: const EdgeInsets.all(16),
            children: [
              const SettingsSection('Workspaces'),
              for (final workspace in data.workspaces)
                WorkspaceTile(
                  workspace: workspace,
                  isActive: workspace.id == active?.id,
                  onSwitch: () => _switchTo(workspace),
                  onEdit: () =>
                      WorkspaceEditorScreen.open(context, existing: workspace),
                  onDelete: () => _delete(workspace),
                ),
              Align(
                alignment: Alignment.centerLeft,
                child: TextButton.icon(
                  onPressed: () => WorkspaceEditorScreen.open(context),
                  icon: const Icon(Icons.add),
                  label: const Text('Add workspace'),
                ),
              ),
              const SizedBox(height: 20),
              _CommitIdentityFields(
                authorName: _authorName,
                authorEmail: _authorEmail,
                onSave: () => _saveFields(data),
              ),
              const SizedBox(height: 24),
              const SettingsSection('Appearance'),
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
              const SizedBox(height: 16),
              FontSizeSetting(
                scale: data.fontScale,
                onChanged: (scale) => ref
                    .read(settingsProvider.notifier)
                    .save(_collect(data).copyWith(fontScale: scale)),
              ),
              const SizedBox(height: 24),
              _InboxNoteField(
                controller: _inboxNoteName,
                errorText: _inboxNameError,
                onChanged: () => setState(() {}),
                onSave: () => _saveFields(data),
              ),
              const SizedBox(height: 24),
            ],
          );
        },
      ),
    );
  }
}

Future<bool?> _confirmRemove(BuildContext context, Workspace workspace) {
  return showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text('Remove "${workspace.name}"?'),
      content: const Text(
        'The notes are deleted from this device. Anything not pushed will be '
        'lost.',
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, true),
          child: const Text('Remove'),
        ),
      ],
    ),
  );
}

/// The commit author, shared by every workspace.
class _CommitIdentityFields extends StatelessWidget {
  const _CommitIdentityFields({
    required this.authorName,
    required this.authorEmail,
    required this.onSave,
  });

  final TextEditingController authorName;
  final TextEditingController authorEmail;
  final VoidCallback onSave;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const SettingsSection('Commits'),
        Text(
          'Used for every workspace.',
          style: Theme.of(context).textTheme.bodySmall,
        ),
        TextField(
          controller: authorName,
          decoration: const InputDecoration(labelText: 'Author name'),
        ),
        TextField(
          controller: authorEmail,
          decoration: const InputDecoration(labelText: 'Author email'),
          keyboardType: TextInputType.emailAddress,
        ),
        const SizedBox(height: 8),
        Align(
          alignment: Alignment.centerLeft,
          child: OutlinedButton(onPressed: onSave, child: const Text('Save')),
        ),
      ],
    );
  }
}

/// The note that quick posts go to.
class _InboxNoteField extends StatelessWidget {
  const _InboxNoteField({
    required this.controller,
    required this.errorText,
    required this.onChanged,
    required this.onSave,
  });

  final TextEditingController controller;
  final String? errorText;
  final VoidCallback onChanged;
  final VoidCallback onSave;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const SettingsSection('Inbox'),
        Text(
          'The note that quick posts from the Inbox button, the launcher '
          'shortcut and text shared from other apps are appended to. It '
          'is hidden from the notes list; open it from the Inbox sheet.',
          style: Theme.of(context).textTheme.bodySmall,
        ),
        TextField(
          controller: controller,
          decoration: InputDecoration(
            labelText: 'Inbox note',
            hintText: Settings.defaultInboxNoteName,
            errorText: errorText,
          ),
          onChanged: (_) => onChanged(),
          onSubmitted: (_) => onSave(),
        ),
        const SizedBox(height: 8),
        Align(
          alignment: Alignment.centerLeft,
          child: OutlinedButton(onPressed: onSave, child: const Text('Save')),
        ),
      ],
    );
  }
}
