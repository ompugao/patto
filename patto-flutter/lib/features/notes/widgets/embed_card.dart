import 'package:flutter/material.dart';

import '../../../core/embed_metadata.dart';
import '../../../src/rust/api/types.dart';
import 'embed_card_parts.dart';
import 'embed_link_cards.dart';
import 'embed_media_cards.dart';

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
  static const maxWidth = 480.0;

  final EmbedRef embed;
  final VoidCallback onTap;
  final double textScale;

  @override
  Widget build(BuildContext context) {
    final card = switch (embed.kind) {
      EmbedKind_Youtube(:final videoId) => YoutubeCard(
        videoId: videoId,
        title: embed.title,
        textScale: textScale,
      ),
      EmbedKind_GooglePhotos() => GooglePhotosCard(
        url: embed.url,
        title: embed.title,
        textScale: textScale,
      ),
      EmbedKind_Pdf() => EmbedTile(
        icon: Icons.picture_as_pdf_outlined,
        title: embed.title ?? _fileName,
        subtitle: embed.title == null ? 'PDF' : 'PDF · $_fileName',
        textScale: textScale,
      ),
      EmbedKind_Twitter() => TweetCard(
        url: embed.url,
        title: embed.title,
        textScale: textScale,
      ),
      EmbedKind_SpeakerDeck() => DeckCard(
        url: embed.url,
        title: embed.title,
        source: 'Speaker Deck',
        lookup: Decks.speakerDeck,
        textScale: textScale,
      ),
      EmbedKind_SlideShare() => DeckCard(
        url: embed.url,
        title: embed.title,
        source: 'SlideShare',
        lookup: Decks.slideShare,
        textScale: textScale,
      ),
      EmbedKind_Other() => LinkCard(
        url: embed.url,
        title: embed.title,
        textScale: textScale,
      ),
    };

    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: maxWidth),
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
