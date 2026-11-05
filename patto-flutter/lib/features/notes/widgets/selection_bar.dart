import 'package:flutter/material.dart';

/// Actions on the viewer's text selection, shown at the bottom of the
/// screen while there is one.
///
/// Flutter's toolbar disappears once both ends of a selection scroll out of
/// the list and does not come back, so these actions cannot live only there.
class SelectionBar extends StatelessWidget {
  const SelectionBar({
    super.key,
    required this.onCopy,
    required this.onCopyMarkdown,
    required this.onCopySource,
    required this.onEdit,
    required this.onClose,
  });

  static const height = 64.0;

  final VoidCallback onCopy;
  final VoidCallback onCopyMarkdown;
  final VoidCallback onCopySource;
  final VoidCallback onEdit;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    return Material(
      elevation: 3,
      color: Theme.of(context).colorScheme.surfaceContainer,
      child: SafeArea(
        top: false,
        child: SizedBox(
          height: height,
          child: Row(
            children: [
              IconButton(
                icon: const Icon(Icons.close),
                tooltip: 'Clear selection',
                onPressed: onClose,
              ),
              Expanded(
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                  children: [
                    TextButton(onPressed: onCopy, child: const Text('Copy')),
                    TextButton(
                      onPressed: onCopyMarkdown,
                      child: const Text('Markdown'),
                    ),
                    TextButton(
                      onPressed: onCopySource,
                      child: const Text('Source'),
                    ),
                    TextButton(onPressed: onEdit, child: const Text('Edit')),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
