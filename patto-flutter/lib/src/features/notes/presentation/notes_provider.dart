import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../domain/note_info.dart';
import '../../git/presentation/git_provider.dart';

/// Notes provider
final notesProvider =
    StateNotifierProvider<NotesNotifier, AsyncValue<List<NoteInfo>>>((ref) {
  final gitState = ref.watch(gitProvider);
  return NotesNotifier(ref, gitState.isCloned ? ref.read(gitProvider.notifier).repoDir : null);
});

/// Sort option provider
final sortOptionProvider = StateProvider<SortOption>((ref) => SortOption.recent);

/// Search query provider
final searchQueryProvider = StateProvider<String>((ref) => '');

/// Filtered and sorted notes provider
final filteredNotesProvider = Provider<AsyncValue<List<NoteInfo>>>((ref) {
  final notesAsync = ref.watch(notesProvider);
  final sortOption = ref.watch(sortOptionProvider);
  final searchQuery = ref.watch(searchQueryProvider).toLowerCase();

  return notesAsync.whenData((notes) {
    // Filter by search query
    var filtered = notes;
    if (searchQuery.isNotEmpty) {
      filtered = notes
          .where((n) => n.name.toLowerCase().contains(searchQuery))
          .toList();
    }

    // Sort
    switch (sortOption) {
      case SortOption.recent:
        filtered.sort((a, b) => b.modified.compareTo(a.modified));
      case SortOption.linked:
        filtered.sort((a, b) => b.linkCount.compareTo(a.linkCount));
      case SortOption.title:
        filtered.sort((a, b) => a.name.compareTo(b.name));
    }

    return filtered;
  });
});

/// Notes state notifier
class NotesNotifier extends StateNotifier<AsyncValue<List<NoteInfo>>> {
  final Ref _ref;
  final String? _repoDir;
  final Map<String, List<String>> _linkGraph = {};

  NotesNotifier(this._ref, this._repoDir) : super(const AsyncValue.loading()) {
    if (_repoDir != null) {
      refresh();
    } else {
      state = const AsyncValue.data([]);
    }
  }

  /// Refresh the notes list
  Future<void> refresh() async {
    if (_repoDir == null) {
      state = const AsyncValue.data([]);
      return;
    }

    state = const AsyncValue.loading();

    try {
      final notes = await _loadNotes();
      state = AsyncValue.data(notes);
    } catch (e, st) {
      state = AsyncValue.error(e, st);
    }
  }

  /// Load notes from the repository
  Future<List<NoteInfo>> _loadNotes() async {
    final notes = <NoteInfo>[];
    final linkCounts = <String, int>{};

    final dir = Directory(_repoDir!);
    if (!await dir.exists()) {
      return notes;
    }

    // Collect all .pn files
    await for (final entity in dir.list(recursive: true)) {
      if (entity is File && entity.path.endsWith('.pn')) {
        // Skip .git directory
        if (entity.path.contains('/.git/')) continue;

        final stat = await entity.stat();
        final relativePath = entity.path.replaceFirst('$_repoDir/', '');
        final name = relativePath.replaceAll('.pn', '').split('/').last;

        // Parse file to extract links
        try {
          final content = await entity.readAsString();
          final links = _extractLinks(content);
          _linkGraph[name] = links;

          // Count incoming links
          for (final link in links) {
            linkCounts[link] = (linkCounts[link] ?? 0) + 1;
          }
        } catch (e) {
          _linkGraph[name] = [];
        }

        notes.add(NoteInfo(
          path: relativePath,
          name: name,
          modified: stat.modified,
        ));
      }
    }

    // Apply link counts
    return notes.map((note) {
      return note.copyWith(linkCount: linkCounts[note.name] ?? 0);
    }).toList();
  }

  /// Extract wiki links from content (simple regex-based extraction)
  List<String> _extractLinks(String content) {
    final links = <String>[];
    // Match wiki links: [PageName] or [PageName#anchor]
    final regex = RegExp(r'\[([^\[\]@`$\/\s][^\[\]#]*?)(?:#[^\[\]]+)?\]');

    for (final match in regex.allMatches(content)) {
      final name = match.group(1)?.trim();
      if (name != null &&
          !name.contains('://') &&
          !name.startsWith('@') &&
          !name.startsWith('`') &&
          !name.startsWith('\$')) {
        links.add(name);
      }
    }

    return links;
  }

  /// Get note by name
  NoteInfo? getNoteByName(String name) {
    return state.valueOrNull?.firstWhere(
      (n) => n.name == name,
      orElse: () => throw StateError('Note not found: $name'),
    );
  }
}

/// Note content provider
final noteContentProvider =
    FutureProvider.family<String, String>((ref, path) async {
  final gitState = ref.watch(gitProvider);
  if (!gitState.isCloned) {
    throw StateError('Repository not cloned');
  }

  final repoDir = ref.read(gitProvider.notifier).repoDir;
  final file = File('$repoDir/$path');

  if (!await file.exists()) {
    throw StateError('File not found: $path');
  }

  return await file.readAsString();
});
