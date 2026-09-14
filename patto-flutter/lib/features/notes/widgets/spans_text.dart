import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_math_fork/flutter_math.dart';

import '../../../src/rust/api/types.dart';
import 'note_image.dart';

/// What a tap on a span should do. Supplied by the screen, so span widgets stay
/// free of providers.
class SpanActions {
  const SpanActions({
    required this.onWikiLink,
    required this.onUrl,
    required this.onAnchor,
  });

  final void Function(String name, String? anchor) onWikiLink;
  final void Function(String url) onUrl;
  final void Function(String anchor) onAnchor;
}

/// Renders a line's spans as rich text.
///
/// Stateful because each tappable span owns a gesture recognizer that has to be
/// disposed with the widget; a list item is rebuilt constantly while scrolling.
class SpansText extends StatefulWidget {
  const SpansText({
    super.key,
    required this.spans,
    required this.actions,
    this.style,
    this.trailing,
    this.noteRoot,
    this.strikeThrough = false,
  });

  final List<NoteSpan> spans;
  final SpanActions actions;
  final TextStyle? style;

  /// Appended after the text, used for the due-date chip so it wraps with it.
  final InlineSpan? trailing;
  final String? noteRoot;
  final bool strikeThrough;

  @override
  State<SpansText> createState() => _SpansTextState();
}

class _SpansTextState extends State<SpansText> {
  final _recognizers = <GestureRecognizer>[];

  @override
  void dispose() {
    for (final r in _recognizers) {
      r.dispose();
    }
    super.dispose();
  }

  GestureRecognizer _tap(VoidCallback action) {
    final recognizer = TapGestureRecognizer()..onTap = action;
    _recognizers.add(recognizer);
    return recognizer;
  }

  @override
  Widget build(BuildContext context) {
    _recognizers.clear();
    final theme = Theme.of(context);
    final base = (widget.style ?? theme.textTheme.bodyLarge!).copyWith(
      decoration: widget.strikeThrough ? TextDecoration.lineThrough : null,
      color: widget.strikeThrough ? theme.colorScheme.outline : null,
    );

    return Text.rich(
      TextSpan(
        children: [
          for (final span in widget.spans) _build(span, base, theme),
          if (widget.trailing != null) widget.trailing!,
        ],
      ),
    );
  }

  InlineSpan _build(NoteSpan span, TextStyle style, ThemeData theme) {
    final colors = theme.colorScheme;

    return switch (span) {
      NoteSpan_Text(:final text) => TextSpan(text: text, style: style),
      NoteSpan_Decoration(
        :final fontsize,
        :final italic,
        :final underline,
        :final deleted,
        :final children,
      ) =>
        () {
          // Matches the web renderer: 100% + 20% per level above one, bold for
          // any explicit size.
          final scale = 1 + math.max(fontsize - 1, 0) * 0.2;
          final decorated = style.copyWith(
            fontSize: (style.fontSize ?? 16) * scale,
            fontWeight: fontsize > 0 ? FontWeight.bold : null,
            fontStyle: italic ? FontStyle.italic : null,
            color: deleted ? colors.outline : style.color,
            decoration: TextDecoration.combine([
              if (underline) TextDecoration.underline,
              if (deleted) TextDecoration.lineThrough,
              if (widget.strikeThrough) TextDecoration.lineThrough,
            ]),
          );
          return TextSpan(
            children: [for (final c in children) _build(c, decorated, theme)],
          );
        }(),
      NoteSpan_WikiLink(:final name, :final anchor) => TextSpan(
        text: name.isEmpty ? '#$anchor' : (anchor == null ? name : '$name#$anchor'),
        style: style.copyWith(color: colors.primary),
        recognizer: _tap(() {
          if (name.isEmpty && anchor != null) {
            widget.actions.onAnchor(anchor);
          } else {
            widget.actions.onWikiLink(name, anchor);
          }
        }),
      ),
      NoteSpan_Url(:final url, :final title) => TextSpan(
        text: title ?? url,
        style: style.copyWith(
          color: colors.primary,
          decoration: TextDecoration.underline,
          decorationColor: colors.primary,
        ),
        recognizer: _tap(() => widget.actions.onUrl(url)),
      ),
      NoteSpan_InlineCode(:final code) => TextSpan(
        text: code,
        style: style.copyWith(
          fontFamily: 'monospace',
          fontFamilyFallback: const ['Roboto Mono', 'Noto Sans Mono CJK JP'],
          backgroundColor: colors.surfaceContainerHighest,
        ),
      ),
      NoteSpan_InlineMath(:final tex) => WidgetSpan(
        alignment: PlaceholderAlignment.middle,
        child: Math.tex(
          tex,
          textStyle: style,
          onErrorFallback: (_) => Text(tex, style: style.copyWith(
            fontFamily: 'monospace',
          )),
        ),
      ),
      NoteSpan_Image(:final image) => WidgetSpan(
        alignment: PlaceholderAlignment.middle,
        child: SizedBox(
          height: (style.fontSize ?? 16) * 1.6,
          child: NoteImage(image: image, root: widget.noteRoot, inline: true),
        ),
      ),
      NoteSpan_Embed(:final url, :final title) => TextSpan(
        text: title ?? url,
        style: style.copyWith(
          color: colors.primary,
          decoration: TextDecoration.underline,
          decorationColor: colors.primary,
        ),
        recognizer: _tap(() => widget.actions.onUrl(url)),
      ),
    };
  }
}
