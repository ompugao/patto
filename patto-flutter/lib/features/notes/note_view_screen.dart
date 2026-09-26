import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:super_sliver_list/super_sliver_list.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../core/providers.dart';
import '../../src/rust/api/types.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../editor/editor_screen.dart';
import '../search/highlight.dart';
import '../sync/sync_sheet.dart';
import '../tasks/task_status_sheet.dart';
import 'widgets/block_widget.dart';
import 'widgets/spans_text.dart';

class NoteViewScreen extends ConsumerStatefulWidget {
  const NoteViewScreen({
    super.key,
    required this.relPath,
    this.initialRow,
    this.initialAnchor,
  });

  final String relPath;
  final int? initialRow;
  final String? initialAnchor;

  static Future<void> open(
    BuildContext context,
    String relPath, {
    int? row,
    String? anchor,
  }) {
    return Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => NoteViewScreen(
          relPath: relPath,
          initialRow: row,
          initialAnchor: anchor,
        ),
      ),
    );
  }

  @override
  ConsumerState<NoteViewScreen> createState() => _NoteViewScreenState();
}

class _NoteViewScreenState extends ConsumerState<NoteViewScreen> {
  final _listController = ListController();
  final _scrollController = ScrollController();
  int? _flashed;
  bool _jumped = false;

  // Find in note. Matching runs over the raw source lines, which are then
  // mapped onto the blocks that render them.
  bool _finding = false;
  final _findController = TextEditingController();
  Timer? _findDebounce;
  String _findTerm = '';
  List<int> _findHits = const [];
  Set<int> _findHitSet = const {};
  int _findCurrent = 0;
  List<String> _sourceLines = const [];
  RenderedNote? _sourceFor;

  String get _title => rust.relPathToNoteName(relPath: widget.relPath);

