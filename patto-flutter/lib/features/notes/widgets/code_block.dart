import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:re_highlight/languages/all.dart';
import 'package:re_highlight/re_highlight.dart';
import 'package:re_highlight/styles/atom-one-dark.dart';
import 'package:re_highlight/styles/atom-one-light.dart';

/// A `[@code lang]` block: highlighted once, with a copy button.
class CodeBlock extends StatefulWidget {
  const CodeBlock({
    super.key,
    required this.lang,
    required this.lines,
    required this.textScale,
  });

  final String lang;
  final List<String> lines;
  final double textScale;

  @override
  State<CodeBlock> createState() => _CodeBlockState();
}

class _CodeBlockState extends State<CodeBlock> {
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
