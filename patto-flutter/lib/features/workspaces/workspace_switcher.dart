import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers.dart';
import '../settings/workspace_editor.dart';

/// Pick a workspace, or add one.
class WorkspaceSwitcher extends ConsumerWidget {
  const WorkspaceSwitcher({super.key});

  static Future<void> show(BuildContext context) {
    return showModalBottomSheet<void>(
      context: context,
      builder: (_) => const WorkspaceSwitcher(),
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final workspaces = ref.watch(workspacesProvider);
    final activeId = ref.watch(workspaceProvider).value?.config.id;

    return SafeArea(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
            child: Text('Workspaces', style: theme.textTheme.titleMedium),
          ),
          for (final workspace in workspaces)
            ListTile(
              leading: Icon(
                workspace.id == activeId
                    ? Icons.folder
                    : Icons.folder_outlined,
                color: workspace.id == activeId
                    ? theme.colorScheme.primary
                    : null,
              ),
              title: Text(workspace.name),
              subtitle: workspace.hasRemote
                  ? Text(
                      workspace.repoUrl,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    )
                  : null,
              trailing: workspace.id == activeId
                  ? const Icon(Icons.check)
                  : null,
              onTap: () async {
                Navigator.pop(context);
                await ref
                    .read(settingsProvider.notifier)
                    .setActiveWorkspace(workspace.id);
              },
            ),
          const Divider(height: 1),
          ListTile(
            leading: const Icon(Icons.add),
            title: const Text('Add workspace'),
            onTap: () {
              Navigator.pop(context);
              WorkspaceEditorScreen.open(context);
            },
          ),
        ],
      ),
    );
  }
}
