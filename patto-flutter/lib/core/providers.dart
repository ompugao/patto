import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../src/rust/api/events.dart';
import '../src/rust/api/git.dart';
import '../src/rust/api/index.dart';
import '../src/rust/api/tasks.dart';
import '../src/rust/api/types.dart';
import '../src/rust/frb_api.dart' as rust;
import 'settings.dart';
import 'workspace.dart';

final settingsStoreProvider = Provider((_) => SettingsStore());

/// Stand-in for the removed StateProvider: one mutable value with a setter.
class ValueNotifierOf<T> extends Notifier<T> {
  ValueNotifierOf(this._initial);

  final T _initial;

  @override
  T build() => _initial;

  set value(T next) => state = next;
  T get value => state;
}

NotifierProvider<ValueNotifierOf<T>, T> valueProvider<T>(T initial) =>
    NotifierProvider<ValueNotifierOf<T>, T>(() => ValueNotifierOf<T>(initial));

class SettingsNotifier extends AsyncNotifier<Settings> {
  @override
  Future<Settings> build() => ref.read(settingsStoreProvider).load();

  Future<void> save(Settings next) async {
    state = AsyncData(next);
    await ref.read(settingsStoreProvider).save(next);
  }

  /// Add a workspace, or replace the one with the same id.
  Future<void> saveWorkspace(Workspace workspace) async {
    final current = state.value ?? const Settings();
    await save(current.withWorkspace(workspace));
  }

  Future<void> removeWorkspace(String id) async {
    final current = state.value ?? const Settings();
    await save(current.withoutWorkspace(id));
    await ref.read(settingsStoreProvider).forgetToken(id);
  }

  Future<void> setActiveWorkspace(String id) async {
    final current = state.value ?? const Settings();
    if (current.activeWorkspaceId == id) return;
    await save(current.copyWith(activeWorkspaceId: id));
  }
}

final settingsProvider = AsyncNotifierProvider<SettingsNotifier, Settings>(
  SettingsNotifier.new,
);

/// Where workspace folders live. Resolved once; the path never changes.
final workspaceBaseDirProvider = FutureProvider<String>(
  (ref) => WorkspaceStorage.baseDir(),
);

/// The workspace the app is currently showing, or null before the first one is
/// configured. Everything derived from notes watches this, so switching
/// workspace re-reads the lot.
final workspaceProvider = FutureProvider<ActiveWorkspace?>((ref) async {
  final settings = await ref.watch(settingsProvider.future);
  final workspace = settings.active;
  if (workspace == null) return null;

  final baseDir = await ref.watch(workspaceBaseDirProvider.future);
  return ActiveWorkspace(
    config: workspace,
    root: WorkspaceStorage.rootFor(baseDir, workspace),
  );
});

/// Every configured workspace, for the switcher and the settings list.
final workspacesProvider = Provider<List<Workspace>>((ref) {
  return ref.watch(settingsProvider).value?.workspaces ?? const [];
});

/// Multiplier for note text, from the appearance setting.
final fontScaleProvider = Provider<double>((ref) {
  return ref.watch(settingsProvider).value?.fontScale ?? 1.0;
});

/// Bumped whenever notes change on disk, to invalidate everything derived.
final notesRevisionProvider = valueProvider<int>(0);

/// How the note list is ordered.
enum NoteSort { recent, linked, title }

final noteSortProvider = valueProvider<NoteSort>(NoteSort.recent);
final noteSearchProvider = valueProvider<String>('');

class IndexState {
  const IndexState({
    this.building = false,
    this.scanned = 0,
    this.total = 0,
    this.ready = false,
    this.error,
  });

  final bool building;
  final int scanned;
  final int total;
  final bool ready;
  final String? error;

  double? get fraction => total == 0 ? null : scanned / total;
}

class IndexNotifier extends Notifier<IndexState> {
  /// Roots indexed during this run. The Rust index is kept per root for the
  /// life of the process, so switching back to a workspace needs no rescan.
  final _built = <String>{};

  @override
  IndexState build() => const IndexState();

  /// Index a workspace unless it has already been indexed in this run.
  Future<void> ensureBuilt(String root) async {
    if (_built.contains(root)) {
      state = const IndexState(ready: true);
      return;
    }
    await rebuild(root);
  }

  /// Scans the whole notes directory. Safe to call again; a second call while
  /// one is running is ignored.
  Future<void> rebuild(String root) async {
    if (state.building) return;
    state = const IndexState(building: true);

    try {
      await for (final event in rust.indexBuild(root: root)) {
        switch (event) {
          case IndexEvent_Progress(:final progress):
            state = IndexState(
              building: true,
              scanned: progress.scanned,
              total: progress.total,
            );
          case IndexEvent_Done():
            _built.add(root);
            state = IndexState(
              ready: true,
              scanned: state.scanned,
              total: state.total,
            );
            ref.read(notesRevisionProvider.notifier).state++;
          case IndexEvent_Failed(:final failure):
            state = IndexState(error: failure.message);
        }
      }
    } catch (e) {
      state = IndexState(error: e.toString());
    }
  }