  @override
  void dispose() {
    _findDebounce?.cancel();
    _findController.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  /// Index of the block that renders source line [row]: the last one starting
  /// at or before it, since code blocks and tables span several lines. Blocks
  /// are in source order.
  int _blockForRow(List<Block> blocks, int row) {
    var low = 0;
    var high = blocks.length;
    while (low < high) {
      final mid = (low + high) >> 1;
      if (blocks[mid].row <= row) {
        low = mid + 1;
      } else {
        high = mid;
      }
    }
    return (low - 1).clamp(0, blocks.length - 1);
  }

  void _jumpTo(int index) {
    if (index < 0) return;
    _listController.jumpToItem(
      index: index,
      scrollController: _scrollController,
      alignment: 0.1,
    );
    setState(() => _flashed = index);
    Future.delayed(const Duration(milliseconds: 900), () {
      if (mounted) setState(() => _flashed = null);
    });
  }

  void _jumpToAnchor(RenderedNote note, String anchor) {
    final target = note.anchors.where((a) => a.name == anchor).firstOrNull;
    if (target == null) {
      _toast('No anchor "$anchor" in this note');
      return;
    }
    _jumpTo(target.blockIndex);
  }

  void _applyInitialJump(RenderedNote note) {
    if (_jumped) return;
    _jumped = true;

    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      if (widget.initialAnchor != null) {
        _jumpToAnchor(note, widget.initialAnchor!);
      } else if (widget.initialRow != null) {
        _jumpTo(_blockForRow(note.blocks, widget.initialRow!));
      }
    });
  }

  void _toggleFind() {
    _findDebounce?.cancel();
    setState(() {
      _finding = !_finding;
      if (!_finding) {
        _findController.clear();
        _findTerm = '';
        _findHits = const [];
        _findHitSet = const {};
        _findCurrent = 0;
      }
    });
  }

  void _onFindChanged(String value) {
    _findDebounce?.cancel();
    _findDebounce = Timer(const Duration(milliseconds: 150), () {
      _findTerm = value.trim();
      _runFind(jump: true);
    });
  }

  /// Recompute which blocks contain the find term. Re-run when the note
  /// changes underneath, without moving the view.
  Future<void> _runFind({required bool jump}) async {
    final note = ref.read(renderedNoteProvider(widget.relPath)).value;
    final term = _findTerm;
    if (note == null || term.isEmpty) {
      if (mounted) {
        setState(() {
          _findHits = const [];
          _findHitSet = const {};
          _findCurrent = 0;
        });
      }
      return;
    }

    if (!identical(_sourceFor, note)) {
      final workspace = await ref.read(workspaceProvider.future);
      if (workspace == null) return;
      try {
        final content =
            await rust.readNote(root: workspace.root, relPath: widget.relPath);
        _sourceLines = content.split('\n');
        _sourceFor = note;
      } catch (e) {
        _toast('Could not search this note: $e');
        return;
      }
    }
    if (!mounted || term != _findTerm || note.blocks.isEmpty) return;

    final hits = <int>[];
    for (var row = 0; row < _sourceLines.length; row++) {
      if (!containsIgnoringCase(_sourceLines[row], term)) continue;
      final index = _blockForRow(note.blocks, row);
      // Rows ascend, so repeats of a multi-line block are adjacent.
      if (hits.isEmpty || hits.last != index) hits.add(index);
    }

    setState(() {
      _findHits = hits;
      _findHitSet = hits.toSet();
      _findCurrent = _findCurrent.clamp(0, hits.isEmpty ? 0 : hits.length - 1);
      if (jump) _findCurrent = 0;
    });
    if (jump && hits.isNotEmpty) _jumpTo(hits.first);
  }

  void _stepFind(int delta) {
    if (_findHits.isEmpty) return;
    setState(() {
      _findCurrent = (_findCurrent + delta) % _findHits.length;
    });
    _jumpTo(_findHits[_findCurrent]);
  }

  PreferredSizeWidget _findBar(BuildContext context) {
    final theme = Theme.of(context);
    final count = _findHits.isEmpty
        ? (_findTerm.isEmpty ? '' : 'No matches')
        : '${_findCurrent + 1} / ${_findHits.length}';

    return PreferredSize(
      preferredSize: const Size.fromHeight(52),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 0, 4, 8),
        child: Row(
          children: [
            Expanded(
              child: TextField(
                controller: _findController,
                autofocus: true,
                onChanged: _onFindChanged,
                onSubmitted: (_) => _stepFind(1),
                textInputAction: TextInputAction.search,
                decoration: const InputDecoration(
                  hintText: 'Find in note',
                  isDense: true,
                  prefixIcon: Icon(Icons.search),
                  border: OutlineInputBorder(),
                ),
              ),
            ),
            const SizedBox(width: 8),
            Text(count, style: theme.textTheme.labelMedium),
            IconButton(
              icon: const Icon(Icons.keyboard_arrow_up),
              tooltip: 'Previous match',
              onPressed: _findHits.isEmpty ? null : () => _stepFind(-1),
            ),
            IconButton(
              icon: const Icon(Icons.keyboard_arrow_down),
              tooltip: 'Next match',
              onPressed: _findHits.isEmpty ? null : () => _stepFind(1),
            ),
          ],
        ),
      ),
    );
  }

  void _toast(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(SnackBar(content: Text(message)));
  }

  Future<void> _openWikiLink(String name, String? anchor) async {
    final workspace = await ref.read(workspaceProvider.future);
    if (workspace == null) return;
    final target = rust.resolveWikiLink(root: workspace.root, name: name);

    if (!mounted) return;
    if (target == null) {
      ScaffoldMessenger.of(context)
        ..hideCurrentSnackBar()
        ..showSnackBar(
          SnackBar(
            content: Text('"$name" does not exist yet'),
            action: SnackBarAction(
              label: 'Create',
              onPressed: () => _createAndOpen(name),
            ),
          ),
        );
      return;
    }
    await NoteViewScreen.open(context, target, anchor: anchor);
  }

  Future<void> _createAndOpen(String name) async {
    final workspace = await ref.read(workspaceProvider.future);
    if (workspace == null) return;
    try {
      final meta = await rust.createNote(
        root: workspace.root,
        name: name,
        initialContent: '',
      );
      ref.read(notesRevisionProvider.notifier).value++;
      if (!mounted) return;
      await EditorScreen.open(context, meta.relPath);
    } catch (e) {
      _toast('Could not create the note: $e');
    }
  }

  Future<void> _openUrl(String url) async {
    final uri = Uri.tryParse(url);
    if (uri == null || !await launchUrl(uri, mode: LaunchMode.externalApplication)) {
      _toast('Could not open $url');
    }
  }

  Future<void> _changeTaskStatus(Block block) async {
    final task = block.task;
    if (task == null) return;

    final next = await TaskStatusSheet.show(context, task.status);
    if (next == null || !mounted) return;

    final workspace = await ref.read(workspaceProvider.future);
    if (workspace == null) return;
    try {
      await rust.setTaskStatus(
        root: workspace.root,
        relPath: widget.relPath,
        row: block.row,
        status: next,
      );
      ref.read(notesRevisionProvider.notifier).value++;
    } catch (e) {
      _toast('Could not update the task: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    final note = ref.watch(renderedNoteProvider(widget.relPath));
    final workspace = ref.watch(workspaceProvider).value;
    final textScale = ref.watch(fontScaleProvider);

    return Scaffold(
      // The only text input is the find bar at the top, so the keyboard must
      // never relayout the note.
      resizeToAvoidBottomInset: false,
      appBar: AppBar(
        title: Text(_title, overflow: TextOverflow.ellipsis),
        bottom: _finding ? _findBar(context) : null,
        actions: [
          IconButton(
            icon: Icon(_finding ? Icons.search_off : Icons.search),
            tooltip: _finding ? 'Close find' : 'Find in note',
            onPressed: _toggleFind,
          ),
          IconButton(
            icon: const Icon(Icons.sync),
            tooltip: 'Sync',
            onPressed: () => SyncSheet.show(context),
          ),
          IconButton(
            icon: const Icon(Icons.edit_outlined),
            tooltip: 'Edit',
            onPressed: () => EditorScreen.open(context, widget.relPath),
          ),
        ],
      ),
      body: note.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Text('Could not open this note.\n\n$e'),
          ),
        ),
        data: (data) {
          _applyInitialJump(data);
          if (_finding && _findTerm.isNotEmpty && !identical(_sourceFor, data)) {
            // The note was edited or synced; the old matches point at stale
            // blocks.
            WidgetsBinding.instance.addPostFrameCallback(
              (_) => _runFind(jump: false),
            );
          }

          final actions = SpanActions(
            onWikiLink: _openWikiLink,
            onUrl: _openUrl,
            onAnchor: (anchor) => _jumpToAnchor(data, anchor),
          );

          return SuperListView.builder(
            listController: _listController,
            controller: _scrollController,
            extentEstimation: (_, _) => 30 * textScale,
            padding: const EdgeInsets.only(top: 8, bottom: 32),
            itemCount: data.blocks.length + 1,
            itemBuilder: (context, i) {
              if (i == data.blocks.length) {
                return _NoteFooter(
                  relPath: widget.relPath,
                  errors: data.errors,
                );
              }
              return RepaintBoundary(
                child: BlockWidget(
                  key: ValueKey(i),
                  block: data.blocks[i],
                  actions: actions,
                  root: workspace?.root,
                  textScale: textScale,
                  highlighted: i == _flashed,
                  matched: _findHitSet.contains(i),
                  searchTerm: _finding ? _findTerm : null,
                  onTaskTap: _changeTaskStatus,
                ),
              );
            },
          );
        },
      ),
    );
  }
}

