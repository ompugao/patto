import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';

import '../../../core/embed_metadata.dart';
import '../../../src/rust/api/types.dart';
import 'embed_card.dart';
import 'embed_card_parts.dart';

/// A tweet: author and the first two lines of its text once the oEmbed
/// lookup returns; the link until then, or when offline.
class TweetCard extends StatelessWidget {
  const TweetCard({
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
    return LookupBuilder<TweetSummary>(
      url: url,
      lookup: Tweets.lookup,
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
                size: 28 * textScale,
                color: theme.colorScheme.primary,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    FixedLines(
                      tweet?.author ?? title ?? 'X / Twitter',
                      lines: 1,
                      style: theme.textTheme.bodyLarge!,
                      textScale: textScale,
                    ),
                    FixedLines(
                      tweet?.text ?? (loading ? '' : url),
                      lines: 2,
                      style: theme.textTheme.bodyMedium!.copyWith(
                        color: tweet == null ? theme.colorScheme.outline : null,
                      ),
                      textScale: textScale,
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
class DeckCard extends StatelessWidget {
  const DeckCard({
    super.key,
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
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final dpr = MediaQuery.devicePixelRatioOf(context);
    final blank = ColoredBox(color: theme.colorScheme.surfaceContainerHighest);

    return LookupBuilder<DeckPreview>(
      url: url,
      lookup: lookup,
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
                  ? EmbedFallback(
                      icon: Icons.slideshow_outlined,
                      text: url,
                      textScale: textScale,
                    )
                  : CachedNetworkImage(
                      imageUrl: thumbnail,
                      memCacheWidth: (EmbedCard.maxWidth * dpr).round(),
                      fit: BoxFit.cover,
                      placeholder: (_, _) => blank,
                      errorWidget: (_, _, _) => EmbedFallback(
                        icon: Icons.slideshow_outlined,
                        text: url,
                        textScale: textScale,
                      ),
                    ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  FixedLines(
                    title ?? deck?.title ?? url,
                    lines: 1,
                    style: theme.textTheme.bodyMedium!,
                    textScale: textScale,
                  ),
                  FixedLines(
                    source,
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
        );
      },
    );
  }
}

/// Any other web page: a link tile whose title and site come from the
/// page's Open Graph tags once read.
class LinkCard extends StatelessWidget {
  const LinkCard({
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
    final host = Uri.tryParse(url)?.host;
    return LookupBuilder<OpenGraphMeta>(
      url: url,
      lookup: LinkPreviews.lookup,
      builder: (context, snapshot) {
        final page = snapshot.data;
        final shown = title ?? page?.title;
        final site = page?.siteName ?? host;
        return EmbedTile(
          icon: Icons.link,
          title: shown ?? url,
          subtitle: shown == null
              ? (site ?? 'Link')
              : (site == null ? url : '$site · $url'),
          textScale: textScale,
        );
      },
    );
  }
}
