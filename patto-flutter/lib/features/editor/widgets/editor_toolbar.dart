import 'package:flutter/material.dart';

import '../trackpad.dart';

/// The bar under the editor: block operations, undo, attachments and the
/// markup snippets a soft keyboard makes awkward to type.
class EditorToolbar extends StatelessWidget {
  const EditorToolbar({
    super.key,
    required this.trackpad,
    required this.onIndent,
    required this.onOutdent,
    required this.onInsert,
    required this.onAttach,
    required this.onPaste,
    required this.onMoveBlock,
    required this.onSelectBlock,
    required this.onUndo,
    required this.onRedo,
    required this.today,
  });

  final CaretTrackpad trackpad;
  final VoidCallback onIndent;
  final VoidCallback onOutdent;
  final void Function(String text, [int back]) onInsert;
  final VoidCallback onAttach;
  final VoidCallback onPaste;
  final void Function(bool up) onMoveBlock;
  final VoidCallback onSelectBlock;
  final VoidCallback onUndo;
  final VoidCallback onRedo;
  final String Function() today;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      color: theme.colorScheme.surfaceContainer,
      child: SafeArea(
        top: false,
        child: SizedBox(
          height: 48,
          // Holding the bar, or dragging its handle, moves the caret.
          child: TrackpadRegion(
            trackpad: trackpad,
            child: Row(
              children: [
                TrackpadHandle(trackpad: trackpad),
                IconButton(
                  icon: const Icon(Icons.format_indent_increase),
                  tooltip: 'Indent block',
                  onPressed: onIndent,
                ),
                IconButton(
                  icon: const Icon(Icons.format_indent_decrease),
                  tooltip: 'Outdent block',
                  onPressed: onOutdent,
                ),
                const VerticalDivider(width: 8),
                Expanded(
                  child: ListView(
                    scrollDirection: Axis.horizontal,
                    children: [
                      IconButton(
                        icon: const Icon(Icons.move_up),
                        tooltip: 'Move block up',
                        onPressed: () => onMoveBlock(true),
                      ),
                      IconButton(
                        icon: const Icon(Icons.move_down),
                        tooltip: 'Move block down',
                        onPressed: () => onMoveBlock(false),
                      ),
                      IconButton(
                        icon: const Icon(Icons.highlight_alt),
                        tooltip: 'Select block',
                        onPressed: onSelectBlock,
                      ),
                      const VerticalDivider(width: 8),
                      IconButton(
                        icon: const Icon(Icons.undo),
                        tooltip: 'Undo',
                        onPressed: onUndo,
                      ),
                      IconButton(
                        icon: const Icon(Icons.redo),
                        tooltip: 'Redo',
                        onPressed: onRedo,
                      ),
                      const VerticalDivider(width: 8),
                      IconButton(
                        icon: const Icon(Icons.attach_file),
                        tooltip: 'Attach a file',
                        onPressed: onAttach,
                      ),
                      IconButton(
                        icon: const Icon(Icons.content_paste_go),
                        tooltip: 'Paste as image, link or embed',
                        onPressed: onPaste,
                      ),
                      const VerticalDivider(width: 8),
                      TextButton(
                        onPressed: () => onInsert('[]', 1),
                        child: const Text('[ ]'),
                      ),
                      TextButton(
                        onPressed: () => onInsert('{@task status=todo}'),
                        child: const Text('task'),
                      ),
                      TextButton(
                        onPressed: () => onInsert('!${today()}'),
                        child: const Text('due'),
                      ),
                      TextButton(
                        onPressed: () => onInsert('[@code ]', 1),
                        child: const Text('code'),
                      ),
                      TextButton(
                        onPressed: () => onInsert('[@quote]'),
                        child: const Text('quote'),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
