import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../src/rust/api/conflict.dart';
import '../../src/rust/api/merge.dart';
import '../../src/rust/frb_api.dart' as rust;
import 'conflict_state.dart';
import 'widgets/conflict_card.dart';
import 'widgets/conflict_dialogs.dart';
import 'widgets/conflict_strip.dart';
import 'widgets/merge_lines.dart';
import 'widgets/whole_note_choice.dart';

/// One clashing note, shown as it will read after the merge.
///
/// Lines only one side changed are already in, tagged with whose they are.
/// Each place both sides changed is a card with a choice, so reading the note
/// top to bottom is also resolving it.
class ConflictNoteScreen extends ConsumerStatefulWidget {
  const ConflictNoteScreen({super.key, required this.relPath});

  final String relPath;

  static Future<void> open(BuildContext context, String relPath) {
    return Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => ConflictNoteScreen(relPath: relPath),
      ),
    );
  }

  @override
  ConsumerState<ConflictNoteScreen> createState() => _ConflictNoteScreenState();
}

class _ConflictNoteScreenState extends ConsumerState<ConflictNoteScreen> {
  final _keys = <int, GlobalKey>{};
  final _expanded = <int>{};

  /// The conflict last jumped to, as an index into [conflictIndices].
  int _cursor = -1;

  GlobalKey _keyFor(int region) => _keys.putIfAbsent(region, GlobalKey.new);

  void _update(NoteDraft draft) {
    ref
        .read(conflictDraftsProvider.notifier)
        .put(widget.relPath, draft.copyWith(done: false));
  }

  void _choose(NoteDraft draft, int region, Choice? choice) {
    final choices = {...draft.choices};
    if (choice == null) {
      choices.remove(region);
    } else {
      choices[region] = choice;
    }
    _update(draft.copyWith(choices: choices));
  }

  void _toggleUndone(NoteDraft draft, int region) {
    final undone = {...draft.undone};
    if (!undone.remove(region)) undone.add(region);
    _update(draft.copyWith(undone: undone));
  }

  void _jump(ConflictDetail detail, NoteDraft draft, int step) {
    final indices = conflictIndices(detail);
    if (indices.isEmpty) return;
    final next = nextConflictCursor(indices, draft.choices, _cursor, step);
    _jumpTo(indices[next]);
    setState(() => _cursor = next);
  }

  void _jumpTo(int region) {
    final context = _keys[region]?.currentContext;
    if (context == null) return;
    Scrollable.ensureVisible(
      context,
      alignment: 0.2,
      duration: const Duration(milliseconds: 250),
      curve: Curves.easeOut,
    );
  }

  void _useForRest(ConflictDetail detail, NoteDraft draft, Pick pick) {
    final choices = {...draft.choices};
    for (final i in conflictIndices(detail)) {
      choices.putIfAbsent(i, () => Choice(pick));
    }
    _update(draft.copyWith(choices: choices));
  }

  Future<void> _edit(
    NoteDraft draft,
    int region,
    MergeRegion_Conflict conflict,
  ) async {
    final current = draft.choices[region];
    final start = current == null ? conflict.ours : linesFor(conflict, current);
    final text = await promptConflictLines(context, start);
    if (text == null) return;
    _choose(
      draft,
      region,
      Choice(Pick.custom, text.isEmpty ? const [] : text.split('\n')),
    );
  }

  void _finish(NoteDraft draft) {
    ref
        .read(conflictDraftsProvider.notifier)
        .put(widget.relPath, draft.copyWith(done: true));
    Navigator.pop(context);
  }

  @override
  Widget build(BuildContext context) {
    final detail = ref.watch(conflictDetailProvider(widget.relPath));
    ref.watch(conflictDraftsProvider);
    final name = rust.relPathToNoteName(relPath: widget.relPath);

    return detail.when(
      loading: () => Scaffold(
        appBar: AppBar(title: Text(name)),
        body: const Center(child: CircularProgressIndicator()),
      ),
      error: (e, _) => Scaffold(
        appBar: AppBar(title: Text(name)),
        body: Padding(
          padding: const EdgeInsets.all(32),
          child: Text(
            'This note cannot be shown as a conflict any more. '
            'Sync again to see where things stand.\n\n$e',
            textAlign: TextAlign.center,
          ),
        ),
      ),
      data: (detail) => _build(context, name, detail),
    );
  }

