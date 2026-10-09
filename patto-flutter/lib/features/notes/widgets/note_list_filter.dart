import 'package:flutter/material.dart';

import '../../../core/providers.dart';

/// The title filter and sort order above the note list.
///
/// The clear button follows the controller's text as of the last build;
/// typing refreshes the list, which rebuilds this with it.
class NoteListFilter extends StatelessWidget {
  const NoteListFilter({
    super.key,
    required this.controller,
    required this.onQueryChanged,
    required this.onClear,
    required this.sort,
    required this.onSortChanged,
  });

  final TextEditingController controller;
  final ValueChanged<String> onQueryChanged;
  final VoidCallback onClear;
  final NoteSort sort;
  final ValueChanged<NoteSort> onSortChanged;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
          child: TextField(
            controller: controller,
            onChanged: onQueryChanged,
            textInputAction: TextInputAction.search,
            decoration: InputDecoration(
              hintText: 'Filter by title',
              prefixIcon: const Icon(Icons.search),
              isDense: true,
              border: const OutlineInputBorder(),
              suffixIcon: controller.text.isEmpty
                  ? null
                  : IconButton(
                      icon: const Icon(Icons.clear),
                      onPressed: onClear,
                    ),
            ),
          ),
        ),
        Align(
          alignment: Alignment.centerLeft,
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: SegmentedButton<NoteSort>(
              showSelectedIcon: false,
              segments: const [
                ButtonSegment(value: NoteSort.recent, label: Text('Recent')),
                ButtonSegment(value: NoteSort.linked, label: Text('Linked')),
                ButtonSegment(value: NoteSort.title, label: Text('Title')),
              ],
              selected: {sort},
              onSelectionChanged: (s) => onSortChanged(s.first),
            ),
          ),
        ),
      ],
    );
  }
}
