import 'package:flutter/material.dart';
import 'package:intl/intl.dart';

import '../../../src/rust/api/types.dart';
import '../note_view_screen.dart';

class NoteTile extends StatelessWidget {
  const NoteTile({
    super.key,
    required this.note,
    required this.backlinks,
    required this.conflicted,
  });

  final NoteMeta note;
  final int backlinks;

  /// Changed on both sides and waiting for a merge.
  final bool conflicted;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final modified = DateTime.fromMillisecondsSinceEpoch(note.modifiedMs);

    return ListTile(
      title: Row(
        children: [
          if (conflicted)
            Padding(
              padding: const EdgeInsets.only(right: 6),
              child: Icon(
                Icons.warning_amber_rounded,
                size: 18,
                color: Colors.amber.shade800,
              ),
            ),
          Flexible(child: Text(note.name, overflow: TextOverflow.ellipsis)),
        ],
      ),
      subtitle: Text(DateFormat.yMMMd().add_Hm().format(modified)),
      trailing: backlinks == 0
          ? null
          : Chip(
              label: Text('$backlinks'),
              visualDensity: VisualDensity.compact,
              labelStyle: theme.textTheme.labelSmall,
            ),
      onTap: () => NoteViewScreen.open(context, note.relPath),
    );
  }
}

/// A message where the list would be, scrollable so pull-to-refresh works.
class NoteListMessage extends StatelessWidget {
  const NoteListMessage(this.text, {super.key});

  final String text;

  @override
  Widget build(BuildContext context) {
    return ListView(
      children: [
        Padding(
          padding: const EdgeInsets.all(32),
          child: Text(text, textAlign: TextAlign.center),
        ),
      ],
    );
  }
}
