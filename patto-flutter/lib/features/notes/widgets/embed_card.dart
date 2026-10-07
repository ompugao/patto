import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';

import '../../../core/google_photos.dart';
import '../../../src/rust/api/types.dart';

/// Preview for a line that is a single `[@embed ...]`.
///
/// Deliberately cheap to build while scrolling: no WebView, no PDF rendering,
/// and no network lookups except one cached page fetch per Google Photos link.
/// The height depends only on the width and text scale, so the list's extent
/// never jumps once a thumbnail arrives. The real content opens on tap.
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
      EmbedKind_Pdf() => _tile(context, Icons.picture_as_pdf_outlined, 'PDF'),
      EmbedKind_Twitter() => _tile(
        context,
        Icons.forum_outlined,
        'X / Twitter',
      ),
      EmbedKind_SpeakerDeck() => _tile(
        context,
        Icons.slideshow_outlined,
        'Speaker Deck',
      ),
      EmbedKind_SlideShare() => _tile(
        context,
        Icons.slideshow_outlined,
        'SlideShare',
      ),
      EmbedKind_Other() => _tile(context, Icons.link, 'Link'),
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

  Widget _tile(BuildContext context, IconData icon, String source) {
    final theme = Theme.of(context);
    final url = embed.url;
    final name = embed.isLocal ? url.split('/').last : url;
    final bodyStyle = theme.textTheme.bodyLarge!;
    final smallStyle = theme.textTheme.bodySmall!;

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
                Text(
                  embed.title ?? name,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: bodyStyle.copyWith(
                    fontSize: (bodyStyle.fontSize ?? 16) * textScale,
                  ),
                ),
                Text(
                  embed.title == null ? source : '$source · $name',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: smallStyle.copyWith(
                    fontSize: (smallStyle.fontSize ?? 12) * textScale,
                    color: theme.colorScheme.outline,
                  ),
                ),
              ],
            ),
          ),
        ],
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
    final theme = Theme.of(context);
    final smallStyle = theme.textTheme.bodySmall!;
    return ColoredBox(
      color: theme.colorScheme.surfaceContainerHighest,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.photo_library_outlined,
              size: 32 * widget.textScale,
              color: theme.colorScheme.primary,
            ),
            const SizedBox(height: 8),
            Text(
              widget.url,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              textAlign: TextAlign.center,
              style: smallStyle.copyWith(
                fontSize: (smallStyle.fontSize ?? 12) * widget.textScale,
                color: theme.colorScheme.outline,
              ),
            ),
          ],
        ),
      ),
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
