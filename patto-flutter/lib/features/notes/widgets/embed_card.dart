import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';

import '../../../core/embed_metadata.dart';
import '../../../core/google_photos.dart';
import '../../../src/rust/api/types.dart';

/// Preview for a line that is a single `[@embed ...]`.
///
/// Deliberately cheap to build while scrolling: no WebView and no PDF
/// rendering. A card that needs the network (a tweet's text, a deck's first
/// slide, a page's Open Graph title) starts one cached lookup when it is first
/// built, which the list only does near the viewport. The height depends only
/// on the width and text scale, never on what a lookup returns, so the list's
/// extent never jumps. The real content opens on tap.
class EmbedCard extends StatelessWidget {
  const EmbedCard({
    super.key,
    required this.embed,
    required this.onTap,
    this.textScale = 1.0,
  });

  /// Wider than this a thumbnail stops helping and just pushes text away.
  static const _maxWidth = 480.0;

  final EmbedRef embed;
  final VoidCallback onTap;
  final double textScale;

  @override
  Widget build(BuildContext context) {
    final card = switch (embed.kind) {
      EmbedKind_Youtube(:final videoId) => _YoutubeCard(
        videoId: videoId,
        title: embed.title,
        textScale: textScale,
      ),
      EmbedKind_GooglePhotos() => _GooglePhotosCard(
        url: embed.url,
        title: embed.title,
        textScale: textScale,
      ),
      EmbedKind_Pdf() => _Tile(
        icon: Icons.picture_as_pdf_outlined,
        title: embed.title ?? _fileName,
        subtitle: embed.title == null ? 'PDF' : 'PDF · $_fileName',
        textScale: textScale,
      ),
      EmbedKind_Twitter() => _TweetCard(
        url: embed.url,
        title: embed.title,
        textScale: textScale,
      ),
      EmbedKind_SpeakerDeck() => _DeckCard(
        url: embed.url,
        title: embed.title,
        source: 'Speaker Deck',
        lookup: Decks.speakerDeck,
        textScale: textScale,
      ),
      EmbedKind_SlideShare() => _DeckCard(
        url: embed.url,
        title: embed.title,
        source: 'SlideShare',
        lookup: Decks.slideShare,
        textScale: textScale,
      ),
      EmbedKind_Other() => _LinkCard(
        url: embed.url,
        title: embed.title,
        textScale: textScale,
      ),
    };

    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: _maxWidth),
        child: Material(
          color: theme.colorScheme.surfaceContainerLow,
          shape: RoundedRectangleBorder(
            side: BorderSide(color: theme.colorScheme.outlineVariant),
            borderRadius: BorderRadius.circular(8),
          ),
          clipBehavior: Clip.antiAlias,
          child: InkWell(onTap: onTap, child: card),
        ),
      ),
    );
  }

  String get _fileName => embed.url.split('/').last;
}

