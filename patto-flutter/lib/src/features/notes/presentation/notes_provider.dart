import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../rust_bridge/api/git_api.dart' as bridge_git;
import '../../../rust_bridge/api/parser_api.dart' as parser;
import '../../git/presentation/git_provider.dart';
import '../domain/note_info.dart';

/// Notes provider
final notesProvider =
    StateNotifierProvider<NotesNotifier, AsyncValue<List<NoteInfo>>>((ref) {
  final gitState = ref.watch(gitProvider);
  return NotesNotifier(
      gitState.isCloned ? ref.read(gitProvider.notifier).repoDir : null);
});

/// Sort option provider
final sortOptionProvider =
    StateProvider<SortOption>((ref) => SortOption.recent);

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
  final String? _repoDir;
  final Map<String, List<String>> _linkGraph = {};

  NotesNotifier(this._repoDir) : super(const AsyncValue.loading()) {
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
    final repoDir = _repoDir!;
    final notes = <NoteInfo>[];
    final linkCounts = <String, int>{};

    final dir = Directory(repoDir);
    if (!await dir.exists()) {
      return notes;
    }

    final files = bridge_git.listPnFiles(repoPath: repoDir);

    for (final file in files) {
      final modifiedSeconds = file.modified.toInt();
      final modified = DateTime.fromMillisecondsSinceEpoch(
        modifiedSeconds * 1000,
        isUtc: true,
      ).toLocal();

      try {
        final content = bridge_git.readFileContent(
          filePath: '$repoDir/${file.path}',
        );
        final links = parser.getLinks(content: content);
        _linkGraph[file.name] = links.map((l) => l.name).toList();

        for (final link in links) {
          linkCounts[link.name] = (linkCounts[link.name] ?? 0) + 1;
        }
      } catch (_) {
        _linkGraph[file.name] = [];
      }

      notes.add(NoteInfo(
        path: file.path,
        name: file.name,
        modified: modified,
      ));
    }

    // Apply link counts
    return notes.map((note) {
      return note.copyWith(linkCount: linkCounts[note.name] ?? 0);
    }).toList();
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
  final fullPath = '$repoDir/$path';
  try {
    return bridge_git.readFileContent(filePath: fullPath);
  } catch (e) {
    throw StateError('Failed to read file: $path ($e)');
  }
});
