import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';

import '../../../src/rust/api/types.dart';

/// Preview for a line that is a single `[@embed ...]`.
///
/// Deliberately cheap to build while scrolling: no WebView, no PDF rendering
/// and no network lookups, and the height depends only on the width and text
/// scale, so the list's extent never jumps once a thumbnail arrives. The real
/// content opens on tap.
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
    final smallStyle = theme.textTheme.bodyMedium!;

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
              const Center(
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: Color(0xCC000000),
                    shape: BoxShape.circle,
                  ),
                  child: Padding(
                    padding: EdgeInsets.all(8),
                    child: Icon(
                      Icons.play_arrow,
                      color: Colors.white,
                      size: 36,
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
        if (title != null)
          Padding(
            padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
            child: Text(
              title!,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: smallStyle.copyWith(
                fontSize: (smallStyle.fontSize ?? 14) * textScale,
              ),
            ),
          ),
      ],
    );
  }
}
