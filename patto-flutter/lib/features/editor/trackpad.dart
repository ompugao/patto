import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:re_editor/re_editor.dart';

/// Moves the caret by relative finger movement, as iOS does when the space
/// bar is held.
///
/// A fingertip covers several characters, so a tap rarely lands the caret
/// where it was aimed. Dragging somewhere else instead keeps the text in
/// view, and the caret follows the finger's movement rather than its
/// position. The keyboard's own space-bar drag cannot do this across lines:
/// re_editor shows the keyboard one line at a time.
class CaretTrackpad {
  CaretTrackpad({required this.controller, required this.paragraphs});

  final CodeLineEditingController controller;

  /// The editor's laid-out visible lines, in its text field's coordinates.
  final List<CodeLineRenderParagraph>? Function() paragraphs;

  /// Where the finger would put the caret horizontally, in the text field's
  /// coordinates; kept across lines so that moving up and down holds the
  /// column, and not clamped to a short line on the way.
  double? _x;
  double _dy = 0;

  void start() {
    _x = null;
    _dy = 0;
  }

  void move(Offset delta) {
    // Moving the caret under a composition would break it; typing Japanese
    // keeps one open until the word is committed.
    if (controller.isComposing) return;

    var paragraph = _caretParagraph();
    final lineHeight = paragraph?.preferredLineHeight ?? 20;
    _x ??= paragraph == null ? null : _caretX(paragraph);

    _dy += delta.dy;
    var lines = 0;
    while (_dy.abs() >= lineHeight) {
      final up = _dy < 0;
      controller.moveCursor(up ? AxisDirection.up : AxisDirection.down);
      _dy += up ? lineHeight : -lineHeight;
      lines++;
    }
    if (lines > 0) {
      HapticFeedback.selectionClick();
      paragraph = _caretParagraph();
    }

    final x = _x;
    if (x == null || paragraph == null) {
      // The caret is off screen; bring it back and aim from there next time.
      controller.makeCursorVisible();
      return;
    }
    _x = x + delta.dx;
    _place(paragraph);
    controller.makeCursorVisible();
  }

  CodeLineRenderParagraph? _caretParagraph() {
    final index = controller.selection.extentIndex;
    for (final p in paragraphs() ?? const <CodeLineRenderParagraph>[]) {
      if (p.index == index) return p;
    }
    return null;
  }

  double _caretX(CodeLineRenderParagraph p) {
    final caret = p.paragraph.getOffset(_caretPosition()) ?? Offset.zero;
    return p.offset.dx + caret.dx;
  }

  TextPosition _caretPosition() => TextPosition(
    offset: controller.selection.extentOffset,
    affinity: controller.selection.extentAffinity,
  );

  /// Puts the caret on its current visual line, nearest to [_x].
  void _place(CodeLineRenderParagraph p) {
    final caret = _caretPosition();
    final top = p.paragraph.getOffset(caret)?.dy ?? 0;
    final y = top + p.preferredLineHeight / 2;

    // Past either end of the line, the finger has to come back only as far
    // as the end, not all the way it overshot.
    final line = p.paragraph.getLineBoundary(caret);
    final minX =
        p.paragraph.getOffset(TextPosition(offset: line.start))?.dx ?? 0;
    final maxX =
        p.paragraph
            .getOffset(
              TextPosition(offset: line.end, affinity: TextAffinity.upstream),
            )
            ?.dx ??
        minX;
    final x = _x!.clamp(p.offset.dx + minX, p.offset.dx + maxX);
    _x = x;

    final target = p.paragraph.getPosition(Offset(x - p.offset.dx, y));
    if (target.offset == caret.offset) return;
    controller.selection = CodeLineSelection.collapsed(
      index: p.index,
      offset: target.offset,
      affinity: target.affinity,
    );
  }
}

/// Turns [child] into a trackpad for [trackpad] while it is long-pressed.
///
/// The press is shorter than the one that shows tooltips, so it wins over
/// the buttons inside; a quick swipe still scrolls.
class TrackpadRegion extends StatefulWidget {
  const TrackpadRegion({
    super.key,
    required this.trackpad,
    required this.child,
  });

  final CaretTrackpad trackpad;
  final Widget child;

  @override
  State<TrackpadRegion> createState() => _TrackpadRegionState();
}

class _TrackpadRegionState extends State<TrackpadRegion> {
  bool _active = false;
  Offset _last = Offset.zero;

  void _start() {
    HapticFeedback.mediumImpact();
    widget.trackpad.start();
    setState(() => _active = true);
  }

  void _end() {
    if (mounted) setState(() => _active = false);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return RawGestureDetector(
      gestures: {
        LongPressGestureRecognizer:
            GestureRecognizerFactoryWithHandlers<LongPressGestureRecognizer>(
              () => LongPressGestureRecognizer(
                duration: const Duration(milliseconds: 300),
              ),
              (recognizer) {
                recognizer
                  ..onLongPressStart = (_) {
                    _last = Offset.zero;
                    _start();
                  }
                  ..onLongPressMoveUpdate = (details) {
                    widget.trackpad.move(details.offsetFromOrigin - _last);
                    _last = details.offsetFromOrigin;
                  }
                  ..onLongPressEnd = (_) {
                    _end();
                  }
                  ..onLongPressCancel = _end;
              },
            ),
      },
      child: Stack(
        children: [
          AnimatedOpacity(
            opacity: _active ? 0.15 : 1,
            duration: const Duration(milliseconds: 120),
            child: IgnorePointer(ignoring: _active, child: widget.child),
          ),
          if (_active)
            Positioned.fill(
              child: Center(
                child: Text(
                  'Drag to move the cursor',
                  style: theme.textTheme.labelLarge,
                ),
              ),
            ),
        ],
      ),
    );
  }
}

/// A handle that moves the caret as soon as it is dragged, no hold needed.
class TrackpadHandle extends StatelessWidget {
  const TrackpadHandle({super.key, required this.trackpad});

  final CaretTrackpad trackpad;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: 'Drag to move the cursor',
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onPanStart: (_) => trackpad.start(),
        onPanUpdate: (details) => trackpad.move(details.delta),
        child: SizedBox(
          width: 48,
          height: 48,
          child: Icon(
            Icons.open_with,
            color: Theme.of(context).colorScheme.primary,
          ),
        ),
      ),
    );
  }
}
