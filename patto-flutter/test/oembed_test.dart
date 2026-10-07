import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/core/embed_metadata.dart';
import 'package:patto_flutter/core/oembed.dart';
import 'package:patto_flutter/src/rust/api/types.dart';

void main() {
  const tweetHtml =
      '<blockquote class="twitter-tweet"><p lang="en" dir="ltr">Shipping '
      '&amp; done.<br>Second line <a href="https://t.co/x">pic.twitter.com/x</a>'
      '</p>&mdash; Someone (@someone) <a href="https://twitter.com/someone/status/1">'
      'October 6, 2026</a></blockquote>';

  test('tweet text is the paragraph without the author line', () {
    expect(
      tweetTextFromHtml(tweetHtml),
      'Shipping & done.\nSecond line pic.twitter.com/x',
    );
  });

  test('a snippet without a paragraph is stripped whole', () {
    expect(tweetTextFromHtml('<b>bold</b> &lt;tag&gt;'), 'bold <tag>');
  });

  test('entities decode by name, decimal and hex', () {
    expect(
      decodeEntities('a &amp; b &#39;c&#x27; &mdash; &unknown;'),
      "a & b 'c' — &unknown;",
    );
  });

  test('stripping collapses blanks and drops empty lines', () {
    expect(stripHtml('<p>  one   two </p><br><br>  three  '), 'one two\nthree');
  });

  test('iframe src is found and protocol-relative URLs are made https', () {
    expect(
      iframeSrc(
        '<iframe src="//www.slideshare.net/slideshow/embed_code/key/abc" '
        'width="100"></iframe>',
      ),
      'https://www.slideshare.net/slideshow/embed_code/key/abc',
    );
    expect(
      iframeSrc(
        "<iframe class='speakerdeck-iframe' src='https://speakerdeck.com/player/1?a=1&amp;b=2'></iframe>",
      ),
      'https://speakerdeck.com/player/1?a=1&b=2',
    );
    expect(iframeSrc('<div>no frame</div>'), isNull);
  });

  test('references outside Unicode or to surrogates are left as written', () {
    expect(
      decodeEntities('&#1114112; &#xD800; &#x10FFFF;'),
      '&#1114112; &#xD800; \u{10FFFF}',
    );
  });

  test('protocol-relative URLs get a scheme and others are untouched', () {
    expect(
      absoluteUrl('//cdn.example.com/a.jpg'),
      'https://cdn.example.com/a.jpg',
    );
    expect(absoluteUrl('http://example.com/a.jpg'), 'http://example.com/a.jpg');
  });

  test('open graph text fields are entity-decoded, the image URL is not', () {
    final meta = decodeOpenGraph(
      const OpenGraphMeta(
        title: 'Rust &#039;n&#x27; roll',
        image: 'https://img.example.com/a.jpg?x=1&amp;y=2',
        description: 'A &lt;b&gt; tag',
        siteName: 'Example &amp; Co',
      ),
    );
    expect(meta.title, "Rust 'n' roll");
    expect(meta.image, 'https://img.example.com/a.jpg?x=1&amp;y=2');
    expect(meta.description, 'A <b> tag');
    expect(meta.siteName, 'Example & Co');
  });
}
