import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../src/rust/api/conflict.dart';
import '../../src/rust/api/merge.dart';
import '../../src/rust/frb_api.dart' as rust;
import 'conflict_state.dart';

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

    int wrap(int n) => n % indices.length;
    // Prefer the nearest conflict still waiting for a choice.
    var next = wrap(_cursor + step);
    for (var n = 1; n <= indices.length; n++) {
      final candidate = wrap(_cursor + step * n);
      if (!draft.choices.containsKey(indices[candidate])) {
        next = candidate;
        break;
      }
    }
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

  void _showVersion(String title, String? text) {
    showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      showDragHandle: true,
      builder: (context) => DraggableScrollableSheet(
        expand: false,
        initialChildSize: 0.8,
        builder: (context, controller) => ListView(
          controller: controller,
          padding: const EdgeInsets.fromLTRB(20, 0, 20, 24),
          children: [
            Text(title, style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 12),
            text == null
                ? const Text('Deleted on this side.')
                : SelectableText(_display(text)),
          ],
        ),
      ),
    );
  }

  Future<void> _edit(
    NoteDraft draft,
    int region,
    MergeRegion_Conflict conflict,
  ) async {
    final current = draft.choices[region];
    final start = current == null ? conflict.ours : linesFor(conflict, current);
    final controller = TextEditingController(text: start.join('\n'));
    final text = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Edit these lines'),
        content: TextField(
          controller: controller,
          autofocus: true,
          maxLines: null,
          minLines: 3,
          style: const TextStyle(fontFamily: 'monospace'),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, controller.text),
            child: const Text('Use'),
          ),
        ],
      ),
    );
    if (text == null) return;
    _choose(
      draft,
      region,
      Choice(Pick.custom, text.isEmpty ? const [] : text.split('\n')),
    );
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
    final theme = Theme.of(context);
    final draft = ref.read(conflictDraftsProvider.notifier).draftFor(detail);
    final total = conflictIndices(detail).length;
    final left = unresolvedCount(detail, draft);
    final whole = isWholeNote(detail.kind);

    return Scaffold(
      appBar: AppBar(
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(name, overflow: TextOverflow.ellipsis),
            Text(
              left == 0
                  ? 'Every conflict has a choice'
                  : '$left of $total left',
              style: theme.textTheme.bodySmall,
            ),
          ],
        ),
        actions: [
          if (!whole) ...[
            IconButton(
              icon: const Icon(Icons.keyboard_arrow_up),
              tooltip: 'Previous conflict',
              onPressed: () => _jump(detail, draft, -1),
            ),
            IconButton(
              icon: const Icon(Icons.keyboard_arrow_down),
              tooltip: 'Next conflict',
              onPressed: () => _jump(detail, draft, 1),
            ),
          ],
          PopupMenuButton<String>(
            onSelected: (value) => switch (value) {
              'mine' => _showVersion('Your version', detail.ours),
              'remote' => _showVersion('Remote version', detail.theirs),
              'rest-mine' => _useForRest(detail, draft, Pick.ours),
              'rest-remote' => _useForRest(detail, draft, Pick.theirs),
              _ => null,
            },
            itemBuilder: (context) => [
              const PopupMenuItem(value: 'mine', child: Text('Your version')),
              const PopupMenuItem(
                value: 'remote',
                child: Text('Remote version'),
              ),
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
      ),
      body: whole
          ? _WholeNoteChoice(
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
                  child: _ConflictStrip(
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
              left == 0
                  ? 'Done with this note'
                  : 'Next conflict ($left left)',
            ),
            onPressed: () {
              if (left > 0) {
                _jump(detail, draft, 1);
                return;
              }
              ref
                  .read(conflictDraftsProvider.notifier)
                  .put(widget.relPath, draft.copyWith(done: true));
              Navigator.pop(context);
            },
          ),
        ),
      ),
    );
  }

  Widget _region(ConflictDetail detail, NoteDraft draft, int i) {
    final region = detail.merged.regions[i];
    return switch (region) {
      MergeRegion_Unchanged(:final lines) => _UnchangedLines(
        lines: lines,
        expanded: _expanded.contains(i),
        onExpand: () => setState(() => _expanded.add(i)),
      ),
      MergeRegion_Ours(:final base, :final lines) => _OneSided(
        side: _Side.ours,
        base: base,
        lines: lines,
        undone: draft.undone.contains(i),
        onToggle: () => _toggleUndone(draft, i),
      ),
      MergeRegion_Theirs(:final base, :final lines) => _OneSided(
        side: _Side.theirs,
        base: base,
        lines: lines,
        undone: draft.undone.contains(i),
        onToggle: () => _toggleUndone(draft, i),
      ),
      MergeRegion_Same(:final base, :final lines) => _OneSided(
        side: _Side.both,
        base: base,
        lines: lines,
        undone: false,
      ),
      MergeRegion_Conflict() => _ConflictCard(
        region: region,
        choice: draft.choices[i],
        onChoose: (c) => _choose(draft, i, c),
        onEdit: () => _edit(draft, i, region),
      ),
    };
  }
}

