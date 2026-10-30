import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

/// Joins the selected text of its children by layout: a tab between parts on
/// the same row (table cells), a newline between rows. Flutter's own
/// containers join them with nothing, which runs lines together on copy.
class JoiningSelectionDelegate extends StaticSelectionContainerDelegate {
  JoiningSelectionDelegate({this.onChanged});

  /// Called with the selected text whenever the selection here changes; null
  /// once it is deselected. Never null before the first selection, so a null
  /// always means the user deselected.
  final void Function(String? text)? onChanged;
  String? _reported;

  @override
  SelectedContent? getSelectedContent() {
    final buffer = StringBuffer();
    double? previousTop;
    for (final selectable in selectables) {
      final content = selectable.getSelectedContent();
      if (content == null) continue;
      final top = selectable.getTransformTo(null).getTranslation().y;
      if (previousTop != null) {
        buffer.write((top - previousTop).abs() < 1 ? '\t' : '\n');
      }
      buffer.write(content.plainText);
      previousTop = top;
    }
    return previousTop == null
        ? null
        : SelectedContent(plainText: buffer.toString());
  }

  @override
  void notifyListeners() {
    super.notifyListeners();
    if (onChanged == null) return;
    var text = value.hasSelection ? getSelectedContent()?.plainText : null;
    if (text != null && text.isEmpty) text = null;
    if (text == _reported) return;
    _reported = text;
    onChanged!(text);
  }
}

/// Puts [child]'s selectable text under a [JoiningSelectionDelegate].
class JoinedSelection extends StatefulWidget {
  const JoinedSelection({
    super.key,
    this.onChanged,
    this.onMounted,
    required this.child,
  });

  final void Function(String? text)? onChanged;

  /// Called when this widget is first built. A rebuilt block starts
  /// unselected; a selection kept for it is re-applied through [onChanged].
  final VoidCallback? onMounted;
  final Widget child;

  @override
  State<JoinedSelection> createState() => _JoinedSelectionState();
}

class _JoinedSelectionState extends State<JoinedSelection> {
  late final _delegate = JoiningSelectionDelegate(
    onChanged: (text) => widget.onChanged?.call(text),
  );

  @override
  void initState() {
    super.initState();
    widget.onMounted?.call();
  }

  @override
  void dispose() {
    _delegate.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) =>
      SelectionContainer(delegate: _delegate, child: widget.child);
}

/// A horizontal scroll view whose content registers for selection with the
/// enclosing container, not with the scroll view.
///
/// A selection container inside a scroll view inside another container is
/// never selected (Flutter 3.47.4), so a [JoinedSelection] under a plain
/// [SingleChildScrollView] in a list block would drop out of every copy.
/// This gives up the scroll view's own auto-scroll while selecting.
class SelectableHorizontalScroll extends StatelessWidget {
  const SelectableHorizontalScroll({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final registrar = SelectionContainer.maybeOf(context);
    if (registrar == null) {
      return SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: child,
      );
    }
    return SelectionContainer.disabled(
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: SelectionRegistrarScope(registrar: registrar, child: child),
      ),
    );
  }
}
