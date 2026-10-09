import 'package:flutter/material.dart';

/// Whether to edit a note that is still waiting to be merged.
Future<bool?> confirmEditingConflicted(BuildContext context) {
  return showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: const Text('Waiting to be merged'),
      content: const Text(
        'This note changed both here and on another device. New edits '
        'become part of your side and will need merging too.',
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, true),
          child: const Text('Edit anyway'),
        ),
      ],
    ),
  );
}

enum UnsavedChoice { cancel, discard, save }

/// What to do with unsaved changes on the way out. Null when dismissed,
/// which counts as cancelling.
Future<UnsavedChoice?> askAboutUnsavedChanges(BuildContext context) {
  return showDialog<UnsavedChoice>(
    context: context,
    builder: (context) => AlertDialog(
      title: const Text('Unsaved changes'),
      content: const Text('Save before leaving?'),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, UnsavedChoice.cancel),
          child: const Text('Cancel'),
        ),
        TextButton(
          onPressed: () => Navigator.pop(context, UnsavedChoice.discard),
          child: const Text('Discard'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, UnsavedChoice.save),
          child: const Text('Save'),
        ),
      ],
    ),
  );
}
