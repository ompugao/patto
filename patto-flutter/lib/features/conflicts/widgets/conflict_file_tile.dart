import 'package:flutter/material.dart';

import '../../../src/rust/api/conflict.dart';
import '../../../src/rust/frb_api.dart' as rust;
import '../conflict_note_screen.dart';
import '../conflict_state.dart';
import '../conflict_text.dart';

/// One clashing note: how far its review has got, and what each side did.
class ConflictFileTile extends StatelessWidget {
  const ConflictFileTile({super.key, required this.file, required this.draft});

  final ConflictFile file;
  final NoteDraft? draft;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final done = draft?.done ?? false;
    final started = !done && (draft?.choices.isNotEmpty ?? false);

    return ListTile(
      leading: Icon(
        done
            ? Icons.check_circle
            : started
            ? Icons.timelapse
            : Icons.warning_amber_rounded,
        color: done ? theme.colorScheme.primary : Colors.amber.shade800,
      ),
      title: Text(rust.relPathToNoteName(relPath: file.path)),
      subtitle: Text(conflictFileSubtitle(file)),
      isThreeLine: !isWholeNote(file.kind),
      trailing: const Icon(Icons.chevron_right),
      onTap: () => ConflictNoteScreen.open(context, file.path),
    );
  }
}
