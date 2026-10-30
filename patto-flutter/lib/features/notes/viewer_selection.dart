import 'selection_range.dart';

/// The viewer's text selection, kept per block so that blocks scrolled out
/// of the list still count.
class ViewerSelection {
  final _texts = <int, String>{};
  int? _latest;
  bool _all = false;
  bool _deselected = false;

  /// Whether a selection exists, including one whose blocks have all
  /// scrolled out of the list.
  bool get active => _active;
  bool _active = false;

  /// The selected text per block, or null when the whole note is selected.
  Map<int, String>? get texts => _all ? null : Map.of(_texts);

  void blockChanged(int index, String? text) {
    // A block re-selected after scrolling back reports the same text; any
    // other change means a handle moved or the selection was cleared.
    if (_all && _texts[index] != text) _all = false;
    if (text == null) {
      if (_texts.remove(index) != null) _deselected = true;
    } else {
      _texts[index] = text;
      _latest = index;
    }
  }

  /// A rebuilt block starts unselected; Flutter re-applies a kept selection
  /// through [blockChanged].
  void blockMounted(int index) {
    if (!_all) _texts.remove(index);
  }

  /// Flutter reports no selection both when it is cleared and when every
  /// selected block has scrolled out of the list. Only a clear deselects
  /// blocks first.
  void regionChanged({required bool hasText}) {
    final scrolledAway =
        !hasText && !_deselected && (_all || _texts.isNotEmpty);
    _deselected = false;
    if (hasText || scrolledAway) {
      _active = true;
    } else {
      clear();
    }
  }

  /// Flutter's Select all selects only the blocks the list has built.
  void selectAll() {
    _all = true;
    _active = true;
  }

  void clear() {
    _texts.clear();
    _latest = null;
    _all = false;
    _deselected = false;
    _active = false;
  }

  (int, int)? range(int blockCount, bool Function(int index) hasText) {
    if (blockCount == 0) return null;
    if (_all) return (0, blockCount - 1);
    return selectedRange(_texts, _latest, blockCount, hasText);
  }
}
