import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// The other way out of a paused sync: the git commands that merge the side
/// branch on a desktop.
class FinishOnDesktopSheet extends StatelessWidget {
  const FinishOnDesktopSheet({super.key, required this.branch});

  final String branch;

  static void show(BuildContext context, String branch) {
    showModalBottomSheet<void>(
      context: context,
      showDragHandle: true,
      builder: (_) => FinishOnDesktopSheet(branch: branch),
    );
  }

  @override
  Widget build(BuildContext context) {
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
  }
}
