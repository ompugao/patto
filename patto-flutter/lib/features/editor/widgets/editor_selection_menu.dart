import 'package:flutter/material.dart';
import 'package:re_editor/re_editor.dart';

/// re_editor implements the long-press menu but leaves the widget to the
/// application, so without this there is no cut, copy or paste.
Widget buildEditorSelectionMenu({
  required BuildContext context,
  required TextSelectionToolbarAnchors anchors,
  required CodeLineEditingController controller,
  required VoidCallback onDismiss,
  required VoidCallback onRefresh,
}) {
  void run(void Function() action) {
    action();
    onDismiss();
  }

  return AdaptiveTextSelectionToolbar.buttonItems(
    anchors: anchors,
    buttonItems: [
      if (!controller.selection.isCollapsed) ...[
        ContextMenuButtonItem(
          type: ContextMenuButtonType.cut,
          onPressed: () => run(controller.cut),
        ),
        ContextMenuButtonItem(
          type: ContextMenuButtonType.copy,
          onPressed: () => run(controller.copy),
        ),
      ],
      ContextMenuButtonItem(
        type: ContextMenuButtonType.paste,
        onPressed: () => run(controller.paste),
      ),
      ContextMenuButtonItem(
        type: ContextMenuButtonType.selectAll,
        onPressed: () {
          controller.selectAll();
          onRefresh();
        },
      ),
    ],
  );
}
