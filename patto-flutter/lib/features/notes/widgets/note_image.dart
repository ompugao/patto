import 'dart:io';

import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';

import '../../../src/rust/api/types.dart';

/// An image from a note, decoded at display size so a large photo never costs
/// full-resolution memory.
class NoteImage extends StatelessWidget {
  const NoteImage({
    super.key,
    required this.image,
    required this.root,
    this.inline = false,
  });

  final ImageRef image;
  final String? root;
  final bool inline;

  String? get _localPath {
    if (!image.isLocal || root == null) return null;
    final src = image.src.startsWith('./') ? image.src.substring(2) : image.src;
    return '$root/$src';
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final dpr = MediaQuery.devicePixelRatioOf(context);
        final width = constraints.maxWidth.isFinite
            ? (constraints.maxWidth * dpr).round()
            : (MediaQuery.sizeOf(context).width * dpr).round();

        final path = _localPath;
        if (path != null) {
          return Image.file(
            File(path),
            cacheWidth: width,
            fit: inline ? BoxFit.contain : BoxFit.fitWidth,
            gaplessPlayback: true,
            errorBuilder: (context, _, _) => _Broken(alt: image.alt ?? image.src),
          );
        }

        return CachedNetworkImage(
          imageUrl: image.src,
          memCacheWidth: width,
          fit: inline ? BoxFit.contain : BoxFit.fitWidth,
          placeholder: (context, _) => const _Placeholder(),
          errorWidget: (context, _, _) => _Broken(alt: image.alt ?? image.src),
        );
      },
    );
  }
}

class _Placeholder extends StatelessWidget {
  const _Placeholder();

  @override
  Widget build(BuildContext context) {
    return Container(
      // A fixed height keeps the scroll extent stable while decoding.
      height: 180,
      color: Theme.of(context).colorScheme.surfaceContainerHighest,
      alignment: Alignment.center,
      child: const SizedBox(
        width: 20,
        height: 20,
        child: CircularProgressIndicator(strokeWidth: 2),
      ),
    );
  }
}

class _Broken extends StatelessWidget {
  const _Broken({required this.alt});

  final String alt;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        border: Border.all(color: theme.colorScheme.outlineVariant),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.broken_image_outlined, color: theme.colorScheme.outline),
          const SizedBox(width: 8),
          Flexible(
            child: Text(
              alt,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.outline,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}

/// Full-screen viewer with pinch-to-zoom.
class ImageLightbox extends StatelessWidget {
  const ImageLightbox({super.key, required this.image, required this.root});

  final ImageRef image;
  final String? root;

  static void open(BuildContext context, ImageRef image, String? root) {
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        fullscreenDialog: true,
        builder: (_) => ImageLightbox(image: image, root: root),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.black,
      appBar: AppBar(
        backgroundColor: Colors.black,
        foregroundColor: Colors.white,
        title: Text(image.alt ?? '', overflow: TextOverflow.ellipsis),
      ),
      body: Center(
        child: InteractiveViewer(
          maxScale: 6,
          child: NoteImage(image: image, root: root),
        ),
      ),
    );
  }
}
