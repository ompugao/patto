import 'package:flutter/material.dart';

import '../../../src/rust/api/types.dart';
import 'note_image.dart';
import 'spans_text.dart';
import 'task_marker.dart';

/// A line of prose: its spans, with a task marker or bullet in front and its
/// anchors under it.
class LineBlock extends StatelessWidget {
  const LineBlock({
    super.key,
    required this.block,
    required this.spans,
    required this.actions,
    required this.root,
    required this.textScale,
    this.searchTerm,
    this.onTaskTap,
  });

  final Block block;
  final List<NoteSpan> spans;
  final SpanActions actions;
  final String? root;
  final double textScale;
  final String? searchTerm;
  final void Function(Block block)? onTaskTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final task = block.task;
    final done = task?.status == TaskStatus.done;
    final base = theme.textTheme.bodyLarge!;

    final text = SpansText(
      spans: spans,
      actions: actions,
      noteRoot: root,
      style: base.copyWith(fontSize: (base.fontSize ?? 16) * textScale),
      strikeThrough: done,
      searchTerm: searchTerm,
      trailing: task?.due != null && !done
          ? WidgetSpan(
              alignment: PlaceholderAlignment.middle,
              child: DueChip(due: task!.due!, textScale: textScale),
            )
          : null,
    );

    final anchors = block.anchors;
    final line = anchors.isEmpty
        ? text
        : Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              text,
              Padding(
                padding: const EdgeInsets.only(top: 2),
                child: Text(
                  anchors.map((a) => '#$a').join(' '),
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: theme.colorScheme.outline,
                    fontSize:
                        (theme.textTheme.labelSmall?.fontSize ?? 11) *
                        textScale,
                  ),
                ),
              ),
            ],
          );

    if (task == null && block.depth == 0) return line;

    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (task != null) ...[
          Padding(
            padding: const EdgeInsets.only(top: 3, right: 6),
            child: TaskMarker(
              status: task.status,
              textScale: textScale,
              onTap: onTaskTap == null ? null : () => onTaskTap!(block),
            ),
          ),
        ] else if (block.depth > 0) ...[
          Padding(
            padding: const EdgeInsets.only(top: 8, right: 8),
            child: Container(
              width: 4,
              height: 4,
              decoration: BoxDecoration(
                color: theme.colorScheme.outlineVariant,
                shape: BoxShape.circle,
              ),
            ),
          ),
        ],
        Expanded(child: line),
      ],
    );
  }
}

/// The images of an `[@img ...]` line, each with its caption.
class ImagesBlock extends StatelessWidget {
  const ImagesBlock({
    super.key,
    required this.images,
    required this.root,
    required this.textScale,
  });

  final List<ImageRef> images;
  final String? root;
  final double textScale;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        for (final image in images)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 4),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                GestureDetector(
                  onTap: () => ImageLightbox.open(context, image, root),
                  child: NoteImage(image: image, root: root),
                ),
                if (image.alt != null)
                  Padding(
                    padding: const EdgeInsets.only(top: 4),
                    child: Text(
                      image.alt!,
                      // A caption, not prose: muted so it reads as a label on
                      // the image rather than as part of the note.
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.outline,
                        fontSize:
                            (theme.textTheme.bodySmall?.fontSize ?? 12) *
                            textScale,
                      ),
                    ),
                  ),
              ],
            ),
          ),
      ],
    );
  }
}
