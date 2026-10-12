import 'package:flutter/material.dart';

/// Text that always takes up a given number of lines, so a card keeps its
/// height whether the text is still loading, short, or long enough to be cut.
class FixedLines extends StatelessWidget {
  const FixedLines(
    this.text, {
    super.key,
    required this.lines,
    required this.style,
    required this.textScale,
  });

  final String text;
  final int lines;
  final TextStyle style;
  final double textScale;

  @override
  Widget build(BuildContext context) {
    final fontSize = (style.fontSize ?? 14) * textScale;
    // The system font scale applies on top of the app's own, as it does to
    // every other Text; the reserved height has to follow it too.
    final lineHeight =
        MediaQuery.textScalerOf(context).scale(fontSize) *
        (style.height ?? 1.5);
    return ConstrainedBox(
      constraints: BoxConstraints(minHeight: lineHeight * lines),
      child: Text(
        text,
        maxLines: lines,
        overflow: TextOverflow.ellipsis,
        style: style.copyWith(fontSize: fontSize),
      ),
    );
  }
}

/// An icon beside a one-line title and a one-line subtitle.
class EmbedTile extends StatelessWidget {
  const EmbedTile({
    super.key,
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.textScale,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final double textScale;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      child: Row(
        children: [
          Icon(icon, size: 28 * textScale, color: theme.colorScheme.primary),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                FixedLines(
                  title,
                  lines: 1,
                  style: theme.textTheme.bodyLarge!,
                  textScale: textScale,
                ),
                FixedLines(
                  subtitle,
                  lines: 1,
                  style: theme.textTheme.bodySmall!.copyWith(
                    color: theme.colorScheme.outline,
                  ),
                  textScale: textScale,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// What a 16:9 box shows when there is no picture for it.
class EmbedFallback extends StatelessWidget {
  const EmbedFallback({
    super.key,
    required this.icon,
    required this.text,
    required this.textScale,
  });

  final IconData icon;
  final String text;
  final double textScale;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final smallStyle = theme.textTheme.bodySmall!;
    return ColoredBox(
      color: theme.colorScheme.surfaceContainerHighest,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(icon, size: 32 * textScale, color: theme.colorScheme.primary),
            const SizedBox(height: 8),
            Text(
              text,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              textAlign: TextAlign.center,
              style: smallStyle.copyWith(
                fontSize: (smallStyle.fontSize ?? 12) * textScale,
                color: theme.colorScheme.outline,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class PlayBadge extends StatelessWidget {
  const PlayBadge({super.key});

  @override
  Widget build(BuildContext context) {
    return const Center(
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: Color(0xCC000000),
          shape: BoxShape.circle,
        ),
        child: Padding(
          padding: EdgeInsets.all(8),
          child: Icon(Icons.play_arrow, color: Colors.white, size: 36),
        ),
      ),
    );
  }
}

class EmbedCaption extends StatelessWidget {
  const EmbedCaption({super.key, required this.title, required this.textScale});

  final String title;
  final double textScale;

  @override
  Widget build(BuildContext context) {
    final smallStyle = Theme.of(context).textTheme.bodyMedium!;
    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
      child: Text(
        title,
        maxLines: 2,
        overflow: TextOverflow.ellipsis,
        style: smallStyle.copyWith(
          fontSize: (smallStyle.fontSize ?? 14) * textScale,
        ),
      ),
    );
  }
}

/// Starts [lookup] for [url] when first built, which the list only does for
/// blocks near the viewport, so a long note never looks up every embed at
/// once. The same URL keeps its future across rebuilds; a new one starts over.
class LookupBuilder<T extends Object> extends StatefulWidget {
  const LookupBuilder({
    super.key,
    required this.url,
    required this.lookup,
    required this.builder,
  });

  final String url;
  final Future<T?> Function(String url) lookup;
  final Widget Function(BuildContext context, AsyncSnapshot<T?> snapshot)
  builder;

  @override
  State<LookupBuilder<T>> createState() => _LookupBuilderState<T>();
}

class _LookupBuilderState<T extends Object> extends State<LookupBuilder<T>> {
  late Future<T?> _result = widget.lookup(widget.url);

  @override
  void didUpdateWidget(LookupBuilder<T> oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.url != widget.url) {
      _result = widget.lookup(widget.url);
    }
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<T?>(future: _result, builder: widget.builder);
  }
}
