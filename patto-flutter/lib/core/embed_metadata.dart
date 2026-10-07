import 'embed_lookup.dart';
import 'oembed.dart';
import '../src/rust/api/types.dart';
import '../src/rust/frb_api.dart' as rust;

/// What a card shows for a tweet before it is opened.
class TweetSummary {
  const TweetSummary({
    required this.author,
    required this.text,
    required this.html,
  });

  final String author;
  final String text;

  /// The oEmbed snippet, which the viewer renders with Twitter's widget script.
  final String html;
}

/// A slide deck on Speaker Deck or SlideShare.
class DeckPreview {
  const DeckPreview({this.title, this.thumbnailUrl, this.playerUrl});

  final String? title;
  final String? thumbnailUrl;

  /// The page the site's own embed iframe points at, which fills a screen on
  /// its own.
  final String? playerUrl;
}

class Tweets {
  Tweets._();

  static Future<TweetSummary?> lookup(String url) {
    return EmbedLookup.cached('tweet:$url', () async {
      final json = await EmbedLookup.fetchJson(
        Uri.https('publish.twitter.com', '/oembed', {
          'url': url,
          'omit_script': 'true',
          'dnt': 'true',
        }),
      );
      final html = json?['html'];
      if (html is! String) return null;
      return TweetSummary(
        author: json?['author_name'] as String? ?? '',
        text: tweetTextFromHtml(html),
        html: html,
      );
    });
  }
}

class Decks {
  Decks._();

  /// Speaker Deck's oEmbed has the title and player but no thumbnail; the
  /// deck page's Open Graph image is the first slide.
  static Future<DeckPreview?> speakerDeck(String url) {
    return EmbedLookup.cached('speakerdeck:$url', () async {
      final results = await Future.wait([
        EmbedLookup.fetchJson(
          Uri.https('speakerdeck.com', '/oembed.json', {'url': url}),
        ).then<Object?>((v) => v, onError: (_) => null),
        LinkPreviews.lookup(url),
      ]);
      final json = results[0] as Map<String, Object?>?;
      final graph = results[1] as OpenGraphMeta?;
      if (json == null && graph == null) return null;
      final html = json?['html'];
      return DeckPreview(
        title: json?['title'] as String? ?? graph?.title,
        thumbnailUrl: graph?.image,
        playerUrl: html is String ? iframeSrc(html) : null,
      );
    });
  }

  static Future<DeckPreview?> slideShare(String url) {
    return EmbedLookup.cached('slideshare:$url', () async {
      final json = await EmbedLookup.fetchJson(
        Uri.https('www.slideshare.net', '/api/oembed/2', {
          'url': url,
          'format': 'json',
        }),
      );
      if (json == null) return null;
      final html = json['html'];
      return DeckPreview(
        title: json['title'] as String?,
        thumbnailUrl: switch (json['thumbnail_url'] ?? json['thumbnail']) {
          final String url => absoluteUrl(url),
          _ => null,
        },
        playerUrl: html is String ? iframeSrc(html) : null,
      );
    });
  }
}

/// Open Graph tags of any page, for embeds of sites the app knows nothing
/// about. A page without tags still resolves, to an empty value.
class LinkPreviews {
  LinkPreviews._();

  static Future<OpenGraphMeta?> lookup(String url) {
    return EmbedLookup.cachedPage(
      'open-graph:$url',
      url,
      (html) async => decodeOpenGraph(await rust.parseOpenGraph(html: html)),
    );
  }
}

/// The Rust scraper leaves most character references as written (`&#039;`,
/// `&#x27;`); decode them so titles read as the page shows them.
OpenGraphMeta decodeOpenGraph(OpenGraphMeta meta) {
  String? decode(String? s) => s == null ? null : decodeEntities(s);
  return OpenGraphMeta(
    title: decode(meta.title),
    image: meta.image,
    description: decode(meta.description),
    siteName: decode(meta.siteName),
  );
}
