import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers.dart';
import '../../core/settings.dart';
import '../../src/rust/api/types.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../notes/note_view_screen.dart';
import '../settings/settings_screen.dart';
import 'quick_note_entry.dart';

/// Capture a few lines into the daily or inbox note without naming a note.
class QuickNoteSheet extends ConsumerStatefulWidget {
  const QuickNoteSheet({super.key, this.initialText = ''});

  final String initialText;

  /// Show the sheet and, once something was saved, offer to open the note.
  static Future<void> show(
    BuildContext context, {
    String initialText = '',
  }) async {
    final messenger = ScaffoldMessenger.of(context);
    final navigator = Navigator.of(context);
    // Not dismissible by tapping outside or dragging: a dismissal while the
    // append is in flight would lose the "Added" confirmation. Cancel and the
    // back gesture (while not saving) close it.
    final meta = await showModalBottomSheet<NoteMeta>(
      context: context,
      isScrollControlled: true,
      isDismissible: false,
      enableDrag: false,
      useSafeArea: true,
      builder: (_) => QuickNoteSheet(initialText: initialText),
    );
    if (meta == null) return;
    messenger.showSnackBar(
      SnackBar(
        content: Text('Added to ${meta.name}'),
        action: SnackBarAction(
          label: 'Open',
          onPressed: () => navigator.push(
            MaterialPageRoute<void>(
              builder: (_) => NoteViewScreen(relPath: meta.relPath),
            ),
          ),
        ),
      ),
    );
  }

  @override
  ConsumerState<QuickNoteSheet> createState() => _QuickNoteSheetState();
}

class _QuickNoteSheetState extends ConsumerState<QuickNoteSheet> {
  late final _controller = TextEditingController(text: widget.initialText);
  bool _saving = false;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final settings = ref.read(settingsProvider).value ?? const Settings();
    final now = DateTime.now();
    final entry = formatQuickNoteEntry(
      _controller.text,
      timePrefix: settings.quickNoteTimePrefix,
      now: now,
    );
    if (entry.isEmpty) return;

    // Everything from `ref` is read before awaiting: the element may be gone
    // by the time the append returns.
    final revision = ref.read(notesRevisionProvider.notifier);
    final workspaceFuture = ref.read(workspaceProvider.future);
    setState(() => _saving = true);
    try {
      final workspace = await workspaceFuture;
      if (workspace == null) {
        throw StateError('no workspace');
      }
      final meta = await rust.appendToNote(
        root: workspace.root,
        name: quickNoteTargetName(settings, now),
        text: entry,
      );
      revision.value++;
      if (!mounted) return;
      Navigator.of(context).pop(meta);
    } catch (e) {
      if (!mounted) return;
      setState(() => _saving = false);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Could not save the quick note: $e')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final settings = ref.watch(settingsProvider).value ?? const Settings();
    final target = quickNoteTargetName(settings, DateTime.now());
    final hasText = _controller.text.trim().isNotEmpty;

    return PopScope(
      canPop: !_saving,
      child: Padding(
        padding: EdgeInsets.only(
          left: 16,
          right: 16,
          top: 12,
          bottom: MediaQuery.viewInsetsOf(context).bottom + 16,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Text('Quick note', style: theme.textTheme.titleMedium),
                const Spacer(),
                Text(
                  'to $target',
                  style: theme.textTheme.bodySmall,
                  overflow: TextOverflow.ellipsis,
                ),
                IconButton(
                  icon: const Icon(Icons.tune, size: 20),
                  tooltip: 'Quick note settings',
                  onPressed: () => SettingsScreen.open(context),
                ),
              ],
            ),
            TextField(
              controller: _controller,
              autofocus: true,
              minLines: 3,
              maxLines: 8,
              textCapitalization: TextCapitalization.sentences,
              decoration: const InputDecoration(
                hintText: 'What is on your mind?',
                border: OutlineInputBorder(),
              ),
              onChanged: (_) => setState(() {}),
            ),
            const SizedBox(height: 12),
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                TextButton(
                  onPressed: _saving ? null : () => Navigator.of(context).pop(),
                  child: const Text('Cancel'),
                ),
                const SizedBox(width: 8),
                FilledButton.icon(
                  onPressed: hasText && !_saving ? _save : null,
                  icon: _saving
                      ? const SizedBox.square(
                          dimension: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.bolt),
                  label: const Text('Save'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
