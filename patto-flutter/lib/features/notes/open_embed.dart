import 'package:flutter/widgets.dart';

import '../../src/rust/api/types.dart';
import 'widgets/embed_viewer_screen.dart';
import 'widgets/note_image.dart';
import 'widgets/pdf_viewer_screen.dart';

/// Opens an embed the way it can be shown. PDFs open in the app, local ones
/// because nothing else can reach them. Videos and Google Photos shares go
/// to the app that owns them, since neither can be framed; everything else
/// opens in the in-app web view, as the web preview frames it.
void openEmbed(
  BuildContext context,
  EmbedRef embed,
  String? root, {
  required void Function(String url) openUrl,
}) {
  final title = embed.title ?? embed.url.split('/').last;
  switch (embed.kind) {
    case EmbedKind_Pdf():
      if (embed.isLocal) {
        if (root == null) return;
        PdfViewerScreen.open(
          context,
          PdfViewerScreen.file(
            path: resolveNotePath(embed.url, root),
            title: title,
          ),
        );
        return;
      }
      final uri = Uri.tryParse(embed.url);
      if (uri != null) {
        PdfViewerScreen.open(
          context,
          PdfViewerScreen.uri(uri: uri, title: title),
        );
        return;
      }
      openUrl(embed.url);
    case EmbedKind_Youtube() || EmbedKind_GooglePhotos():
      openUrl(embed.url);
    case _ when embed.isLocal:
      openUrl(embed.url);
    default:
      EmbedViewerScreen.open(context, embed);
  }
}
