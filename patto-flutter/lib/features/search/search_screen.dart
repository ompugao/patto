import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../src/rust/api/types.dart';
import '../notes/note_providers.dart';
import '../notes/note_view_screen.dart';
import 'highlight.dart';

/// Search the text of every note in the workspace.
class SearchScreen extends ConsumerStatefulWidget {
  const SearchScreen({super.key});

  static Future<void> open(BuildContext context) {
    return Navigator.of(context)
        .push(MaterialPageRoute<void>(builder: (_) => const SearchScreen()));
  }

  @override
  ConsumerState<SearchScreen> createState() => _SearchScreenState();
}

class _SearchScreenState extends ConsumerState<SearchScreen> {
  final _controller = TextEditingController();
  Timer? _debounce;
  String _query = '';

  @override
  void dispose() {
    _debounce?.cancel();
    _controller.dispose();
    super.dispose();
  }

  void _onChanged(String value) {
    // Every keystroke reads the whole workspace, so wait for a pause.
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 250), () {
      if (mounted) setState(() => _query = value.trim());
    });
  }

  void _clear() {
    _debounce?.cancel();
    _controller.clear();
    setState(() => _query = '');
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: TextField(
          controller: _controller,
          autofocus: true,
          onChanged: _onChanged,
          onSubmitted: (v) => setState(() => _query = v.trim()),
          textInputAction: TextInputAction.search,
          decoration: const InputDecoration(
            hintText: 'Search text in notes',
            border: InputBorder.none,
          ),
        ),
        actions: [
          ListenableBuilder(
            listenable: _controller,
            builder: (context, _) => _controller.text.isEmpty
                ? const SizedBox.shrink()
                : IconButton(
                    icon: const Icon(Icons.clear),
                    tooltip: 'Clear',
                    onPressed: _clear,
                  ),
          ),
        ],
      ),
      body: _query.isEmpty
          ? const _Message('Type to search every note.')
          : _results(),
    );
  }

  Widget _results() {
    final hits = ref.watch(textSearchProvider(_query));

    return hits.when(
      // Keep the results on screen while a note edit refreshes them.
      skipLoadingOnReload: true,
      loading: () => const Center(child: CircularProgressIndicator()),
      error: (e, _) => _Message('Search failed.\n\n$e'),
      data: (list) {
        if (list.isEmpty) return _Message('No notes contain "$_query".');
        return ListView.builder(
          itemCount: list.length,
          itemBuilder: (context, i) => _HitCard(hit: list[i], query: _query),
        );
      },
    );
  }
}

/// One note and its matching lines.
class _HitCard extends StatelessWidget {
  const _HitCard({required this.hit, required this.query});

  final TextSearchHit hit;
  final String query;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final match = TextStyle(
      backgroundColor: colors.tertiaryContainer,
      color: colors.onTertiaryContainer,
    );
    final hidden = hit.totalMatches - hit.matches.length;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        ListTile(
          dense: true,
          title: Text.rich(
            highlightedSpan(
              hit.note.name,
              query,
              style: theme.textTheme.titleMedium,
              highlight: match,
            ),
            overflow: TextOverflow.ellipsis,
          ),
          trailing: hit.totalMatches == 0
              ? null
              : Text(
                  hit.totalMatches == 1
                      ? '1 line'
                      : '${hit.totalMatches} lines',
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: colors.outline,
                  ),
                ),
          onTap: () => NoteViewScreen.open(context, hit.note.relPath),
        ),
        for (final m in hit.matches)
          InkWell(
            onTap: () =>
                NoteViewScreen.open(context, hit.note.relPath, row: m.row),
            child: Padding(
              padding: const EdgeInsets.fromLTRB(16, 4, 16, 4),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  SizedBox(
                    width: 40,
                    child: Text(
                      '${m.row + 1}',
                      textAlign: TextAlign.right,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colors.outline,
                        fontFeatures: const [FontFeature.tabularFigures()],
                      ),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Text.rich(
                      highlightedSpan(
                        m.line,
                        query,
                        style: theme.textTheme.bodyMedium,
                        highlight: match,
                      ),
                      maxLines: 3,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ],
              ),
            ),
          ),
        if (hidden > 0)
          Padding(
            padding: const EdgeInsets.fromLTRB(68, 2, 16, 4),
            child: Text(
              '$hidden more in this note',
              style: theme.textTheme.bodySmall?.copyWith(color: colors.outline),
            ),
          ),
        const Divider(height: 16),
      ],
    );
  }
}

class _Message extends StatelessWidget {
  const _Message(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Text(text, textAlign: TextAlign.center),
      ),
    );
  }
}
