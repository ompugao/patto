import 'package:flutter/material.dart';

import '../../../src/rust/api/types.dart';

/// Notes matching the link being typed, as chips that complete it.
class LinkCandidateBar extends StatelessWidget {
  const LinkCandidateBar({
    super.key,
    required this.candidates,
    required this.query,
    required this.onPick,
  });

  final List<NoteMeta> candidates;
  final String query;
  final void Function(String name) onPick;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final offerNew =
        query.isNotEmpty && !candidates.any((c) => c.name == query);
    return Container(
      height: 44,
      color: theme.colorScheme.surfaceContainerHighest,
      child: ListView(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 8),
        children: [
          for (final c in candidates)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 6),
              child: ActionChip(
                label: Text(c.name),
                onPressed: () => onPick(c.name),
              ),
            ),
          // Links to a note that does not exist yet are how new notes start.
          if (offerNew)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 6),
              child: ActionChip(
                avatar: const Icon(Icons.add, size: 16),
                label: Text(query),
                onPressed: () => onPick(query),
              ),
            ),
        ],
      ),
    );
  }
}
