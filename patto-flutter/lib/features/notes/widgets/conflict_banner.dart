import 'package:flutter/material.dart';

import '../../../src/rust/api/conflict.dart';
import '../../conflicts/conflict_list_screen.dart';

/// A standing reminder that a sync is paused, until the merge is done.
class ConflictBanner extends StatelessWidget {
  const ConflictBanner({super.key, required this.pending});

  final PendingConflict pending;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final n = pending.files.length;
    return Material(
      color: Colors.amber.withValues(alpha: 0.18),
      child: InkWell(
        onTap: () => ConflictListScreen.open(context),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
          child: Row(
            children: [
              Icon(Icons.warning_amber_rounded, color: Colors.amber.shade800),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  n == 0
                      ? 'A sync is paused. Sync again to finish.'
                      : '$n ${n == 1 ? 'note is' : 'notes are'} waiting to '
                            'be merged',
                  style: theme.textTheme.bodyMedium,
                ),
              ),
              const Icon(Icons.chevron_right),
            ],
          ),
        ),
      ),
    );
  }
}