class _NoteFooter extends ConsumerWidget {
  const _NoteFooter({required this.relPath, required this.errors});

  static const _backlinkLimit = 50;

  final String relPath;
  final List<ParseIssue> errors;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final backlinks = ref.watch(backlinksProvider(relPath));
    final twoHop = ref.watch(twoHopProvider(relPath));
    final indexing = ref.watch(indexProvider).building;

    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 24, 16, 16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (errors.isNotEmpty) ...[
            _Heading('Syntax (${errors.length})'),
            for (final issue in errors.take(10))
              Text(
                'line ${issue.row + 1}: ${issue.message}',
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.error,
                ),
              ),
            const SizedBox(height: 16),
          ],
          const Divider(),
          _Heading('Backlinks'),
          if (indexing)
            Text('Indexing…', style: theme.textTheme.bodySmall)
          else
            backlinks.when(
              loading: () => const LinearProgressIndicator(minHeight: 2),
              error: (_, _) => const SizedBox.shrink(),
              data: (links) => links.isEmpty
                  ? Text('None', style: theme.textTheme.bodySmall)
                  : Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        // A note linked from thousands of lines would otherwise
                        // make the footer longer than the note itself.
                        for (final link in links.take(_backlinkLimit))
                          ListTile(
                            dense: true,
                            contentPadding: EdgeInsets.zero,
                            title: Text(link.sourceName),
                            subtitle: Text(
                              link.context,
                              maxLines: 2,
                              overflow: TextOverflow.ellipsis,
                            ),
                            onTap: () => NoteViewScreen.open(
                              context,
                              link.sourceRelPath,
                              row: link.row,
                            ),
                          ),
                        if (links.length > _backlinkLimit)
                          Padding(
                            padding: const EdgeInsets.only(top: 4),
                            child: Text(
                              'and ${links.length - _backlinkLimit} more',
                              style: theme.textTheme.bodySmall,
                            ),
                          ),
                      ],
                    ),
            ),
          const SizedBox(height: 16),
          _Heading('2-hop links'),
          twoHop.when(
            loading: () => const SizedBox.shrink(),
            error: (_, _) => const SizedBox.shrink(),
            data: (hops) => hops.isEmpty
                ? Text('None', style: theme.textTheme.bodySmall)
                : Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      for (final hop in hops)
                        Padding(
                          padding: const EdgeInsets.only(bottom: 8),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                'via ${hop.viaName}',
                                style: theme.textTheme.labelMedium,
                              ),
                              Wrap(
                                spacing: 6,
                                children: [
                                  for (final name in hop.names)
                                    ActionChip(
                                      label: Text(name),
                                      onPressed: () => _openByName(context, ref, name),
                                    ),
                                ],
                              ),
                            ],
                          ),
                        ),
                    ],
                  ),
          ),
        ],
      ),
    );
  }

  Future<void> _openByName(BuildContext context, WidgetRef ref, String name) async {
    final workspace = await ref.read(workspaceProvider.future);
    if (workspace == null) return;
    final target = rust.resolveWikiLink(root: workspace.root, name: name);
    if (!context.mounted || target == null) return;
    await NoteViewScreen.open(context, target);
  }
}

class _Heading extends StatelessWidget {
  const _Heading(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Text(text, style: Theme.of(context).textTheme.titleSmall),
    );
  }
}
