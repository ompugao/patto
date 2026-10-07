import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/core/oembed.dart';

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
}
