import 'dart:async';
import 'dart:convert';
import 'dart:io';

import '../src/rust/frb_api.dart' as rust;

/// The network side of embed cards: one fetch per link, shared and kept for
/// the life of the app.
///
/// A failure (offline, removed, rate-limited) resolves to `null` and is
/// forgotten only after a pause, so scrolling back and forth past a broken
/// embed does not keep hitting the network. Every request has a connect and
/// a total timeout, so a stalled network degrades a card instead of hanging it.
class EmbedLookup {
  EmbedLookup._();

  static const connectTimeout = Duration(seconds: 5);
  static const timeout = Duration(seconds: 10);
  static const retryAfter = Duration(minutes: 1);

  /// Open Graph tags sit in `<head>`, and an oEmbed response is a few KB.
  static const maxBytes = 2 * 1024 * 1024;

  static final _lookups = <String, Future<Object?>>{};
  static HttpClient? _client;

  static Future<T?> cached<T extends Object>(
    String key,
    Future<T?> Function() fetch,
  ) {
    final pending = _lookups.putIfAbsent(key, () {
      final result = _guard(fetch);
      result.then((value) {
        if (value == null) {
          Timer(retryAfter, () => _lookups.remove(key));
        }
      });
      return result;
    });
    return pending as Future<T?>;
  }

  static Future<T?> _guard<T extends Object>(
    Future<T?> Function() fetch,
  ) async {
    try {
      return await fetch().timeout(timeout);
    } catch (_) {
      return null;
    }
  }

  static HttpClient get _http => _client ??= HttpClient()
    ..connectionTimeout = connectTimeout
    ..userAgent = rust.browserUserAgent();

  /// Fetches the page at `url` and hands its `<head>` to `parse`, for the
  /// lookups that read a page's Open Graph tags.
  static Future<T?> cachedPage<T extends Object>(
    String key,
    String url,
    Future<T?> Function(String html) parse,
  ) {
    return cached(key, () async {
      final uri = Uri.tryParse(url);
      if (uri == null || !uri.hasScheme) return null;
      final html = await fetchHead(uri);
      return html == null ? null : await parse(html);
    });
  }

  /// The start of a page up to `</head>`, which holds its Open Graph tags.
  static Future<String?> fetchHead(Uri uri) async {
    final response = await _get(uri);
    if (response == null) return null;

    const end = '</head>';
    final html = StringBuffer();
    var size = 0;
    var tail = '';
    await for (final chunk in _text(response)) {
      html.write(chunk);
      size += chunk.length;
      // Carry the previous chunk's end over, in case the tag straddles two.
      final seen = tail + chunk;
      if (seen.contains(end) || size > maxBytes) break;
      tail = seen.substring(
        seen.length - (end.length - 1).clamp(0, seen.length),
      );
    }
    return html.toString();
  }

  static Future<Map<String, Object?>?> fetchJson(Uri uri) async {
    final response = await _get(uri);
    if (response == null) return null;
    final body = StringBuffer();
    await for (final chunk in _text(response)) {
      body.write(chunk);
      if (body.length > maxBytes) return null;
    }
    final decoded = jsonDecode(body.toString());
    return decoded is Map<String, Object?> ? decoded : null;
  }

  static Stream<String> _text(HttpClientResponse response) =>
      response.transform(const Utf8Decoder(allowMalformed: true));

  /// A successful response in UTF-8. Bodies are decoded as UTF-8 and a lookup
  /// is kept for the life of the app, so a page declaring another charset is
  /// refused rather than cached as garbled text.
  static Future<HttpClientResponse?> _get(Uri uri) async {
    final request = await _http.getUrl(uri);
    final response = await request.close();
    final charset = response.headers.contentType?.charset?.toLowerCase();
    final utf8 = charset == null || charset == 'utf-8' || charset == 'utf8';
    if (response.statusCode != HttpStatus.ok || !utf8) {
      unawaited(response.drain<void>().catchError((_) {}));
      return null;
    }
    return response;
  }
}
