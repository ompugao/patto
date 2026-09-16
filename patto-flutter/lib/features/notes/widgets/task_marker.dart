import 'package:flutter/material.dart';

import '../../../src/rust/api/types.dart';

/// Status icon shown at the start of a task line.
class TaskMarker extends StatelessWidget {
  const TaskMarker({
    super.key,
    required this.status,
    this.onTap,
    this.textScale = 1.0,
  });

  final TaskStatus status;
  final VoidCallback? onTap;

  /// Keeps the marker in proportion with the note text size.
  final double textScale;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final (icon, color) = switch (status) {
      TaskStatus.todo => (Icons.radio_button_unchecked, colors.outline),
      TaskStatus.doing => (Icons.play_circle_outline, colors.primary),
      TaskStatus.paused => (Icons.pause_circle_outline, colors.tertiary),
      TaskStatus.done => (Icons.check_circle, Colors.green.shade600),
    };

    final marker = Icon(icon, size: 18 * textScale, color: color);
    if (onTap == null) return marker;

    return InkResponse(
      onTap: onTap,
      radius: 18,
      child: Padding(padding: const EdgeInsets.all(2), child: marker),
    );
  }
}

/// Due-date chip. Red once overdue, amber within a week, plain after that.
class DueChip extends StatelessWidget {
  const DueChip({super.key, required this.due, this.now, this.textScale = 1.0});

  final TaskDate due;
  final DateTime? now;
  final double textScale;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final today = now ?? DateTime.now();
    final date = DateTime.tryParse(due.text);

    Color background = theme.colorScheme.surfaceContainerHighest;
    Color foreground = theme.colorScheme.onSurfaceVariant;

    if (date != null && due.kind != DateKind.unparsed) {
      final days = DateTime(date.year, date.month, date.day)
          .difference(DateTime(today.year, today.month, today.day))
          .inDays;
      if (days < 0) {
        background = theme.colorScheme.errorContainer;
        foreground = theme.colorScheme.onErrorContainer;
      } else if (days <= 7) {
        background = const Color(0xFFFFF0C2);
        foreground = const Color(0xFF6B4E00);
      }
    }

    return Container(
      margin: const EdgeInsets.only(left: 6),
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
      decoration: BoxDecoration(
        color: background,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        due.text,
        style: theme.textTheme.labelSmall?.copyWith(
          color: foreground,
          fontSize: (theme.textTheme.labelSmall?.fontSize ?? 11) * textScale,
        ),
      ),
    );
  }
}
