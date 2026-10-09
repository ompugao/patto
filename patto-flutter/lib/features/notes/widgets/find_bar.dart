import 'package:flutter/material.dart';

/// The find field under the app bar, with the match count and arrows.
class FindBar extends StatelessWidget implements PreferredSizeWidget {
  const FindBar({
    super.key,
    required this.controller,
    required this.countLabel,
    required this.hasHits,
    required this.onChanged,
    required this.onStep,
  });

  final TextEditingController controller;
  final String countLabel;
  final bool hasHits;
  final ValueChanged<String> onChanged;
  final ValueChanged<int> onStep;

  @override
  Size get preferredSize => const Size.fromHeight(52);

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 0, 4, 8),
      child: Row(
        children: [
          Expanded(
            child: TextField(
              controller: controller,
              autofocus: true,
              onChanged: onChanged,
              onSubmitted: (_) => onStep(1),
              textInputAction: TextInputAction.search,
              decoration: const InputDecoration(
                hintText: 'Find in note',
                isDense: true,
                prefixIcon: Icon(Icons.search),
                border: OutlineInputBorder(),
              ),
            ),
          ),
          const SizedBox(width: 8),
          Text(countLabel, style: theme.textTheme.labelMedium),
          IconButton(
            icon: const Icon(Icons.keyboard_arrow_up),
            tooltip: 'Previous match',
            onPressed: hasHits ? () => onStep(-1) : null,
          ),
          IconButton(
            icon: const Icon(Icons.keyboard_arrow_down),
            tooltip: 'Next match',
            onPressed: hasHits ? () => onStep(1) : null,
          ),
        ],
      ),
    );
  }
}
