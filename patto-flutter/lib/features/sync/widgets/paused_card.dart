import 'package:flutter/material.dart';

import '../../conflicts/conflict_list_screen.dart';

/// Says the sync stopped short of merging, why that is safe, and where to go.
class PausedCard extends StatelessWidget {
  const PausedCard({super.key, this.sideBranch, this.count});

  /// Set right after the sync that paused; otherwise the pause is older.
  final String? sideBranch;
  final int? count;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final n = count;
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Colors.amber.withValues(alpha: 0.18),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(Icons.warning_amber_rounded, color: Colors.amber.shade800),
              const SizedBox(width: 8),
              Text('Sync paused', style: theme.textTheme.titleSmall),
            ],
          ),
          const SizedBox(height: 6),
          Text(
            [
              n == null
                  ? 'Some notes changed both here and on another device.'
                  : '$n ${n == 1 ? 'note' : 'notes'} changed both here and '
                        'on another device.',
              sideBranch == null
                  ? 'Your edits are safe on the remote.'
                  : 'Your edits are safe: they were pushed to $sideBranch.',
              'Resolve them here, or merge that branch on your desktop.',
            ].join(' '),
          ),
          Align(
            alignment: Alignment.centerRight,
            child: TextButton(
              onPressed: () {
                // The sheet's context is gone once it is popped.
                final navigator = Navigator.of(context);
                navigator.pop();
                navigator.push(
                  MaterialPageRoute<void>(
                    builder: (_) => const ConflictListScreen(),
                  ),
                );
              },
              child: const Text('Review conflicts'),
            ),
          ),
        ],
      ),
    );
  }
}
