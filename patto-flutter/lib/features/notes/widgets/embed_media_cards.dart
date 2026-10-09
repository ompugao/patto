import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';

import '../../../core/embed_metadata.dart';
import '../../../src/rust/api/types.dart';
import 'embed_card.dart';
import 'embed_card_parts.dart';

class YoutubeCard extends StatelessWidget {
  const YoutubeCard({
    super.key,
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
                memCacheWidth: (EmbedCard.maxWidth * dpr).round(),
                fit: BoxFit.cover,
                placeholder: (_, _) => ColoredBox(
                  color: theme.colorScheme.surfaceContainerHighest,
                ),
                errorWidget: (_, _, _) => ColoredBox(
                  color: theme.colorScheme.surfaceContainerHighest,
                ),
              ),
              const PlayBadge(),
            ],
          ),
        ),
        if (title != null) EmbedCaption(title: title!, textScale: textScale),
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
class GooglePhotosCard extends StatelessWidget {
  const GooglePhotosCard({
    super.key,
    required this.url,
    required this.title,
    required this.textScale,
  });

  final String url;
  final String? title;
  final double textScale;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final dpr = MediaQuery.devicePixelRatioOf(context);
    final blank = ColoredBox(color: theme.colorScheme.surfaceContainerHighest);
    final fallback = EmbedFallback(
      icon: Icons.photo_library_outlined,
      text: url,
      textScale: textScale,
    );

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        AspectRatio(
          aspectRatio: 16 / 9,
          child: LookupBuilder<GooglePhotosMedia>(
            url: url,
            lookup: GooglePhotos.lookup,
            builder: (context, snapshot) {
              if (snapshot.connectionState != ConnectionState.done) {
                return blank;
              }
              final media = snapshot.data;
              if (media == null) return fallback;
              return Stack(
                fit: StackFit.expand,
                children: [
                  CachedNetworkImage(
                    imageUrl: media.thumbnailUrl,
                    memCacheWidth: (EmbedCard.maxWidth * dpr).round(),
                    fit: BoxFit.cover,
                    placeholder: (_, _) => blank,
                    errorWidget: (_, _, _) => fallback,
                  ),
                  if (media.videoUrl != null) const PlayBadge(),
                ],
              );
            },
          ),
        ),
        if (title != null) EmbedCaption(title: title!, textScale: textScale),
      ],
    );
  }
}
