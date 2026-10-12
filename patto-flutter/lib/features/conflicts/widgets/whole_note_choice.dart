import 'package:flutter/material.dart';

import '../../../src/rust/api/conflict.dart';
import '../conflict_state.dart';
import '../conflict_text.dart';

/// A note one side deleted and the other edited: keep it or not.
class WholeNoteChoice extends StatelessWidget {
  const WholeNoteChoice({
    super.key,
    required this.detail,
    required this.choice,
    required this.onChoose,
  });

  final ConflictDetail detail;
  final Choice? choice;
  final ValueChanged<Choice?> onChoose;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final weDeleted = detail.kind == ConflictKind.deletedByUs;
    final kept = weDeleted ? detail.theirs : detail.ours;

    // Pick.ours keeps our side, which for "we deleted" means staying deleted.
    final keepPick = weDeleted ? Pick.theirs : Pick.ours;
    final deletePick = weDeleted ? Pick.ours : Pick.theirs;

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text(
          weDeleted
              ? 'You deleted this note, and the remote edited it.'
              : 'The remote deleted this note, and you edited it.',
          style: theme.textTheme.titleMedium,
        ),
        const SizedBox(height: 16),
        SegmentedButton<Pick>(
          emptySelectionAllowed: true,
          segments: [
            ButtonSegment(
              value: keepPick,
              icon: const Icon(Icons.note_outlined),
              label: Text(weDeleted ? 'Restore it' : 'Keep it'),
            ),
            ButtonSegment(
              value: deletePick,
              icon: const Icon(Icons.delete_outline),
              label: const Text('Delete it'),
            ),
          ],
          selected: {?choice?.pick},
          onSelectionChanged: (s) =>
              onChoose(s.isEmpty ? null : Choice(s.first)),
        ),
        const SizedBox(height: 24),
        Text(
          weDeleted ? 'The remote\'s version' : 'Your version',
          style: theme.textTheme.labelLarge,
        ),
        const SizedBox(height: 8),
        SelectableText(displayLine(kept ?? '')),
      ],
    );
  }
}
