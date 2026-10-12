import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../core/providers.dart';
import '../../src/rust/api/conflict.dart';
import '../../src/rust/api/merge.dart';
import '../../src/rust/frb_api.dart' as rust;

/// The sync that stopped at a conflict, or null when nothing is waiting.
final pendingConflictProvider = FutureProvider<PendingConflict?>((ref) async {
  final workspace = await ref.watch(workspaceProvider.future);
  ref.watch(notesRevisionProvider);
  if (workspace == null || !workspace.isCloned) return null;

  try {
    return await rust.pendingConflict(root: workspace.root);
  } catch (_) {
    return null;
  }
});

/// Paths of the notes that clash, for marking them in lists.
final conflictedPathsProvider = Provider<Set<String>>((ref) {
  final pending = ref.watch(pendingConflictProvider).value;
  return {...?pending?.files.map((f) => f.path)};
});

final conflictDetailProvider = FutureProvider.autoDispose
    .family<ConflictDetail, String>((ref, relPath) async {
      final workspace = await ref.watch(workspaceProvider.future);
      ref.watch(notesRevisionProvider);
      if (workspace == null) throw StateError('no workspace is active');
      return rust.conflictDetail(root: workspace.root, relPath: relPath);
    });

/// What the user picked for one conflict.
enum Pick { ours, theirs, suggested, both, custom }

class Choice {
  const Choice(this.pick, [this.custom]);

  final Pick pick;

  /// The lines typed by hand, for [Pick.custom].
  final List<String>? custom;

  Map<String, Object?> toJson() => {'pick': pick.name, 'custom': custom};

  static Choice fromJson(Map<String, Object?> json) => Choice(
    Pick.values.byName(json['pick']! as String),
    (json['custom'] as List?)?.cast<String>(),
  );
}

/// The user's progress on one note. Tied to the versions it was made against,
/// so a note that changed underneath starts over.
class NoteDraft {
  const NoteDraft({
    required this.oursId,
    required this.theirsId,
    this.choices = const {},
    this.undone = const {},
    this.done = false,
  });

  final String? oursId;
  final String? theirsId;

  /// Keyed by region index. For a note deleted on one side the whole note is
  /// region -1: [Pick.ours] keeps our side, [Pick.theirs] the remote's.
  final Map<int, Choice> choices;

  /// One-sided changes the user took back, by region index.
  final Set<int> undone;

  /// The user has reviewed the whole note and confirmed it.
  final bool done;

  bool matches(ConflictDetail detail) =>
      oursId == detail.oursId && theirsId == detail.theirsId;

  NoteDraft copyWith({
    Map<int, Choice>? choices,
    Set<int>? undone,
    bool? done,
  }) => NoteDraft(
    oursId: oursId,
    theirsId: theirsId,
    choices: choices ?? this.choices,
    undone: undone ?? this.undone,
    done: done ?? this.done,
  );

  Map<String, Object?> toJson() => {
    'oursId': oursId,
    'theirsId': theirsId,
    'choices': {for (final e in choices.entries) '${e.key}': e.value.toJson()},
    'undone': undone.toList(),
    'done': done,
  };

  static NoteDraft fromJson(Map<String, Object?> json) => NoteDraft(
    oursId: json['oursId'] as String?,
    theirsId: json['theirsId'] as String?,
    choices: {
      for (final e in (json['choices']! as Map).entries)
        int.parse(e.key as String): Choice.fromJson(
          (e.value as Map).cast<String, Object?>(),
        ),
    },
    undone: {...(json['undone']! as List).cast<int>()},
    done: json['done']! as bool,
  );
}

bool isWholeNote(ConflictKind kind) =>
    kind == ConflictKind.deletedByUs || kind == ConflictKind.deletedByThem;

/// A draft for a note seen for the first time: every conflict that has a
/// suggestion starts on it, so a plain case needs only a look and a tap.
NoteDraft freshDraft(ConflictDetail detail) {
  final choices = <int, Choice>{};
  if (!isWholeNote(detail.kind)) {
    final regions = detail.merged.regions;
    for (var i = 0; i < regions.length; i++) {
      final region = regions[i];
      if (region is MergeRegion_Conflict && region.suggestion != null) {
        choices[i] = const Choice(Pick.suggested);
      }
    }
  }
  return NoteDraft(
    oursId: detail.oursId,
    theirsId: detail.theirsId,
    choices: choices,
  );
}

/// Indices of the regions that need a choice.
List<int> conflictIndices(ConflictDetail detail) {
  if (isWholeNote(detail.kind)) return const [-1];
  final regions = detail.merged.regions;
  return [
    for (var i = 0; i < regions.length; i++)
      if (regions[i] is MergeRegion_Conflict) i,
  ];
}

