import 'package:flutter/material.dart';
import 'package:re_editor/re_editor.dart';

import 'outline.dart';

/// The editor's left gutter: fold markers, plus a thin line down every indent
/// column of the text beside it.
///
/// The guides are painted from the gutter, beyond its right edge, because the
/// gutter is painted before the text: they sit under the text rather than
/// over it, which matters for wrapped lines, whose continuation starts at the
/// left edge. The gutter is also the one place re_editor hands the laid-out
/// position of every visible line to.
class IndentGuideGutter extends StatelessWidget {
  const IndentGuideGutter({
    super.key,
    required this.width,
    required this.chunks,
    required this.notifier,
    required this.lines,
    required this.guide,
    required this.tabWidth,
    required this.repaint,
  });

  final double width;
  final CodeChunkController chunks;
  final CodeIndicatorValueNotifier notifier;
  final List<String> Function() lines;
  final ({int column, int start, int end})? Function() guide;
  final double tabWidth;

  /// Also repaint when this changes, e.g. the caret moving within a line.
  final Listenable repaint;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return SizedBox(
      width: width,
      child: Stack(
        clipBehavior: Clip.none,
        children: [
          Positioned.fill(
            child: CustomPaint(
              painter: _IndentGuidePainter(
                notifier: notifier,
                repaint: repaint,
                lines: lines,
                guide: guide,
                fieldLeft: width,
                tabWidth: tabWidth,
                color: scheme.outlineVariant,
                activeColor: scheme.primary,
              ),
            ),
          ),
          DefaultCodeChunkIndicator(
            width: width,
            controller: chunks,
            notifier: notifier,
          ),
        ],
      ),
    );
  }
}

class _IndentGuidePainter extends CustomPainter {
  _IndentGuidePainter({
    required this.notifier,
    required Listenable repaint,
    required this.lines,
    required this.guide,
    required this.fieldLeft,
    required this.tabWidth,
    required this.color,
    required this.activeColor,
  }) : super(repaint: Listenable.merge([notifier, repaint]));

  final CodeIndicatorValueNotifier notifier;
  final List<String> Function() lines;
  final ({int column, int start, int end})? Function() guide;

  /// Where the text field starts, measured from the gutter's left edge.
  final double fieldLeft;
  final double tabWidth;
  final Color color;
  final Color activeColor;

  @override
  void paint(Canvas canvas, Size size) {
    final paragraphs = notifier.value?.paragraphs;
    if (paragraphs == null || paragraphs.isEmpty) return;
    final lines = this.lines();
    final guide = this.guide();

    final thin = Paint()
      ..color = color
      ..strokeWidth = 1;
    final thick = Paint()
      ..color = activeColor
      ..strokeWidth = 2;

    canvas.save();
    // Lines partly scrolled out of view must not spill over the app bar or
    // the toolbar.
    canvas.clipRect(Rect.fromLTRB(0, 0, 1e5, size.height));
    for (final p in paragraphs) {
      if (p.index >= lines.length) continue;
      final top = p.offset.dy;
      final bottom = top + p.paragraph.height;
      final active =
          guide != null && p.index >= guide.start && p.index < guide.end;
      for (var c = guideDepth(lines, p.index) - 1; c >= 0; c--) {
        // Under the first character of the parent's text.
        final x = fieldLeft + p.offset.dx + c * tabWidth + 1;
        final isActive = active && c == guide.column;
        canvas.drawLine(
          Offset(x, top),
          Offset(x, bottom),
          isActive ? thick : thin,
        );
      }
    }
    canvas.restore();
  }

  @override
  bool shouldRepaint(_IndentGuidePainter old) => true;
}
