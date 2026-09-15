import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_math_fork/flutter_math.dart';
import 'package:re_highlight/languages/all.dart';
import 'package:re_highlight/re_highlight.dart';
import 'package:re_highlight/styles/atom-one-dark.dart';
import 'package:re_highlight/styles/atom-one-light.dart';

import '../../../src/rust/api/types.dart';
import 'note_image.dart';
import 'spans_text.dart';
import 'task_marker.dart';

/// One element of a note. Kept free of providers: the list rebuilds these
/// constantly while scrolling.
class BlockWidget extends StatelessWidget {
  const BlockWidget({
    super.key,
    required this.block,
    required this.actions,
    required this.root,
    this.textScale = 1.0,
    this.highlighted = false,
    this.onTaskTap,
    this.onLongPress,
  });

  static const indentPerLevel = 18.0;

  final Block block;
  final SpanActions actions;
  final String? root;

  /// Multiplier from the appearance setting, applied to every size this block
  /// draws so code, math and tables grow with the prose.
  final double textScale;
  final bool highlighted;
  final void Function(Block block)? onTaskTap;
  final void Function(Block block)? onLongPress;

  TextStyle _bodyStyle(ThemeData theme) {
    final base = theme.textTheme.bodyLarge!;
    return base.copyWith(fontSize: (base.fontSize ?? 16) * textScale);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final quoted = block.quoteDepth > 0;

    Widget content = switch (block.kind) {
      BlockKind_Blank() => SizedBox(height: 10 * textScale),
      BlockKind_Rule() => const Divider(height: 20),
      BlockKind_Line(:final spans) => _line(context, spans),
      BlockKind_Code(:final lang, :final lines) =>
        _CodeBlock(lang: lang, lines: lines, textScale: textScale),
      BlockKind_Math(:final tex) => _MathBlock(tex: tex, textScale: textScale),
      BlockKind_Table(:final caption, :final rows) => _TableBlock(
        caption: caption,
        rows: rows,
        actions: actions,
        root: root,
        textScale: textScale,
      ),
      BlockKind_Images(:final images) => _images(context, images),
    };

    if (quoted) {
      content = Container(
        margin: EdgeInsets.only(left: (block.quoteDepth - 1) * 10.0),
        padding: const EdgeInsets.only(left: 10),
        decoration: BoxDecoration(
          border: Border(
            left: BorderSide(color: theme.colorScheme.outlineVariant, width: 3),
          ),
        ),
        child: DefaultTextStyle.merge(
          style: TextStyle(color: theme.colorScheme.onSurfaceVariant),
          child: content,
        ),
      );
    }

    final body = Container(
      width: double.infinity,
      color: highlighted ? theme.colorScheme.primaryContainer : null,
      padding: EdgeInsets.fromLTRB(
        16 + block.depth * indentPerLevel,
        2,
        16,
        2,
      ),
      child: content,
    );

    if (onLongPress == null) return body;
    return InkWell(onLongPress: () => onLongPress!(block), child: body);
  }

