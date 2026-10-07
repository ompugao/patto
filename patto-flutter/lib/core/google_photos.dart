import 'embed_lookup.dart';
import '../src/rust/api/types.dart';
import '../src/rust/frb_api.dart' as rust;

/// Looks up the thumbnail behind Google Photos share links.
///
/// Google Photos has no oEmbed endpoint, so the share page is fetched and its
/// Open Graph tags parsed. See [EmbedLookup] for caching and timeouts.
class GooglePhotos {
  GooglePhotos._();

  static Future<GooglePhotosMedia?> lookup(String shareUrl) {
    return EmbedLookup.cachedPage(
      'google-photos:$shareUrl',
      shareUrl,
      (html) => rust.parseGooglePhotosPage(html: html),
    );
  }
}
