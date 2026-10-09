import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../conflicts/conflict_state.dart';
import '../editor/editor_screen.dart';
import '../inbox/inbox_sheet.dart';
import '../search/search_screen.dart';
import '../sync/sync_sheet.dart';
import '../workspaces/workspace_switcher.dart';
import 'widgets/conflict_banner.dart';
import 'widgets/note_list_filter.dart';
import 'widgets/note_tile.dart';

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

  void _clearQuery() {
    _searchController.clear();
    _onQueryChanged('');
    setState(() {});
  }

  Future<void> _createNote() async {
    final name = await _askNoteName(context);
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
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('Could not create the note: $e')));
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
        title: _WorkspaceTitle(
          name: ref.watch(workspaceProvider).value?.config.name ?? 'Notes',
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.note_add_outlined),
            tooltip: 'New note',
            onPressed: _createNote,
          ),
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
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () => InboxSheet.show(context),
        icon: const Icon(Icons.edit_outlined),
        label: const Text('Inbox'),
      ),
      body: Column(
        children: [
          if (pending != null) ConflictBanner(pending: pending),
          NoteListFilter(
            controller: _searchController,
            onQueryChanged: _onQueryChanged,
            onClear: _clearQuery,
            sort: sort,
            onSortChanged: (s) => ref.read(noteSortProvider.notifier).value = s,
          ),
          const SizedBox(height: 8),
          Expanded(
            child: RefreshIndicator(
              onRefresh: () async => SyncSheet.show(context),
              child: notes.when(
                loading: () => const Center(child: CircularProgressIndicator()),
                error: (e, _) => NoteListMessage('Could not list notes.\n\n$e'),
                data: (list) => list.isEmpty
                    ? const NoteListMessage(
                        'No notes yet. Use the new-note button above to create one.',
                      )
                    : ListView.builder(
                        key: const PageStorageKey('note-list'),
                        itemCount: list.length,
                        itemBuilder: (context, i) => NoteTile(
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

/// The title names the workspace and opens the switcher: with several
/// repositories it is the fastest way to tell them apart and move between
/// them.
class _WorkspaceTitle extends StatelessWidget {
  const _WorkspaceTitle({required this.name});

  final String name;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: () => WorkspaceSwitcher.show(context),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Flexible(child: Text(name, overflow: TextOverflow.ellipsis)),
          const Icon(Icons.arrow_drop_down),
        ],
      ),
    );
  }
}

Future<String?> _askNoteName(BuildContext context) {
  final controller = TextEditingController();
  return showDialog<String>(
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
}
