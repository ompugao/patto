import 'package:flutter/material.dart';

import '../../../src/rust/api/merge.dart';
import '../conflict_state.dart';
import 'conflict_dialogs.dart';
import 'merge_lines.dart';

/// A place both sides changed: both versions until a choice is made, then
/// the chosen lines with the sides a tap away.
class ConflictCard extends StatefulWidget {
  const ConflictCard({
    super.key,
    required this.region,
    required this.choice,
    required this.onChoose,
    required this.onEdit,
  });

  final MergeRegion_Conflict region;
  final Choice? choice;
  final ValueChanged<Choice?> onChoose;
  final VoidCallback onEdit;

  @override
  State<ConflictCard> createState() => _ConflictCardState();
}

class _ConflictCardState extends State<ConflictCard> {
  /// Show both sides again after a choice was made.
  bool _compare = false;

  void _pick(Pick pick) {
    widget.onChoose(widget.choice?.pick == pick ? null : Choice(pick));
  }

  String _caption(Choice choice) => switch (choice.pick) {
    Pick.ours => 'Using yours',
    Pick.theirs => 'Using remote',
    Pick.suggested =>
      widget.region.suggestion?.kind == SuggestionKind.both
          ? '✦ Both, yours first'
          : '✦ Combined',
    Pick.both => 'Both, yours first',
    Pick.custom => 'Edited by hand',
  };

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final region = widget.region;
    final choice = widget.choice;
    final suggestion = region.suggestion;
    final settled = choice != null;

    Widget chip(String label, Pick pick) => ChoiceChip(
      label: Text(label),
      selected: choice?.pick == pick,
      onSelected: (_) => _pick(pick),
      visualDensity: VisualDensity.compact,
    );

    return GestureDetector(
      onLongPress: () => showBaseLinesDialog(context, region.base),
      // Swipe right to keep yours, left to take the remote's.
      onHorizontalDragEnd: (details) {
        final v = details.primaryVelocity ?? 0;
        if (v > 300) widget.onChoose(const Choice(Pick.ours));
        if (v < -300) widget.onChoose(const Choice(Pick.theirs));
      },
      child: Card(
        margin: const EdgeInsets.symmetric(vertical: 6),
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
          side: BorderSide(
            color: settled
                ? theme.colorScheme.outlineVariant
                : Colors.amber.shade700,
            width: settled ? 1 : 2,
          ),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(12, 10, 12, 8),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (!settled) ...[
                Row(
                  children: [
                    Icon(
                      Icons.warning_amber_rounded,
                      size: 16,
                      color: Colors.amber.shade800,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      'Both changed this',
                      style: theme.textTheme.labelMedium,
                    ),
                  ],
                ),
                const SizedBox(height: 6),
              ],
              if (!settled || _compare) ...[
                SideLines(
                  side: MergeSide.ours,
                  lines: region.ours,
                  other: region.theirs,
                ),
                SideLines(
                  side: MergeSide.theirs,
                  lines: region.theirs,
                  other: region.ours,
                ),
              ],
              if (settled) ...[
                InkWell(
                  onTap: () => setState(() => _compare = !_compare),
                  child: Row(
                    children: [
                      Text(
                        _caption(choice),
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: theme.colorScheme.primary,
                        ),
                      ),
                      const Spacer(),
                      Text(
                        _compare ? 'hide sides' : 'compare',
                        style: theme.textTheme.labelSmall,
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 2),
                for (final l in linesFor(region, choice)) MergeLine(l),
                if (linesFor(region, choice).isEmpty)
                  Text(
                    '(no lines)',
                    style: theme.textTheme.bodySmall?.copyWith(
                      fontStyle: FontStyle.italic,
                    ),
                  ),
              ],
              const SizedBox(height: 6),
              Wrap(
                spacing: 6,
                runSpacing: 4,
                children: [
                  chip('You', Pick.ours),
                  chip('Remote', Pick.theirs),
                  if (suggestion?.kind == SuggestionKind.combined)
                    chip('✦ Combined', Pick.suggested),
                  if (suggestion?.kind == SuggestionKind.both)
                    chip('✦ Both', Pick.suggested)
                  else
                    chip('Both', Pick.both),
                  ChoiceChip(
                    avatar: const Icon(Icons.edit, size: 16),
                    label: const Text('Edit'),
                    selected: choice?.pick == Pick.custom,
                    onSelected: (_) => widget.onEdit(),
                    visualDensity: VisualDensity.compact,
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
