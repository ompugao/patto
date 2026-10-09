import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:re_editor/re_editor.dart';

/// Moves the caret on arrow keys from a virtual keyboard, such as Gboard's
/// cursor buttons. On Android and iOS re_editor leaves those keys unhandled,
/// so the app's default shortcuts would turn them into focus traversal and
/// the caret would stay put.
class CaretKeys extends StatelessWidget {
  const CaretKeys({super.key, required this.controller, required this.child});

  final CodeLineEditingController controller;
  final Widget child;

  static const _shortcuts = <ShortcutActivator, Intent>{
    SingleActivator(LogicalKeyboardKey.arrowLeft): _MoveCaret(
      AxisDirection.left,
    ),
    SingleActivator(LogicalKeyboardKey.arrowRight): _MoveCaret(
      AxisDirection.right,
    ),
    SingleActivator(LogicalKeyboardKey.arrowUp): _MoveCaret(AxisDirection.up),
    SingleActivator(LogicalKeyboardKey.arrowDown): _MoveCaret(
      AxisDirection.down,
    ),
    SingleActivator(LogicalKeyboardKey.arrowLeft, shift: true):
        _ExtendSelection(AxisDirection.left),
    SingleActivator(LogicalKeyboardKey.arrowRight, shift: true):
        _ExtendSelection(AxisDirection.right),
    SingleActivator(LogicalKeyboardKey.arrowUp, shift: true): _ExtendSelection(
      AxisDirection.up,
    ),
    SingleActivator(LogicalKeyboardKey.arrowDown, shift: true):
        _ExtendSelection(AxisDirection.down),
  };

  @override
  Widget build(BuildContext context) {
    return Shortcuts(
      shortcuts: _shortcuts,
      child: Actions(
        actions: {
          _MoveCaret: CallbackAction<_MoveCaret>(
            onInvoke: (intent) {
              // Moving the caret would break an uncommitted composition.
              if (!controller.isComposing) {
                controller.moveCursor(intent.direction);
              }
              return null;
            },
          ),
          _ExtendSelection: CallbackAction<_ExtendSelection>(
            onInvoke: (intent) {
              if (!controller.isComposing) {
                controller.extendSelection(intent.direction);
              }
              return null;
            },
          ),
        },
        child: child,
      ),
    );
  }
}

class _MoveCaret extends Intent {
  const _MoveCaret(this.direction);

  final AxisDirection direction;
}

class _ExtendSelection extends Intent {
  const _ExtendSelection(this.direction);

  final AxisDirection direction;
}
