import 'package:flutter/material.dart';
import 'package:pdfrx/pdfrx.dart';

/// Full-screen PDF viewer, for files in the notes repository and on the web.
///
/// Rendered in-app because handing a `file://` path to another app is refused
/// on Android, and a note's `./paper.pdf` is not a URL a browser can open.
/// PDFium is only touched once this screen opens, never while a note scrolls.
class PdfViewerScreen extends StatelessWidget {
  const PdfViewerScreen.file({
    super.key,
    required this.path,
    required this.title,
  }) : uri = null;

  const PdfViewerScreen.uri({super.key, required this.uri, required this.title})
    : path = null;

  final String? path;
  final Uri? uri;
  final String title;

  static void open(BuildContext context, PdfViewerScreen screen) {
    Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => screen));
  }

  @override
  Widget build(BuildContext context) {
    final params = PdfViewerParams(
      errorBannerBuilder: (context, error, _, _) => Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Text('Could not open this PDF.\n\n$error'),
        ),
      ),
    );

    return Scaffold(
      appBar: AppBar(title: Text(title, overflow: TextOverflow.ellipsis)),
      body: path != null
          ? PdfViewer.file(path!, params: params)
          : PdfViewer.uri(uri!, params: params),
    );
  }
}
