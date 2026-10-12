import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../src/rust/api/events.dart';
import '../src/rust/frb_api.dart' as rust;
import 'quick_note_intents.dart';
import 'settings.dart';
import 'workspace.dart';

final settingsStoreProvider = Provider((_) => SettingsStore());

final quickNoteIntentsProvider = Provider<QuickNoteIntents>((ref) {
  final intents = QuickNoteIntents();
  ref.onDispose(intents.dispose);
  return intents;
});

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

/// Which bottom tab is showing; a shared text returns to the notes tab
/// before the inbox sheet opens over it.
final rootTabProvider = valueProvider<int>(0);

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
      state = IndexState(
        ready: true,
        scanned: state.scanned,
        total: state.total,
      );
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

/// Runs [query] against the active workspace once its index is ready, and
/// again whenever the notes change. [empty] stands in before that, and when
/// the query fails: the index may be mid-rebuild.
Future<T> indexedQuery<T>(
  Ref ref, {
  required T empty,
  required Future<T> Function(String root) query,
}) async {
  final workspace = await ref.watch(workspaceProvider.future);
  ref.watch(notesRevisionProvider);
  if (workspace == null || !ref.watch(indexProvider).ready) return empty;

  try {
    return await query(workspace.root);
  } catch (_) {
    return empty;
  }
}