  Widget _build(BuildContext context, String name, ConflictDetail detail) {
    final draft = ref.read(conflictDraftsProvider.notifier).draftFor(detail);
    final total = conflictIndices(detail).length;
    final left = unresolvedCount(detail, draft);
    final whole = isWholeNote(detail.kind);

    return Scaffold(
      appBar: _ConflictAppBar(
        name: name,
        left: left,
        total: total,
        whole: whole,
        onStep: (step) => _jump(detail, draft, step),
        onShowOurs: () =>
            showVersionSheet(context, 'Your version', detail.ours),
        onShowTheirs: () =>
            showVersionSheet(context, 'Remote version', detail.theirs),
        onUseForRest: (pick) => _useForRest(detail, draft, pick),
      ),
      body: whole
          ? WholeNoteChoice(
              detail: detail,
              choice: draft.choices[-1],
              onChoose: (c) => _choose(draft, -1, c),
            )
          : Stack(
              children: [
                ListView(
                  padding: const EdgeInsets.fromLTRB(12, 8, 20, 24),
                  children: [
                    for (var i = 0; i < detail.merged.regions.length; i++)
                      KeyedSubtree(
                        key: _keyFor(i),
                        child: _region(detail, draft, i),
                      ),
                  ],
                ),
                Positioned(
                  top: 8,
                  bottom: 8,
                  right: 2,
                  width: 10,
                  child: ConflictStrip(
                    regionCount: detail.merged.regions.length,
                    conflicts: conflictIndices(detail),
                    resolved: draft.choices.keys.toSet(),
                    onTap: _jumpTo,
                  ),
                ),
              ],
            ),
      bottomNavigationBar: SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 12),
          child: FilledButton.icon(
            icon: Icon(left == 0 ? Icons.check : Icons.arrow_downward),
            label: Text(
              left == 0 ? 'Done with this note' : 'Next conflict ($left left)',
            ),
            onPressed: () {
              if (left > 0) {
                _jump(detail, draft, 1);
                return;
              }
              _finish(draft);
            },
          ),
        ),
      ),
    );
  }

  Widget _region(ConflictDetail detail, NoteDraft draft, int i) {
    final region = detail.merged.regions[i];
    return switch (region) {
      MergeRegion_Unchanged(:final lines) => UnchangedLines(
        lines: lines,
        expanded: _expanded.contains(i),
        onExpand: () => setState(() => _expanded.add(i)),
      ),
      MergeRegion_Ours(:final base, :final lines) => OneSidedLines(
        side: MergeSide.ours,
        base: base,
        lines: lines,
        undone: draft.undone.contains(i),
        onToggle: () => _toggleUndone(draft, i),
      ),
      MergeRegion_Theirs(:final base, :final lines) => OneSidedLines(
        side: MergeSide.theirs,
        base: base,
        lines: lines,
        undone: draft.undone.contains(i),
        onToggle: () => _toggleUndone(draft, i),
      ),
      MergeRegion_Same(:final base, :final lines) => OneSidedLines(
        side: MergeSide.both,
        base: base,
        lines: lines,
        undone: false,
      ),
      MergeRegion_Conflict() => ConflictCard(
        region: region,
        choice: draft.choices[i],
        onChoose: (c) => _choose(draft, i, c),
        onEdit: () => _edit(draft, i, region),
      ),
    };
  }
}

/// The note's name over how much is left, with the ways to move through the
/// conflicts and to settle the rest at once.
class _ConflictAppBar extends StatelessWidget implements PreferredSizeWidget {
  const _ConflictAppBar({
    required this.name,
    required this.left,
    required this.total,
    required this.whole,
    required this.onStep,
    required this.onShowOurs,
    required this.onShowTheirs,
    required this.onUseForRest,
  });

  final String name;
  final int left;
  final int total;
  final bool whole;
  final ValueChanged<int> onStep;
  final VoidCallback onShowOurs;
  final VoidCallback onShowTheirs;
  final ValueChanged<Pick> onUseForRest;

  @override
  Size get preferredSize => const Size.fromHeight(kToolbarHeight);

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return AppBar(
      title: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(name, overflow: TextOverflow.ellipsis),
          Text(
            left == 0 ? 'Every conflict has a choice' : '$left of $total left',
            style: theme.textTheme.bodySmall,
          ),
        ],
      ),
      actions: [
        if (!whole) ...[
          IconButton(
            icon: const Icon(Icons.keyboard_arrow_up),
            tooltip: 'Previous conflict',
            onPressed: () => onStep(-1),
          ),
          IconButton(
            icon: const Icon(Icons.keyboard_arrow_down),
            tooltip: 'Next conflict',
            onPressed: () => onStep(1),
          ),
        ],
        PopupMenuButton<String>(
          onSelected: (value) => switch (value) {
            'mine' => onShowOurs(),
            'remote' => onShowTheirs(),
            'rest-mine' => onUseForRest(Pick.ours),
            'rest-remote' => onUseForRest(Pick.theirs),
            _ => null,
          },
          itemBuilder: (context) => [
            const PopupMenuItem(value: 'mine', child: Text('Your version')),
            const PopupMenuItem(value: 'remote', child: Text('Remote version')),
            if (!whole && left > 0) ...[
              const PopupMenuDivider(),
              const PopupMenuItem(
                value: 'rest-mine',
                child: Text('Use yours for the rest'),
              ),
              const PopupMenuItem(
                value: 'rest-remote',
                child: Text('Use remote for the rest'),
              ),
            ],
          ],
        ),
      ],
    );
  }
}
