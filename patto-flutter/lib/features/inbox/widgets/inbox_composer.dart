import 'package:flutter/material.dart';

/// The text field and send button at the bottom of the inbox sheet.
class InboxComposer extends StatelessWidget {
  const InboxComposer({
    super.key,
    required this.controller,
    required this.focusNode,
    required this.canSend,
    required this.sending,
    required this.onChanged,
    required this.onSend,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final bool canSend;
  final bool sending;
  final VoidCallback onChanged;
  final VoidCallback onSend;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      color: theme.colorScheme.surface,
      elevation: 3,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 8, 8, 8),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            Expanded(
              child: TextField(
                controller: controller,
                focusNode: focusNode,
                autofocus: true,
                minLines: 1,
                maxLines: 5,
                textInputAction: TextInputAction.newline,
                keyboardType: TextInputType.multiline,
                textCapitalization: TextCapitalization.sentences,
                decoration: const InputDecoration(
                  hintText: 'Write a quick note',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
                onChanged: (_) => onChanged(),
              ),
            ),
            const SizedBox(width: 4),
            sending
                ? const Padding(
                    padding: EdgeInsets.all(12),
                    child: SizedBox.square(
                      dimension: 24,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                  )
                : IconButton(
                    icon: const Icon(Icons.send),
                    tooltip: 'Post',
                    onPressed: canSend ? onSend : null,
                  ),
          ],
        ),
      ),
    );
  }
}
