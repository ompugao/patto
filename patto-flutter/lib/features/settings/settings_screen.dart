import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers.dart';
import '../../core/settings.dart';
import '../../core/workspace.dart';
import 'workspace_editor.dart';

class SettingsScreen extends ConsumerStatefulWidget {
  const SettingsScreen({super.key});

  static Future<void> open(BuildContext context) {
    return Navigator.of(context).push(
      MaterialPageRoute<void>(builder: (_) => const SettingsScreen()),
    );
  }

  @override
  ConsumerState<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends ConsumerState<SettingsScreen> {
  final _authorName = TextEditingController();
  final _authorEmail = TextEditingController();
  bool _filled = false;

  @override
  void dispose() {
    _authorName.dispose();
    _authorEmail.dispose();
    super.dispose();
  }

  void _fill(Settings settings) {
    if (_filled) return;
    _filled = true;
    _authorName.text = settings.authorName;
    _authorEmail.text = settings.authorEmail;
  }

  Settings _collect(Settings base) => base.copyWith(
    authorName: _authorName.text.trim(),
    authorEmail: _authorEmail.text.trim(),
  );

  Future<void> _saveAuthor(Settings base) async {
    await ref.read(settingsProvider.notifier).save(_collect(base));
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('Saved')),
    );
  }

  Future<void> _switchTo(Workspace workspace) async {
    await ref.read(settingsProvider.notifier).setActiveWorkspace(workspace.id);
  }

  Future<void> _delete(Workspace workspace) async {
    final confirmed = await showDialog<bool>(
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
              _Section('Workspaces'),
              for (final workspace in data.workspaces)
                _WorkspaceTile(
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
              _Section('Commits'),
              Text(
                'Used for every workspace.',
                style: Theme.of(context).textTheme.bodySmall,
              ),
              TextField(
                controller: _authorName,
                decoration: const InputDecoration(labelText: 'Author name'),
              ),
              TextField(
                controller: _authorEmail,
                decoration: const InputDecoration(labelText: 'Author email'),
                keyboardType: TextInputType.emailAddress,
              ),
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerLeft,
                child: OutlinedButton(
                  onPressed: () => _saveAuthor(data),
                  child: const Text('Save'),
                ),
              ),
              const SizedBox(height: 24),
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
              const SizedBox(height: 16),
              _FontSizeSetting(
                scale: data.fontScale,
                onChanged: (scale) => ref
                    .read(settingsProvider.notifier)
                    .save(_collect(data).copyWith(fontScale: scale)),
              ),
              const SizedBox(height: 24),
            ],
          );
        },
      ),
    );
  }
}

class _WorkspaceTile extends StatelessWidget {
  const _WorkspaceTile({
    required this.workspace,
    required this.isActive,
    required this.onSwitch,
    required this.onEdit,
    required this.onDelete,
  });

  final Workspace workspace;
  final bool isActive;
  final VoidCallback onSwitch;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: Icon(
        isActive ? Icons.folder : Icons.folder_outlined,
        color: isActive ? theme.colorScheme.primary : null,
      ),
      title: Text(workspace.name),
      subtitle: Text(
        workspace.hasRemote ? workspace.repoUrl : 'No repository',
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      onTap: isActive ? null : onSwitch,
      trailing: PopupMenuButton<String>(
        onSelected: (choice) => switch (choice) {
          'edit' => onEdit(),
          'delete' => onDelete(),
          _ => null,
        },
        itemBuilder: (context) => const [
          PopupMenuItem(value: 'edit', child: Text('Edit')),
          PopupMenuItem(value: 'delete', child: Text('Remove')),
        ],
      ),
    );
  }
}

/// Note text size, with a sample so the effect is visible before leaving the
/// screen. The slider is debounced: dragging it writes on release, not on every
/// frame.
class _FontSizeSetting extends StatefulWidget {
  const _FontSizeSetting({required this.scale, required this.onChanged});

  final double scale;
  final void Function(double) onChanged;

  @override
  State<_FontSizeSetting> createState() => _FontSizeSettingState();
}

class _FontSizeSettingState extends State<_FontSizeSetting> {
  double? _dragging;

  double get _value => _dragging ?? widget.scale;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final body = theme.textTheme.bodyLarge!;
    final sample = body.copyWith(fontSize: (body.fontSize ?? 16) * _value);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text('Note text size', style: theme.textTheme.bodyMedium),
            const Spacer(),
            Text(
              '${(_value * 100).round()}%',
              style: theme.textTheme.labelMedium,
            ),
          ],
        ),
        Row(
          children: [
            const Icon(Icons.text_fields, size: 16),
            Expanded(
              child: Slider(
                value: _value,
                min: Settings.minFontScale,
                max: Settings.maxFontScale,
                // 10% steps: fine enough to tune, coarse enough to land on.
                divisions:
                    ((Settings.maxFontScale - Settings.minFontScale) * 10)
                        .round(),
                label: '${(_value * 100).round()}%',
                onChanged: (v) => setState(() => _dragging = v),
                onChangeEnd: (v) {
                  setState(() => _dragging = null);
                  widget.onChanged(v);
                },
              ),
            ),
            const Icon(Icons.text_fields, size: 24),
          ],
        ),
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: theme.colorScheme.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(8),
          ),
          child: Text(
            'A nested note line with [a link] and a task.',
            style: sample,
          ),
        ),
      ],
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