/// Tabs as spaces, so indentation reads the same as in the note.
String _display(String line) => line.replaceAll('\t', '    ');

class _Line extends StatelessWidget {
  const _Line(this.text, {this.style, this.span});

  final String text;
  final TextStyle? style;
  final InlineSpan? span;

  @override
  Widget build(BuildContext context) {
    final base = Theme.of(context).textTheme.bodyMedium!.merge(style);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 1),
      child: span != null
          ? Text.rich(span!, style: base)
          : Text(text.isEmpty ? ' ' : _display(text), style: base),
    );
  }
}

class _UnchangedLines extends StatelessWidget {
  const _UnchangedLines({
    required this.lines,
    required this.expanded,
    required this.onExpand,
  });

  final List<String> lines;
  final bool expanded;
  final VoidCallback onExpand;

  /// Lines of context kept on each side of a collapsed run.
  static const _context = 2;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final faded = TextStyle(color: theme.colorScheme.onSurfaceVariant);

    if (expanded || lines.length <= _context * 2 + 1) {
      return Padding(
        padding: const EdgeInsets.only(left: 8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [for (final l in lines) _Line(l, style: faded)],
        ),
      );
    }

    final hidden = lines.length - _context * 2;
    return Padding(
      padding: const EdgeInsets.only(left: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          for (final l in lines.take(_context)) _Line(l, style: faded),
          InkWell(
            onTap: onExpand,
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: Text(
                '⋯ $hidden unchanged lines',
                style: theme.textTheme.labelMedium?.copyWith(
                  color: theme.colorScheme.primary,
                ),
              ),
            ),
          ),
          for (final l in lines.skip(lines.length - _context))
            _Line(l, style: faded),
        ],
      ),
    );
  }
}

enum _Side { ours, theirs, both }

extension on _Side {
  String get label => switch (this) {
    _Side.ours => 'you',
    _Side.theirs => 'remote',
    _Side.both => 'both',
  };

  Color color(ColorScheme scheme) => switch (this) {
    _Side.ours => scheme.primary,
    _Side.theirs => scheme.tertiary,
    _Side.both => scheme.outline,
  };
}

/// Lines only one side changed, already merged in. The tag takes the change
/// back, or puts it in again.
class _OneSided extends StatelessWidget {
  const _OneSided({
    required this.side,
    required this.base,
    required this.lines,
    required this.undone,
    this.onToggle,
  });

  final _Side side;
  final List<String> base;
  final List<String> lines;
  final bool undone;
  final VoidCallback? onToggle;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = side.color(theme.colorScheme);
    final deleted = lines.isEmpty;

    final shown = undone || deleted ? base : lines;
    final struck = !undone && deleted;

