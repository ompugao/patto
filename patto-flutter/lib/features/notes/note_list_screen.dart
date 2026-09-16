import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../../core/providers.dart';
import '../../src/rust/api/types.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../editor/editor_screen.dart';
import '../sync/sync_sheet.dart';
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

    return Scaffold(
      appBar: AppBar(
        title: const Text('Notes'),
        actions: [
          IconButton(
            icon: const Icon(Icons.sync),
            tooltip: 'Sync',
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
      floatingActionButton: FloatingActionButton(
        onPressed: _createNote,
        child: const Icon(Icons.add),
      ),
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
            child: TextField(
              controller: _searchController,
              onChanged: _onQueryChanged,
              textInputAction: TextInputAction.search,
              decoration: InputDecoration(
                hintText: 'Search notes',
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
  const _NoteTile({required this.note, required this.backlinks});

  final NoteMeta note;
  final int backlinks;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final modified = DateTime.fromMillisecondsSinceEpoch(note.modifiedMs);

    return ListTile(
      title: Text(note.name, overflow: TextOverflow.ellipsis),
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
