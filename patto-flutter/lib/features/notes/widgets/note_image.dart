import 'dart:io';
import 'dart:math' as math;

import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';

import '../../../src/rust/api/types.dart';

String? _resolveLocalPath(ImageRef image, String? root) {
  if (!image.isLocal || root == null) return null;
  final src = image.src.startsWith('./') ? image.src.substring(2) : image.src;
  return '$root/$src';
}

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

  String? get _localPath => _resolveLocalPath(image, root);

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
///
/// The image is decoded at full resolution here so that zooming stays sharp,
/// and a double tap toggles between fitting the screen and filling it, which
/// matters for wide images that otherwise show as a thin strip.
class ImageLightbox extends StatefulWidget {
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
  State<ImageLightbox> createState() => _ImageLightboxState();
}

class _ImageLightboxState extends State<ImageLightbox>
    with SingleTickerProviderStateMixin {
  static const _minZoomStep = 1.05;

  final TransformationController _controller = TransformationController();
  late final AnimationController _animation = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 200),
  )..addListener(_applyAnimation);
  Animation<Matrix4>? _zoom;

  late final ImageProvider _provider;
  ImageStream? _stream;
  ImageStreamListener? _listener;
  double? _aspectRatio;
  Offset? _doubleTapPosition;

  @override
  void initState() {
    super.initState();
    final path = _resolveLocalPath(widget.image, widget.root);
    _provider = path != null
        ? FileImage(File(path))
        : CachedNetworkImageProvider(widget.image.src) as ImageProvider;
    _stream = _provider.resolve(ImageConfiguration.empty);
    _listener = ImageStreamListener((info, _) {
      final ratio = info.image.width / info.image.height;
      if (mounted && _aspectRatio != ratio) {
        setState(() => _aspectRatio = ratio);
      }
    }, onError: (_, _) {});
    _stream!.addListener(_listener!);
  }

  @override
  void dispose() {
    _stream?.removeListener(_listener!);
    _animation.dispose();
    _controller.dispose();
    super.dispose();
  }

  void _applyAnimation() {
    final zoom = _zoom;
    if (zoom != null) _controller.value = zoom.value;
  }

  /// Scale that makes the fitted image cover the viewport on both axes.
  double _coverScale(Size viewport) {
    final ratio = _aspectRatio;
    if (ratio == null || viewport.isEmpty) return 1;
    final fitted = applyBoxFit(BoxFit.contain, Size(ratio, 1), viewport);
    if (fitted.destination.isEmpty) return 1;
    return math.max(
      viewport.width / fitted.destination.width,
      viewport.height / fitted.destination.height,
    );
  }

  void _onDoubleTap(Size viewport) {
    final current = _controller.value.getMaxScaleOnAxis();
    final Matrix4 target;
    if (current > _minZoomStep) {
      target = Matrix4.identity();
    } else {
      final cover = _coverScale(viewport);
      final scale = cover > _minZoomStep ? cover : 2.5;
      final focus = _doubleTapPosition ?? viewport.center(Offset.zero);
      target = Matrix4.identity()
        ..translateByDouble(focus.dx, focus.dy, 0, 1)
        ..scaleByDouble(scale, scale, 1, 1)
        ..translateByDouble(-focus.dx, -focus.dy, 0, 1);
    }
    _zoom = Matrix4Tween(begin: _controller.value, end: target).animate(
      CurvedAnimation(parent: _animation, curve: Curves.easeOutCubic),
    );
    _animation.forward(from: 0);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.black,
      extendBodyBehindAppBar: true,
      appBar: AppBar(
        backgroundColor: Colors.black38,
        foregroundColor: Colors.white,
        elevation: 0,
        title: Text(widget.image.alt ?? '', overflow: TextOverflow.ellipsis),
      ),
      body: LayoutBuilder(
        builder: (context, constraints) {
          final viewport = constraints.biggest;
          final cover = _coverScale(viewport);
          return GestureDetector(
            onDoubleTapDown: (details) =>
                _doubleTapPosition = details.localPosition,
            onDoubleTap: () => _onDoubleTap(viewport),
            child: InteractiveViewer(
              transformationController: _controller,
              maxScale: math.max(8, cover * 4),
              child: SizedBox.expand(
                child: Image(
                  image: _provider,
                  fit: BoxFit.contain,
                  filterQuality: FilterQuality.medium,
                  errorBuilder: (context, _, _) =>
                      _Broken(alt: widget.image.alt ?? widget.image.src),
                ),
              ),
            ),
          );
        },
      ),
    );
  }
}