  /// Re-reads only the notes that changed, after a sync.
  Future<void> refresh(String root) async {
    try {
      await rust.indexRefresh(root: root);
      _built.add(root);
      state = IndexState(ready: true, scanned: state.scanned, total: state.total);
      ref.read(notesRevisionProvider.notifier).state++;
    } catch (e) {
      state = IndexState(error: e.toString(), ready: state.ready);
    }
  }

  /// Forget a root, so it is scanned afresh next time. Used when a workspace is
  /// re-cloned or deleted.
  void forget(String root) {
    _built.remove(root);
    state = const IndexState();
  }
}

final indexProvider = NotifierProvider<IndexNotifier, IndexState>(
  IndexNotifier.new,
);

final noteListProvider = FutureProvider<List<NoteMeta>>((ref) async {
  final workspace = await ref.watch(workspaceProvider.future);
  ref.watch(notesRevisionProvider);
  final query = ref.watch(noteSearchProvider);
  final sort = ref.watch(noteSortProvider);

  if (workspace == null || !workspace.exists) return const [];

  final notes = query.trim().isEmpty
      ? await rust.listNotes(root: workspace.root)
      : await rust.searchNotes(root: workspace.root, query: query, limit: 200);

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

final linkCountsProvider = FutureProvider<Map<String, int>>((ref) async {
  final workspace = await ref.watch(workspaceProvider.future);
  ref.watch(notesRevisionProvider);
  if (workspace == null || !ref.watch(indexProvider).ready) return const {};

  try {
    return {
      for (final c in await rust.linkCounts(root: workspace.root))
        c.name: c.backlinks,
    };
  } catch (_) {
    return const {};
  }
});

final renderedNoteProvider =
    FutureProvider.autoDispose.family<RenderedNote, String>((ref, relPath) async {
  final workspace = await ref.watch(workspaceProvider.future);
  ref.watch(notesRevisionProvider);
  if (workspace == null) {
    throw StateError('no workspace is active');
  }

  // Keep recently viewed notes parsed so going back is instant.
  final link = ref.keepAlive();
  final timer = Timer(const Duration(minutes: 5), link.close);
  ref.onDispose(timer.cancel);

  final content = await rust.readNote(root: workspace.root, relPath: relPath);
  return rust.renderNote(content: content);
});

final backlinksProvider =
    FutureProvider.autoDispose.family<List<BackLink>, String>((ref, relPath) async {
  final workspace = await ref.watch(workspaceProvider.future);
  ref.watch(notesRevisionProvider);
  if (workspace == null || !ref.watch(indexProvider).ready) return const [];

  try {
    return await rust.backlinks(root: workspace.root, relPath: relPath);
  } catch (_) {
    return const [];
  }
});

final twoHopProvider =
    FutureProvider.autoDispose.family<List<TwoHop>, String>((ref, relPath) async {
  final workspace = await ref.watch(workspaceProvider.future);
  ref.watch(notesRevisionProvider);
  if (workspace == null || !ref.watch(indexProvider).ready) return const [];

  try {
    return await rust.twoHopLinks(root: workspace.root, relPath: relPath);
  } catch (_) {
    return const [];
  }
});

final pendingTasksProvider = FutureProvider<List<TaskItem>>((ref) async {
  final workspace = await ref.watch(workspaceProvider.future);
  ref.watch(notesRevisionProvider);
  if (workspace == null || !ref.watch(indexProvider).ready) return const [];

  try {
    return await rust.pendingTasks(root: workspace.root);
  } catch (_) {
    return const [];
  }
});

/// Timeframe for the completed-tasks review.
final reviewTimeframeProvider = valueProvider<String>('today');
final reviewRangeProvider = valueProvider<DateTimeRange?>(null);

final completedTasksProvider = FutureProvider<List<TaskItem>>((ref) async {
  final workspace = await ref.watch(workspaceProvider.future);
  ref.watch(notesRevisionProvider);
  if (workspace == null || !ref.watch(indexProvider).ready) return const [];

  final timeframe = ref.watch(reviewTimeframeProvider);
  final range = ref.watch(reviewRangeProvider);
  String? asDate(DateTime? d) =>
      d == null ? null : '${d.year.toString().padLeft(4, '0')}-'
          '${d.month.toString().padLeft(2, '0')}-'
          '${d.day.toString().padLeft(2, '0')}';

  try {
    return await rust.completedTasks(
      root: workspace.root,
      timeframe: timeframe,
      from: asDate(range?.start),
      to: asDate(range?.end),
    );
  } catch (_) {
    return const [];
  }
});

final gitStatusProvider = FutureProvider.autoDispose<GitStatus?>((ref) async {
  final workspace = await ref.watch(workspaceProvider.future);
  ref.watch(notesRevisionProvider);
  if (workspace == null || !workspace.isCloned) return null;

  try {
    return await rust.gitStatus(root: workspace.root);
  } catch (_) {
    return null;
  }
});
