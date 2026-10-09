import 'package:flutter/material.dart';

import '../conflict_text.dart';

/// One whole side of the note, for reading it in full.
void showVersionSheet(BuildContext context, String title, String? text) {
  showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    showDragHandle: true,
    builder: (context) => DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.8,
      builder: (context, controller) => ListView(
        controller: controller,
        padding: const EdgeInsets.fromLTRB(20, 0, 20, 24),
        children: [
          Text(title, style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 12),
          text == null
              ? const Text('Deleted on this side.')
              : SelectableText(displayLine(text)),
        ],
      ),
    ),
  );
}

/// Lets the user type the lines a conflict should become, starting from
/// [start]. Null when cancelled.
Future<String?> promptConflictLines(BuildContext context, List<String> start) {
  final controller = TextEditingController(text: start.join('\n'));
  return showDialog<String>(
    context: context,
    builder: (context) => AlertDialog(
      title: const Text('Edit these lines'),
      content: TextField(
        controller: controller,
        autofocus: true,
        maxLines: null,
        minLines: 3,
        style: const TextStyle(fontFamily: 'monospace'),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, controller.text),
          child: const Text('Use'),
        ),
      ],
    ),
  );
}

/// The lines as they were before either side changed them.
void showBaseLinesDialog(BuildContext context, List<String> base) {
  showDialog<void>(
    context: context,
    builder: (context) => AlertDialog(
      title: const Text('Before either change'),
      content: SingleChildScrollView(
        child: base.isEmpty
            ? const Text('These lines did not exist yet.')
            : SelectableText(base.map(displayLine).join('\n')),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Close'),
        ),
      ],
    ),
  );
}