  Widget _line(BuildContext context, List<NoteSpan> spans) {
    final theme = Theme.of(context);
    final task = block.task;
    final done = task?.status == TaskStatus.done;

    final text = SpansText(
      spans: spans,
      actions: actions,
      noteRoot: root,
      style: _bodyStyle(theme),
      strikeThrough: done,
      trailing: task?.due != null && !done
          ? WidgetSpan(
              alignment: PlaceholderAlignment.middle,
              child: DueChip(due: task!.due!, textScale: textScale),
            )
          : null,
    );

    final anchors = block.anchors;
    final line = anchors.isEmpty
        ? text
        : Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              text,
              Padding(
                padding: const EdgeInsets.only(top: 2),
                child: Text(
                  anchors.map((a) => '#$a').join(' '),
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: theme.colorScheme.outline,
                    fontSize:
                        (theme.textTheme.labelSmall?.fontSize ?? 11) * textScale,
                  ),
                ),
              ),
            ],
          );

    if (task == null && block.depth == 0) return line;

    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (task != null) ...[
          Padding(
            padding: const EdgeInsets.only(top: 3, right: 6),
            child: TaskMarker(
              status: task.status,
              textScale: textScale,
              onTap: onTaskTap == null ? null : () => onTaskTap!(block),
            ),
          ),
        ] else if (block.depth > 0) ...[
          Padding(
            padding: const EdgeInsets.only(top: 8, right: 8),
            child: Container(
              width: 4,
              height: 4,
              decoration: BoxDecoration(
                color: theme.colorScheme.outlineVariant,
                shape: BoxShape.circle,
              ),
            ),
          ),
        ],
        Expanded(child: line),
      ],
    );
  }

  Widget _images(BuildContext context, List<ImageRef> images) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        for (final image in images)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 4),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                GestureDetector(
                  onTap: () => ImageLightbox.open(context, image, root),
                  child: NoteImage(image: image, root: root),
                ),
                if (image.alt != null)
                  Padding(
                    padding: const EdgeInsets.only(top: 4),
                    child: Text(
                      image.alt!,
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  ),
              ],
            ),
          ),
      ],
    );
  }
}

class _CodeBlock extends StatefulWidget {
  const _CodeBlock({
    required this.lang,
    required this.lines,
    required this.textScale,
  });

  final String lang;
  final List<String> lines;
  final double textScale;

  @override
  State<_CodeBlock> createState() => _CodeBlockState();
}

class _CodeBlockState extends State<_CodeBlock> {
  /// Highlighting a very large block costs more than it is worth on a phone.
  static const _highlightLimit = 20000;

  late final String _text = widget.lines.join('\n');
  HighlightResult? _result;
  Brightness? _renderedFor;

  void _highlight() {
    if (_text.length > _highlightLimit) return;
    final language = builtinAllLanguages[widget.lang];
    if (language == null) return;

    final highlight = Highlight()..registerLanguage(widget.lang, language);
    try {
      _result = highlight.highlight(code: _text, language: widget.lang);
    } catch (_) {
      _result = null;
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final dark = theme.brightness == Brightness.dark;
    if (_renderedFor != theme.brightness) {
      _renderedFor = theme.brightness;
      if (_result == null) _highlight();
    }

    final mono = TextStyle(
      fontFamily: 'monospace',
      fontFamilyFallback: const ['Roboto Mono', 'Noto Sans Mono CJK JP'],
      fontSize: 13 * widget.textScale,
      height: 1.4,
      color: theme.colorScheme.onSurface,
    );

    final renderer = TextSpanRenderer(
      mono,
      dark ? atomOneDarkTheme : atomOneLightTheme,
    );
    _result?.render(renderer);

    return Container(
      margin: const EdgeInsets.symmetric(vertical: 4),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(10, 6, 4, 0),
            child: Row(
              children: [
                Text(
                  widget.lang.isEmpty ? 'code' : widget.lang,
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: theme.colorScheme.outline,
                  ),
                ),
                const Spacer(),
                IconButton(
                  icon: const Icon(Icons.copy, size: 16),
                  visualDensity: VisualDensity.compact,
                  tooltip: 'Copy',
                  onPressed: () =>
                      Clipboard.setData(ClipboardData(text: _text)),
                ),
              ],
            ),
          ),
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.fromLTRB(10, 0, 10, 10),
            child: Text.rich(
              renderer.span ?? TextSpan(text: _text, style: mono),
            ),
          ),
        ],
      ),
    );
  }
}

class _MathBlock extends StatelessWidget {
  const _MathBlock({required this.tex, required this.textScale});

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
              fontSize: (theme.textTheme.bodyMedium?.fontSize ?? 14) * textScale,
            ),
          ),
        ),
      ),
    );
  }
}

class _TableBlock extends StatelessWidget {
  const _TableBlock({
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
    final columns = rows.fold<int>(0, (n, r) => r.cells.length > n ? r.cells.length : n);

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
