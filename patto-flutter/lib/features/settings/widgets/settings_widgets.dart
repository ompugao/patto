import 'package:flutter/material.dart';

import '../../../core/workspace.dart';

class SettingsSection extends StatelessWidget {
  const SettingsSection(this.title, {super.key});

  final String title;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Text(title, style: Theme.of(context).textTheme.titleSmall),
    );
  }
}

class WorkspaceTile extends StatelessWidget {
  const WorkspaceTile({
    super.key,
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