int unresolvedCount(ConflictDetail detail, NoteDraft draft) =>
    conflictIndices(detail).where((i) => !draft.choices.containsKey(i)).length;

/// Where to go from [cursor], a position in [indices], when stepping by
/// [step]: the nearest conflict in that direction still without a choice, or
/// simply the next one when every conflict has one. Wraps around the ends.
int nextConflictCursor(
  List<int> indices,
  Map<int, Choice> choices,
  int cursor,
  int step,
) {
  int wrap(int n) => n % indices.length;
  var next = wrap(cursor + step);
  for (var n = 1; n <= indices.length; n++) {
    final candidate = wrap(cursor + step * n);
    if (!choices.containsKey(indices[candidate])) {
      next = candidate;
      break;
    }
  }
  return next;
}

/// The lines a conflict stands for under [choice].
List<String> linesFor(MergeRegion_Conflict region, Choice choice) =>
    switch (choice.pick) {
      Pick.ours => region.ours,
      Pick.theirs => region.theirs,
      Pick.suggested => region.suggestion?.lines ?? region.ours,
      Pick.both => [...region.ours, ...region.theirs],
      Pick.custom => choice.custom ?? const [],
    };

/// The lines a region contributes to the merged note, or null for a conflict
/// that has no choice yet.
List<String>? regionResult(ConflictDetail detail, NoteDraft draft, int i) {
  final region = detail.merged.regions[i];
  return switch (region) {
    MergeRegion_Unchanged(:final lines) => lines,
    MergeRegion_Ours(:final base, :final lines) ||
    MergeRegion_Theirs(:final base, :final lines) ||
    MergeRegion_Same(
      :final base,
      :final lines,
    ) => draft.undone.contains(i) ? base : lines,
    MergeRegion_Conflict() => switch (draft.choices[i]) {
      null => null,
      final choice => linesFor(region, choice),
    },
  };
}

/// The merged text of a note, or null for "delete it".
String? mergedContent(ConflictDetail detail, NoteDraft draft) {
  if (isWholeNote(detail.kind)) {
    final keepOurs = draft.choices[-1]?.pick == Pick.ours;
    return keepOurs ? detail.ours : detail.theirs;
  }

  final lines = <String>[];
  for (var i = 0; i < detail.merged.regions.length; i++) {
    lines.addAll(regionResult(detail, draft, i) ?? const []);
  }
  var text = lines.join('\n');
  if (detail.merged.trailingNewline && lines.isNotEmpty) text += '\n';
  return text;
}

/// Drafts for every clashing note of one paused sync, kept across restarts.
class ConflictDrafts extends Notifier<Map<String, NoteDraft>> {
  String? _key;

  /// Mirrors [state], which cannot be read while rebuilding.
  Map<String, NoteDraft> _drafts = const {};

  @override
  Map<String, NoteDraft> build() {
    final pending = ref.watch(pendingConflictProvider).value;
    final workspace = ref.watch(workspaceProvider).value;
    // Not tied to the remote commit: when the remote moves on, a note whose
    // sides did not change keeps its draft (see [NoteDraft.matches]).
    final key = pending == null || workspace == null
        ? null
        : 'conflict-draft:${workspace.config.id}';

    // Rebuilt on every change to the notes; keep what is already loaded.
    if (key == _key) return _drafts;
    _key = key;
    _drafts = const {};
    if (key != null) _load(key);
    return _drafts;
  }

  Future<void> _load(String key) async {
    final prefs = await SharedPreferences.getInstance();
    final raw = prefs.getString(key);
    if (raw == null || key != _key) return;
    try {
      final json = (jsonDecode(raw) as Map).cast<String, Object?>();
      _set({
        for (final e in json.entries)
          e.key: NoteDraft.fromJson((e.value! as Map).cast<String, Object?>()),
      });
    } catch (_) {
      await prefs.remove(key);
    }
  }

  void _set(Map<String, NoteDraft> drafts) {
    _drafts = drafts;
    state = drafts;
  }

  Future<void> _save() async {
    final key = _key;
    if (key == null) return;
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(
      key,
      jsonEncode({for (final e in _drafts.entries) e.key: e.value.toJson()}),
    );
  }

  /// The draft for [detail], starting a fresh one if there is none or the note
  /// changed since.
  NoteDraft draftFor(ConflictDetail detail) {
    final existing = _drafts[detail.path];
    if (existing != null && existing.matches(detail)) return existing;
    return freshDraft(detail);
  }

  void put(String path, NoteDraft draft) {
    _set({..._drafts, path: draft});
    _save();
  }

  Future<void> clear() async {
    final key = _key;
    _set(const {});
    if (key == null) return;
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove(key);
  }
}

final conflictDraftsProvider =
    NotifierProvider<ConflictDrafts, Map<String, NoteDraft>>(
      ConflictDrafts.new,
    );