/// Text that always takes up a given number of lines, so a card keeps its
/// height whether the text is still loading, short, or long enough to be cut.
class _Lines extends StatelessWidget {
  const _Lines(
    this.text, {
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
class _Tile extends StatelessWidget {
  const _Tile({
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
                _Lines(
                  title,
                  lines: 1,
                  style: theme.textTheme.bodyLarge!,
                  textScale: textScale,
                ),
                _Lines(
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

/// A tweet: author and the first two lines of its text once the oEmbed
/// lookup returns; the link until then, or when offline.
class _TweetCard extends StatefulWidget {
  const _TweetCard({
    required this.url,
    required this.title,
    required this.textScale,
  });

  final String url;
  final String? title;
  final double textScale;

  @override
  State<_TweetCard> createState() => _TweetCardState();
}

class _TweetCardState extends State<_TweetCard> {
  late Future<TweetSummary?> _tweet = Tweets.lookup(widget.url);

  @override
  void didUpdateWidget(_TweetCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.url != widget.url) {
      _tweet = Tweets.lookup(widget.url);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return FutureBuilder<TweetSummary?>(
      future: _tweet,
      builder: (context, snapshot) {
        final tweet = snapshot.data;
        final loading = snapshot.connectionState != ConnectionState.done;
        return Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(
                Icons.forum_outlined,
                size: 28 * widget.textScale,
                color: theme.colorScheme.primary,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    _Lines(
                      tweet?.author ?? widget.title ?? 'X / Twitter',
                      lines: 1,
                      style: theme.textTheme.bodyLarge!,
                      textScale: widget.textScale,
                    ),
                    _Lines(
                      tweet?.text ?? (loading ? '' : widget.url),
                      lines: 2,
                      style: theme.textTheme.bodyMedium!.copyWith(
                        color: tweet == null ? theme.colorScheme.outline : null,
                      ),
                      textScale: widget.textScale,
                    ),
                  ],
                ),
              ),
            ],
          ),
        );
      },
    );
  }
}

/// A slide deck: its first slide and title once looked up. The box is always
/// 16:9 over a two-line caption, so the note never shifts.
class _DeckCard extends StatefulWidget {
  const _DeckCard({
    required this.url,
    required this.title,
    required this.source,
    required this.lookup,
    required this.textScale,
  });

  final String url;
  final String? title;
  final String source;
  final Future<DeckPreview?> Function(String url) lookup;
  final double textScale;

  @override
  State<_DeckCard> createState() => _DeckCardState();
}

class _DeckCardState extends State<_DeckCard> {
  late Future<DeckPreview?> _deck = widget.lookup(widget.url);

  @override
  void didUpdateWidget(_DeckCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.url != widget.url) {
      _deck = widget.lookup(widget.url);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final dpr = MediaQuery.devicePixelRatioOf(context);
    final blank = ColoredBox(color: theme.colorScheme.surfaceContainerHighest);

    return FutureBuilder<DeckPreview?>(
      future: _deck,
      builder: (context, snapshot) {
        final deck = snapshot.data;
        final done = snapshot.connectionState == ConnectionState.done;
        final thumbnail = deck?.thumbnailUrl;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            AspectRatio(
              aspectRatio: 16 / 9,
              child: !done
                  ? blank
                  : thumbnail == null
                  ? _EmbedFallback(
                      icon: Icons.slideshow_outlined,
                      text: widget.url,
                      textScale: widget.textScale,
                    )
                  : CachedNetworkImage(
                      imageUrl: thumbnail,
                      memCacheWidth: (EmbedCard._maxWidth * dpr).round(),
                      fit: BoxFit.cover,
                      placeholder: (_, _) => blank,
                      errorWidget: (_, _, _) => _EmbedFallback(
                        icon: Icons.slideshow_outlined,
                        text: widget.url,
                        textScale: widget.textScale,
                      ),
                    ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  _Lines(
                    widget.title ?? deck?.title ?? widget.url,
                    lines: 1,
                    style: theme.textTheme.bodyMedium!,
                    textScale: widget.textScale,
                  ),
                  _Lines(
                    widget.source,
                    lines: 1,
                    style: theme.textTheme.bodySmall!.copyWith(
                      color: theme.colorScheme.outline,
                    ),
                    textScale: widget.textScale,
                  ),
                ],
              ),
            ),
          ],
        );
      },
    );
  }
}

/// Any other web page: a link tile whose title and site come from the
/// page's Open Graph tags once read.
class _LinkCard extends StatefulWidget {
  const _LinkCard({
    required this.url,
    required this.title,
    required this.textScale,
  });

  final String url;
  final String? title;
  final double textScale;

  @override
  State<_LinkCard> createState() => _LinkCardState();
}

class _LinkCardState extends State<_LinkCard> {
  late Future<OpenGraphMeta?> _page = LinkPreviews.lookup(widget.url);

  @override
  void didUpdateWidget(_LinkCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.url != widget.url) {
      _page = LinkPreviews.lookup(widget.url);
    }
  }

  @override
  Widget build(BuildContext context) {
    final host = Uri.tryParse(widget.url)?.host;
    return FutureBuilder<OpenGraphMeta?>(
      future: _page,
      builder: (context, snapshot) {
        final page = snapshot.data;
        final title = widget.title ?? page?.title;
        final site = page?.siteName ?? host;
        return _Tile(
          icon: Icons.link,
          title: title ?? widget.url,
          subtitle: title == null
              ? (site ?? 'Link')
              : (site == null ? widget.url : '$site · ${widget.url}'),
          textScale: widget.textScale,
        );
      },
    );
  }
}

