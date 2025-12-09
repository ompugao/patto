import 'package:flutter/material.dart';

import '../../domain/note_info.dart';

/// Sort selector widget
class SortSelector extends StatelessWidget {
  final SortOption value;
  final ValueChanged<SortOption> onChanged;

  const SortSelector({
    super.key,
    required this.value,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Text(
          'Sort by:',
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
        ),
        const SizedBox(width: 8),
        SegmentedButton<SortOption>(
          segments: SortOption.values
              .map(
                (option) => ButtonSegment(
                  value: option,
                  label: Text(option.label),
                ),
              )
              .toList(),
          selected: {value},
          onSelectionChanged: (selection) => onChanged(selection.first),
          style: SegmentedButton.styleFrom(
            visualDensity: VisualDensity.compact,
          ),
        ),
      ],
    );
  }
}
