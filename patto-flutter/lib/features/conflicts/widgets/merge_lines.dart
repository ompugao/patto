import 'package:flutter/material.dart';

import '../conflict_text.dart';

class MergeLine extends StatelessWidget {
  const MergeLine(this.text, {super.key, this.style, this.span});

  final String text;
  final TextStyle? style;
  final InlineSpan? span;

  @override
  Widget build(BuildContext context) {
    final base = Theme.of(context).textTheme.bodyMedium!.merge(style);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 1),
      child: span != null
          ? Text.rich(span!, style: base)
          : Text(text.isEmpty ? ' ' : displayLine(text), style: base),
    );
  }
}

/// A run of lines neither side touched, collapsed to its edges until tapped.
class UnchangedLines extends StatelessWidget {
  const UnchangedLines({
    super.key,
    required this.lines,
    required this.expanded,
    required this.onExpand,
  });

  final List<String> lines;
  final bool expanded;
  final VoidCallback onExpand;

  /// Lines of context kept on each side of a collapsed run.
  static const _context = 2;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final faded = TextStyle(color: theme.colorScheme.onSurfaceVariant);

    if (expanded || lines.length <= _context * 2 + 1) {
      return Padding(
        padding: const EdgeInsets.only(left: 8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [for (final l in lines) MergeLine(l, style: faded)],
        ),
      );
    }

    final hidden = lines.length - _context * 2;
    return Padding(
      padding: const EdgeInsets.only(left: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          for (final l in lines.take(_context)) MergeLine(l, style: faded),
          InkWell(
            onTap: onExpand,
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: Text(
                '⋯ $hidden unchanged lines',
                style: theme.textTheme.labelMedium?.copyWith(
                  color: theme.colorScheme.primary,
                ),
              ),
            ),
          ),
          for (final l in lines.skip(lines.length - _context))
            MergeLine(l, style: faded),
        ],
      ),
    );
  }
}

enum MergeSide { ours, theirs, both }

extension MergeSideStyle on MergeSide {
  String get label => switch (this) {
    MergeSide.ours => 'you',
    MergeSide.theirs => 'remote',
    MergeSide.both => 'both',
  };

  Color color(ColorScheme scheme) => switch (this) {
    MergeSide.ours => scheme.primary,
    MergeSide.theirs => scheme.tertiary,
    MergeSide.both => scheme.outline,
  };
}

/// Lines only one side changed, already merged in. The tag takes the change
/// back, or puts it in again.
class OneSidedLines extends StatelessWidget {
  const OneSidedLines({
    super.key,
    required this.side,
    required this.base,
    required this.lines,
    required this.undone,
    this.onToggle,
  });

  final MergeSide side;
  final List<String> base;
  final List<String> lines;
  final bool undone;
  final VoidCallback? onToggle;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = side.color(theme.colorScheme);
    final deleted = lines.isEmpty;

    final shown = undone || deleted ? base : lines;
    final struck = !undone && deleted;

    return Container(
      margin: const EdgeInsets.symmetric(vertical: 2),
      padding: const EdgeInsets.only(left: 6),
      decoration: BoxDecoration(
        border: Border(
          left: BorderSide(
            color: undone ? theme.colorScheme.outlineVariant : color,
            width: 3,
          ),
        ),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                for (final l in shown)
                  MergeLine(
                    l,
                    style: struck
                        ? TextStyle(
                            decoration: TextDecoration.lineThrough,
                            color: theme.colorScheme.onSurfaceVariant,
                          )
                        : null,
                  ),
              ],
            ),
          ),
          InkWell(
            onTap: onToggle,
            borderRadius: BorderRadius.circular(8),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
              child: Text(
                undone
                    ? '${side.label} · undone'
                    : deleted
                    ? '${side.label} deleted'
                    : side.label,
                style: theme.textTheme.labelSmall?.copyWith(
                  color: undone ? theme.colorScheme.outline : color,
                  decoration: undone ? TextDecoration.lineThrough : null,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// [line] with the part that differs from [other] highlighted.
InlineSpan highlightedDiff(String line, String other, Color highlight) {
  final a = displayLine(line);
  final range = changedRange(a, displayLine(other));
  final changed = a.substring(range.start, range.end);
  return TextSpan(
    children: [
      TextSpan(text: a.substring(0, range.start)),
      if (changed.isNotEmpty)
        TextSpan(
          text: changed,
          style: TextStyle(backgroundColor: highlight),
        ),
      TextSpan(text: a.substring(range.end)),
    ],
  );
}

/// One side of an unsettled conflict, with its differences from the other side
/// highlighted when the lines pair up.
class SideLines extends StatelessWidget {
  const SideLines({
    super.key,
    required this.side,
    required this.lines,
    required this.other,
  });

  final MergeSide side;
  final List<String> lines;
  final List<String> other;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = side.color(theme.colorScheme);
    final paired = lines.length == other.length;

    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            side == MergeSide.ours ? 'You' : 'Remote',
            style: theme.textTheme.labelSmall?.copyWith(color: color),
          ),
          Container(
            padding: const EdgeInsets.only(left: 6),
            decoration: BoxDecoration(
              border: Border(left: BorderSide(color: color, width: 3)),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (lines.isEmpty)
                  Text(
                    '(deleted these lines)',
                    style: theme.textTheme.bodySmall?.copyWith(
                      fontStyle: FontStyle.italic,
                    ),
                  ),
                for (var k = 0; k < lines.length; k++)
                  MergeLine(
                    lines[k],
                    span: paired
                        ? highlightedDiff(
                            lines[k],
                            other[k],
                            color.withValues(alpha: 0.25),
                          )
                        : null,
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
