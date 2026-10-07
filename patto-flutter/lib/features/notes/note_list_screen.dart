import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../../core/providers.dart';
import '../../src/rust/api/conflict.dart';
import '../../src/rust/api/types.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../conflicts/conflict_list_screen.dart';
import '../conflicts/conflict_state.dart';
import '../editor/editor_screen.dart';
import '../quick_note/quick_note_sheet.dart';
import '../search/search_screen.dart';
import '../sync/sync_sheet.dart';
import '../workspaces/workspace_switcher.dart';
import 'note_view_screen.dart';

class NoteListScreen extends ConsumerStatefulWidget {
  const NoteListScreen({super.key});

  @override
  ConsumerState<NoteListScreen> createState() => _NoteListScreenState();
}

class _NoteListScreenState extends ConsumerState<NoteListScreen> {
  final _searchController = TextEditingController();
  Timer? _debounce;

  @override
  void dispose() {
    _debounce?.cancel();
    _searchController.dispose();
    super.dispose();
  }

  void _onQueryChanged(String value) {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 150), () {
      ref.read(noteSearchProvider.notifier).value = value;
    });
  }

  Future<void> _createNote() async {
    final controller = TextEditingController();
    final name = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('New note'),
        content: TextField(
          controller: controller,
          autofocus: true,
          decoration: const InputDecoration(hintText: 'Note name'),
          onSubmitted: (v) => Navigator.pop(context, v),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, controller.text),
            child: const Text('Create'),
          ),
        ],
      ),
    );

    if (name == null || name.trim().isEmpty || !mounted) return;

    final workspace = await ref.read(workspaceProvider.future);
    if (workspace == null) return;
    try {
      final meta = await rust.createNote(
        root: workspace.root,
        name: name.trim(),
        initialContent: '',
      );
      ref.read(notesRevisionProvider.notifier).value++;
      if (!mounted) return;
      await EditorScreen.open(context, meta.relPath);
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Could not create the note: $e')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final notes = ref.watch(noteListProvider);
    final sort = ref.watch(noteSortProvider);
    final index = ref.watch(indexProvider);
    final counts = ref.watch(linkCountsProvider).value ?? const {};
    final pending = ref.watch(pendingConflictProvider).value;
    final conflicted = ref.watch(conflictedPathsProvider);

    return Scaffold(
      appBar: AppBar(
        // The title names the workspace and opens the switcher: with several
        // repositories it is the fastest way to tell them apart and move
        // between them.
        title: InkWell(
          onTap: () => WorkspaceSwitcher.show(context),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Flexible(
                child: Text(
                  ref.watch(workspaceProvider).value?.config.name ?? 'Notes',
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const Icon(Icons.arrow_drop_down),
            ],
          ),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.manage_search),
            tooltip: 'Search text',
            onPressed: () => SearchScreen.open(context),
          ),
          IconButton(
            icon: Badge(
              isLabelVisible: pending != null,
              backgroundColor: Colors.amber.shade800,
              child: const Icon(Icons.sync),
            ),
            tooltip: pending == null ? 'Sync' : 'Sync (waiting to merge)',
            onPressed: () => SyncSheet.show(context),
          ),
        ],
        bottom: index.building
            ? PreferredSize(
                preferredSize: const Size.fromHeight(2),
                child: LinearProgressIndicator(
                  minHeight: 2,
                  value: index.fraction,
                ),
              )
            : null,
      ),
      floatingActionButton: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          FloatingActionButton.small(
            heroTag: 'quick-note',
            tooltip: 'Quick note',
            onPressed: () => QuickNoteSheet.show(context),
            child: const Icon(Icons.bolt),
          ),
          const SizedBox(height: 12),
          FloatingActionButton(
            heroTag: 'new-note',
            tooltip: 'New note',
            onPressed: _createNote,
            child: const Icon(Icons.add),
          ),
        ],
      ),
      body: Column(
        children: [
          if (pending != null) ConflictBanner(pending: pending),
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
            child: TextField(
              controller: _searchController,
              onChanged: _onQueryChanged,
              textInputAction: TextInputAction.search,
              decoration: InputDecoration(
                hintText: 'Filter by title',
                prefixIcon: const Icon(Icons.search),
                isDense: true,
                border: const OutlineInputBorder(),
                suffixIcon: _searchController.text.isEmpty
                    ? null
                    : IconButton(
                        icon: const Icon(Icons.clear),
                        onPressed: () {
                          _searchController.clear();
                          _onQueryChanged('');
                          setState(() {});
                        },
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
                onSelectionChanged: (s) =>
                    ref.read(noteSortProvider.notifier).value = s.first,
              ),
            ),
          ),
          const SizedBox(height: 8),
          Expanded(
            child: RefreshIndicator(
              onRefresh: () async => SyncSheet.show(context),
              child: notes.when(
                loading: () => const Center(child: CircularProgressIndicator()),
                error: (e, _) => _Message('Could not list notes.\n\n$e'),
                data: (list) => list.isEmpty
                    ? const _Message('No notes yet. Use + to create one.')
                    : ListView.builder(
                        key: const PageStorageKey('note-list'),
                        itemCount: list.length,
                        itemBuilder: (context, i) => _NoteTile(
                          note: list[i],
                          backlinks: counts[list[i].name] ?? 0,
                          conflicted: conflicted.contains(list[i].relPath),
                        ),
                      ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _NoteTile extends StatelessWidget {
  const _NoteTile({
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

class _Message extends StatelessWidget {
  const _Message(this.text);

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

/// A standing reminder that a sync is paused, until the merge is done.
class ConflictBanner extends StatelessWidget {
  const ConflictBanner({super.key, required this.pending});

  final PendingConflict pending;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final n = pending.files.length;
    return Material(
      color: Colors.amber.withValues(alpha: 0.18),
      child: InkWell(
        onTap: () => ConflictListScreen.open(context),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
          child: Row(
            children: [
              Icon(Icons.warning_amber_rounded, color: Colors.amber.shade800),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  n == 0
                      ? 'A sync is paused. Sync again to finish.'
                      : '$n ${n == 1 ? 'note is' : 'notes are'} waiting to '
                            'be merged',
                  style: theme.textTheme.bodyMedium,
                ),
              ),
              const Icon(Icons.chevron_right),
            ],
          ),
        ),
      ),
    );
  }
}
