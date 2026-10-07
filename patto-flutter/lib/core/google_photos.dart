import 'dart:async';
import 'dart:convert';
import 'dart:io';

import '../src/rust/api/types.dart';
import '../src/rust/frb_api.dart' as rust;

/// Looks up the thumbnail behind Google Photos share links.
///
/// Google Photos has no oEmbed endpoint, so the share page is fetched and its
/// Open Graph tags parsed. Lookups are shared across every card showing the
/// same link and kept for the life of the app; a failure (offline, unshared,
/// rate-limited) resolves to `null` and is retried only after a pause, so
/// scrolling back and forth past a broken embed does not keep hitting the
/// network.
class GooglePhotos {
  GooglePhotos._();

  static const _connectTimeout = Duration(seconds: 5);
  static const _timeout = Duration(seconds: 10);
  static const _retryAfter = Duration(minutes: 1);

  /// The Open Graph tags sit in `<head>`; nothing past it is needed.
  static const _maxBytes = 2 * 1024 * 1024;

  static final _lookups = <String, Future<GooglePhotosMedia?>>{};
  static HttpClient? _client;

  static Future<GooglePhotosMedia?> lookup(String shareUrl) {
    return _lookups.putIfAbsent(shareUrl, () {
      final pending = _fetch(shareUrl);
      pending.then((media) {
        if (media == null) {
          Timer(_retryAfter, () => _lookups.remove(shareUrl));
        }
      });
      return pending;
    });
  }

  static Future<GooglePhotosMedia?> _fetch(String shareUrl) async {
    final uri = Uri.tryParse(shareUrl);
    if (uri == null) return null;
    try {
      final html = await _readHead(uri).timeout(_timeout);
      if (html == null) return null;
      return await rust.parseGooglePhotosPage(html: html);
    } catch (_) {
      return null;
    }
  }

  static Future<String?> _readHead(Uri uri) async {
    final client = _client ??= HttpClient()
      ..connectionTimeout = _connectTimeout
      ..userAgent = rust.googlePhotosUserAgent();
    final request = await client.getUrl(uri);
    final response = await request.close();
    if (response.statusCode != HttpStatus.ok) {
      unawaited(response.drain<void>().catchError((_) {}));
      return null;
    }

    const end = '</head>';
    final html = StringBuffer();
    var size = 0;
    var tail = '';
    await for (final chunk in response.transform(
      const Utf8Decoder(allowMalformed: true),
    )) {
      html.write(chunk);
      size += chunk.length;
      // Carry the previous chunk's end over, in case the tag straddles two.
      final seen = tail + chunk;
      if (seen.contains(end) || size > _maxBytes) break;
      tail = seen.substring(
        seen.length - (end.length - 1).clamp(0, seen.length),
      );
    }
    return html.toString();
  }
}