    return Container(
      margin: const EdgeInsets.symmetric(vertical: 2),
      padding: const EdgeInsets.only(left: 6),
      decoration: BoxDecoration(
        border: Border(
          left: BorderSide(
            color: undone ? theme.colorScheme.outlineVariant : color,
            width: 3,
          ),
        ),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                for (final l in shown)
                  _Line(
                    l,
                    style: struck
                        ? TextStyle(
                            decoration: TextDecoration.lineThrough,
                            color: theme.colorScheme.onSurfaceVariant,
                          )
                        : null,
                  ),
              ],
            ),
          ),
          InkWell(
            onTap: onToggle,
            borderRadius: BorderRadius.circular(8),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
              child: Text(
                undone
                    ? '${side.label} · undone'
                    : deleted
                    ? '${side.label} deleted'
                    : side.label,
                style: theme.textTheme.labelSmall?.copyWith(
                  color: undone ? theme.colorScheme.outline : color,
                  decoration: undone ? TextDecoration.lineThrough : null,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// [line] with the part that differs from [other] highlighted.
InlineSpan _highlighted(String line, String other, Color highlight) {
  final a = _display(line);
  final b = _display(other);
  var prefix = 0;
  while (prefix < a.length && prefix < b.length && a[prefix] == b[prefix]) {
    prefix++;
  }
  var suffix = 0;
  while (suffix < a.length - prefix &&
      suffix < b.length - prefix &&
      a[a.length - 1 - suffix] == b[b.length - 1 - suffix]) {
    suffix++;
  }
  final changed = a.substring(prefix, a.length - suffix);
  return TextSpan(
    children: [
      TextSpan(text: a.substring(0, prefix)),
      if (changed.isNotEmpty)
        TextSpan(
          text: changed,
          style: TextStyle(backgroundColor: highlight),
        ),
      TextSpan(text: a.substring(a.length - suffix)),
    ],
  );
}

/// One side of an unsettled conflict, with its differences from the other side
/// highlighted when the lines pair up.
class _SideLines extends StatelessWidget {
  const _SideLines({
    required this.side,
    required this.lines,
    required this.other,
  });

  final _Side side;
  final List<String> lines;
  final List<String> other;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = side.color(theme.colorScheme);
    final paired = lines.length == other.length;

    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            side == _Side.ours ? 'You' : 'Remote',
            style: theme.textTheme.labelSmall?.copyWith(color: color),
          ),
          Container(
            padding: const EdgeInsets.only(left: 6),
            decoration: BoxDecoration(
              border: Border(left: BorderSide(color: color, width: 3)),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (lines.isEmpty)
                  Text(
                    '(deleted these lines)',
                    style: theme.textTheme.bodySmall?.copyWith(
                      fontStyle: FontStyle.italic,
                    ),
                  ),
                for (var k = 0; k < lines.length; k++)
                  _Line(
                    lines[k],
                    span: paired
                        ? _highlighted(
                            lines[k],
                            other[k],
                            color.withValues(alpha: 0.25),
                          )
                        : null,
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _ConflictCard extends StatefulWidget {
  const _ConflictCard({
    required this.region,
    required this.choice,
    required this.onChoose,
    required this.onEdit,
  });

  final MergeRegion_Conflict region;
  final Choice? choice;
  final ValueChanged<Choice?> onChoose;
  final VoidCallback onEdit;

  @override
  State<_ConflictCard> createState() => _ConflictCardState();
}

class _ConflictCardState extends State<_ConflictCard> {
  /// Show both sides again after a choice was made.
  bool _compare = false;

  void _pick(Pick pick) {
    widget.onChoose(widget.choice?.pick == pick ? null : Choice(pick));
  }

  void _showBase() {
    final base = widget.region.base;
    showDialog<void>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Before either change'),
        content: SingleChildScrollView(
          child: base.isEmpty
              ? const Text('These lines did not exist yet.')
              : SelectableText(base.map(_display).join('\n')),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }

  String _caption(Choice choice) => switch (choice.pick) {
    Pick.ours => 'Using yours',
    Pick.theirs => 'Using remote',
    Pick.suggested =>
      widget.region.suggestion?.kind == SuggestionKind.both
          ? '✦ Both, yours first'
          : '✦ Combined',
    Pick.both => 'Both, yours first',
    Pick.custom => 'Edited by hand',
  };

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final region = widget.region;
    final choice = widget.choice;
    final suggestion = region.suggestion;
    final settled = choice != null;

    Widget chip(String label, Pick pick) => ChoiceChip(
      label: Text(label),
      selected: choice?.pick == pick,
      onSelected: (_) => _pick(pick),
      visualDensity: VisualDensity.compact,
    );

    return GestureDetector(
      onLongPress: _showBase,
      // Swipe right to keep yours, left to take the remote's.
      onHorizontalDragEnd: (details) {
        final v = details.primaryVelocity ?? 0;
        if (v > 300) widget.onChoose(const Choice(Pick.ours));
        if (v < -300) widget.onChoose(const Choice(Pick.theirs));
      },
      child: Card(
        margin: const EdgeInsets.symmetric(vertical: 6),
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
          side: BorderSide(
            color: settled
                ? theme.colorScheme.outlineVariant
                : Colors.amber.shade700,
            width: settled ? 1 : 2,
          ),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(12, 10, 12, 8),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (!settled) ...[
                Row(
                  children: [
                    Icon(
                      Icons.warning_amber_rounded,
                      size: 16,
                      color: Colors.amber.shade800,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      'Both changed this',
                      style: theme.textTheme.labelMedium,
                    ),
                  ],
                ),
                const SizedBox(height: 6),
              ],
              if (!settled || _compare) ...[
                _SideLines(
                  side: _Side.ours,
                  lines: region.ours,
                  other: region.theirs,
                ),
                _SideLines(
                  side: _Side.theirs,
                  lines: region.theirs,
                  other: region.ours,
                ),
              ],
              if (settled) ...[
                InkWell(
                  onTap: () => setState(() => _compare = !_compare),
                  child: Row(
                    children: [
                      Text(
                        _caption(choice),
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: theme.colorScheme.primary,
                        ),
                      ),
                      const Spacer(),
                      Text(
                        _compare ? 'hide sides' : 'compare',
                        style: theme.textTheme.labelSmall,
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 2),
                for (final l in linesFor(region, choice)) _Line(l),
                if (linesFor(region, choice).isEmpty)
                  Text(
                    '(no lines)',
                    style: theme.textTheme.bodySmall?.copyWith(
                      fontStyle: FontStyle.italic,
                    ),
                  ),
              ],
              const SizedBox(height: 6),
              Wrap(
                spacing: 6,
                runSpacing: 4,
                children: [
                  chip('You', Pick.ours),
                  chip('Remote', Pick.theirs),
                  if (suggestion?.kind == SuggestionKind.combined)
                    chip('✦ Combined', Pick.suggested),
                  if (suggestion?.kind == SuggestionKind.both)
                    chip('✦ Both', Pick.suggested)
                  else
                    chip('Both', Pick.both),
                  ChoiceChip(
                    avatar: const Icon(Icons.edit, size: 16),
                    label: const Text('Edit'),
                    selected: choice?.pick == Pick.custom,
                    onSelected: (_) => widget.onEdit(),
                    visualDensity: VisualDensity.compact,
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Marks where the conflicts are along the note: amber for waiting, grey for
/// settled. Tapping one jumps to it.
class _ConflictStrip extends StatelessWidget {
  const _ConflictStrip({
    required this.regionCount,
    required this.conflicts,
    required this.resolved,
    required this.onTap,
  });

  final int regionCount;
  final List<int> conflicts;
  final Set<int> resolved;
  final ValueChanged<int> onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return LayoutBuilder(
      builder: (context, constraints) {
        final height = constraints.maxHeight;
        return Stack(
          children: [
            for (final i in conflicts)
              Positioned(
                top: regionCount <= 1
                    ? 0
                    : (height - 14) * i / (regionCount - 1),
                left: 0,
                right: 0,
                child: GestureDetector(
                  onTap: () => onTap(i),
                  child: Container(
                    height: 14,
                    decoration: BoxDecoration(
                      color: resolved.contains(i)
                          ? theme.colorScheme.outlineVariant
                          : Colors.amber.shade700,
                      borderRadius: BorderRadius.circular(3),
                    ),
                  ),
                ),
              ),
          ],
        );
      },
    );
  }
}

/// A note one side deleted and the other edited: keep it or not.
class _WholeNoteChoice extends StatelessWidget {
  const _WholeNoteChoice({
    required this.detail,
    required this.choice,
    required this.onChoose,
  });

  final ConflictDetail detail;
  final Choice? choice;
  final ValueChanged<Choice?> onChoose;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final weDeleted = detail.kind == ConflictKind.deletedByUs;
    final kept = weDeleted ? detail.theirs : detail.ours;

    // Pick.ours keeps our side, which for "we deleted" means staying deleted.
    final keepPick = weDeleted ? Pick.theirs : Pick.ours;
    final deletePick = weDeleted ? Pick.ours : Pick.theirs;

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text(
          weDeleted
              ? 'You deleted this note, and the remote edited it.'
              : 'The remote deleted this note, and you edited it.',
          style: theme.textTheme.titleMedium,
        ),
        const SizedBox(height: 16),
        SegmentedButton<Pick>(
          emptySelectionAllowed: true,
          segments: [
            ButtonSegment(
              value: keepPick,
              icon: const Icon(Icons.note_outlined),
              label: Text(weDeleted ? 'Restore it' : 'Keep it'),
            ),
            ButtonSegment(
              value: deletePick,
              icon: const Icon(Icons.delete_outline),
              label: const Text('Delete it'),
            ),
          ],
          selected: {?choice?.pick},
          onSelectionChanged: (s) =>
              onChoose(s.isEmpty ? null : Choice(s.first)),
        ),
        const SizedBox(height: 24),
        Text(
          weDeleted ? 'The remote\'s version' : 'Your version',
          style: theme.textTheme.labelLarge,
        ),
        const SizedBox(height: 8),
        SelectableText(_display(kept ?? '')),
      ],
    );
  }
}
