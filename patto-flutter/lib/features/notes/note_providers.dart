import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers.dart';
import '../../src/rust/api/index.dart';
import '../../src/rust/api/types.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../inbox/inbox_providers.dart';

/// How the note list is ordered.
enum NoteSort { recent, linked, title }

final noteSortProvider = valueProvider<NoteSort>(NoteSort.recent);
final noteSearchProvider = valueProvider<String>('');

final noteListProvider = FutureProvider<List<NoteMeta>>((ref) async {
  final workspace = await ref.watch(workspaceProvider.future);
  ref.watch(notesRevisionProvider);
  final query = ref.watch(noteSearchProvider);
  final sort = ref.watch(noteSortProvider);
  final inboxName = ref.watch(inboxNoteNameProvider);

  if (workspace == null || !workspace.exists) return const [];

  final found = query.trim().isEmpty
      ? await rust.listNotes(root: workspace.root)
      : await rust.searchNotes(root: workspace.root, query: query, limit: 200);
  // The inbox has its own sheet; listing it too would show it twice.
  final notes = found.where((n) => n.name != inboxName).toList();

  if (sort == NoteSort.title) {
    final sorted = [...notes]..sort((a, b) => a.name.compareTo(b.name));
    return sorted;
  }
  if (sort == NoteSort.linked) {
    final counts = <String, int>{};
    try {
      for (final c in await rust.linkCounts(root: workspace.root)) {
        counts[c.name] = c.backlinks;
      }
    } catch (_) {
      // The index may not be built yet; fall back to the filesystem order.
      return notes;
    }
    final sorted = [...notes]
      ..sort((a, b) {
        final byCount = (counts[b.name] ?? 0).compareTo(counts[a.name] ?? 0);
        return byCount != 0 ? byCount : a.name.compareTo(b.name);
      });
    return sorted;
  }
  // `search` already ranks by match, and `list` by modification time.
  return notes;
});

/// Notes whose name or contents contain the query, for the search screen.
final textSearchProvider = FutureProvider.autoDispose
    .family<List<TextSearchHit>, String>((ref, query) async {
      final workspace = await ref.watch(workspaceProvider.future);
      ref.watch(notesRevisionProvider);
      if (workspace == null || !workspace.exists || query.trim().isEmpty) {
        return const [];
      }
      return rust.searchText(
        root: workspace.root,
        query: query,
        maxNotes: 100,
        maxLinesPerNote: 5,
      );
    });

final linkCountsProvider = FutureProvider<Map<String, int>>((ref) {
  return indexedQuery(
    ref,
    empty: const {},
    query: (root) async => {
      for (final c in await rust.linkCounts(root: root)) c.name: c.backlinks,
    },
  );
});

final renderedNoteProvider = FutureProvider.autoDispose
    .family<RenderedNote, String>((ref, relPath) async {
      final workspace = await ref.watch(workspaceProvider.future);
      ref.watch(notesRevisionProvider);
      if (workspace == null) {
        throw StateError('no workspace is active');
      }

      // Keep recently viewed notes parsed so going back is instant.
      final link = ref.keepAlive();
      final timer = Timer(const Duration(minutes: 5), link.close);
      ref.onDispose(timer.cancel);

      final content = await rust.readNote(
        root: workspace.root,
        relPath: relPath,
      );
      return rust.renderNote(content: content);
    });

final backlinksProvider = FutureProvider.autoDispose
    .family<List<BackLink>, String>((ref, relPath) {
      return indexedQuery(
        ref,
        empty: const [],
        query: (root) => rust.backlinks(root: root, relPath: relPath),
      );
    });

final twoHopProvider = FutureProvider.autoDispose.family<List<TwoHop>, String>((
  ref,
  relPath,
) {
  return indexedQuery(
    ref,
    empty: const [],
    query: (root) => rust.twoHopLinks(root: root, relPath: relPath),
  );
});
