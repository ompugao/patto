import 'package:flutter/material.dart';
import 'package:flutter_math_fork/flutter_math.dart';

import '../../../src/rust/api/types.dart';
import 'spans_text.dart';

/// A `[@math]` block in display style; the source when it does not parse.
class MathBlock extends StatelessWidget {
  const MathBlock({super.key, required this.tex, required this.textScale});

  final String tex;
  final double textScale;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.symmetric(vertical: 6),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Math.tex(
          tex,
          mathStyle: MathStyle.display,
          textStyle: theme.textTheme.bodyLarge?.copyWith(
            fontSize: (theme.textTheme.bodyLarge?.fontSize ?? 16) * textScale,
          ),
          onErrorFallback: (_) => Text(
            tex,
            style: theme.textTheme.bodyMedium?.copyWith(
              fontFamily: 'monospace',
              fontSize:
                  (theme.textTheme.bodyMedium?.fontSize ?? 14) * textScale,
            ),
          ),
        ),
      ),
    );
  }
}

/// A `[@table]` block, padded to its widest row.
class TableBlock extends StatelessWidget {
  const TableBlock({
    super.key,
    required this.caption,
    required this.rows,
    required this.actions,
    required this.root,
    required this.textScale,
  });

  final String? caption;
  final List<NoteTableRow> rows;
  final SpanActions actions;
  final String? root;
  final double textScale;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final columns = rows.fold<int>(
      0,
      (n, r) => r.cells.length > n ? r.cells.length : n,
    );

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (caption != null)
          Padding(
            padding: const EdgeInsets.only(bottom: 4),
            child: Text(caption!, style: theme.textTheme.labelMedium),
          ),
        SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: Table(
            defaultColumnWidth: const IntrinsicColumnWidth(),
            border: TableBorder.all(color: theme.colorScheme.outlineVariant),
            children: [
              for (final row in rows)
                TableRow(
                  children: [
                    for (var i = 0; i < columns; i++)
                      Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 8,
                          vertical: 4,
                        ),
                        child: i < row.cells.length
                            ? SpansText(
                                spans: row.cells[i].spans,
                                actions: actions,
                                noteRoot: root,
                                style: theme.textTheme.bodyMedium?.copyWith(
                                  fontSize:
                                      (theme.textTheme.bodyMedium?.fontSize ??
                                          14) *
                                      textScale,
                                ),
                              )
                            : const SizedBox.shrink(),
                      ),
                  ],
                ),
            ],
          ),
        ),
      ],
    );
  }
}
