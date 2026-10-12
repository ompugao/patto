import 'package:flutter/material.dart';

/// Marks where the conflicts are along the note: amber for waiting, grey for
/// settled. Tapping one jumps to it.
class ConflictStrip extends StatelessWidget {
  const ConflictStrip({
    super.key,
    required this.regionCount,
    required this.conflicts,
    required this.resolved,
    required this.onTap,
  });

  final int regionCount;
  final List<int> conflicts;
  final Set<int> resolved;
  final ValueChanged<int> onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return LayoutBuilder(
      builder: (context, constraints) {
        final height = constraints.maxHeight;
        return Stack(
          children: [
            for (final i in conflicts)
              Positioned(
                top: regionCount <= 1
                    ? 0
                    : (height - 14) * i / (regionCount - 1),
                left: 0,
                right: 0,
                child: GestureDetector(
                  onTap: () => onTap(i),
                  child: Container(
                    height: 14,
                    decoration: BoxDecoration(
                      color: resolved.contains(i)
                          ? theme.colorScheme.outlineVariant
                          : Colors.amber.shade700,
                      borderRadius: BorderRadius.circular(3),
                    ),
                  ),
                ),
              ),
          ],
        );
      },
    );
  }
}