class _EmbedFallback extends StatelessWidget {
  const _EmbedFallback({
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

class _YoutubeCard extends StatelessWidget {
  const _YoutubeCard({
    required this.videoId,
    required this.title,
    required this.textScale,
  });

  final String videoId;
  final String? title;
  final double textScale;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final dpr = MediaQuery.devicePixelRatioOf(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        AspectRatio(
          aspectRatio: 16 / 9,
          child: Stack(
            fit: StackFit.expand,
            children: [
              // A fixed image URL rather than an oEmbed lookup: no request
              // before the thumbnail, and nothing to wait on to lay out.
              CachedNetworkImage(
                imageUrl: 'https://i.ytimg.com/vi/$videoId/hqdefault.jpg',
                memCacheWidth: (EmbedCard._maxWidth * dpr).round(),
                fit: BoxFit.cover,
                placeholder: (_, _) => ColoredBox(
                  color: theme.colorScheme.surfaceContainerHighest,
                ),
                errorWidget: (_, _, _) => ColoredBox(
                  color: theme.colorScheme.surfaceContainerHighest,
                ),
              ),
              const _PlayBadge(),
            ],
          ),
        ),
        if (title != null) _Caption(title: title!, textScale: textScale),
      ],
    );
  }
}

/// A Google Photos share: its thumbnail once the share page has been read,
/// with a play badge for videos. Tapping opens the share itself, since the
/// page cannot be framed and its video stream refuses foreign referrers.
///
/// Offline or unshared links keep the same 16:9 box, showing the link
/// instead, so a failed lookup never shifts the note.
class _GooglePhotosCard extends StatefulWidget {
  const _GooglePhotosCard({
    required this.url,
    required this.title,
    required this.textScale,
  });

  final String url;
  final String? title;
  final double textScale;

  @override
  State<_GooglePhotosCard> createState() => _GooglePhotosCardState();
}

class _GooglePhotosCardState extends State<_GooglePhotosCard> {
  // Started when the card is first built, which the list only does for blocks
  // near the viewport, so a long note never looks up every embed at once.
  late Future<GooglePhotosMedia?> _media = GooglePhotos.lookup(widget.url);

  @override
  void didUpdateWidget(_GooglePhotosCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.url != widget.url) {
      _media = GooglePhotos.lookup(widget.url);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final dpr = MediaQuery.devicePixelRatioOf(context);
    final blank = ColoredBox(color: theme.colorScheme.surfaceContainerHighest);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        AspectRatio(
          aspectRatio: 16 / 9,
          child: FutureBuilder<GooglePhotosMedia?>(
            future: _media,
            builder: (context, snapshot) {
              if (snapshot.connectionState != ConnectionState.done) {
                return blank;
              }
              final media = snapshot.data;
              if (media == null) return _fallback(context);
              return Stack(
                fit: StackFit.expand,
                children: [
                  CachedNetworkImage(
                    imageUrl: media.thumbnailUrl,
                    memCacheWidth: (EmbedCard._maxWidth * dpr).round(),
                    fit: BoxFit.cover,
                    placeholder: (_, _) => blank,
                    errorWidget: (context, _, _) => _fallback(context),
                  ),
                  if (media.videoUrl != null) const _PlayBadge(),
                ],
              );
            },
          ),
        ),
        if (widget.title != null)
          _Caption(title: widget.title!, textScale: widget.textScale),
      ],
    );
  }

  Widget _fallback(BuildContext context) {
    return _EmbedFallback(
      icon: Icons.photo_library_outlined,
      text: widget.url,
      textScale: widget.textScale,
    );
  }
}

class _PlayBadge extends StatelessWidget {
  const _PlayBadge();

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

class _Caption extends StatelessWidget {
  const _Caption({required this.title, required this.textScale});

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
