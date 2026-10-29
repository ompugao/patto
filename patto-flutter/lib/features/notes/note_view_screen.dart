import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:super_sliver_list/super_sliver_list.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../core/providers.dart';
import '../../src/rust/api/types.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../editor/editor_screen.dart';
import '../sync/sync_sheet.dart';
import '../tasks/task_status_sheet.dart';
import 'block_source.dart';
import 'note_find.dart';
import 'note_plain_text.dart';
import 'note_providers.dart';
import 'open_embed.dart';
import 'selection_range.dart';
import 'widgets/block_widget.dart';
import 'widgets/find_bar.dart';
import 'widgets/joined_selection.dart';
import 'widgets/note_footer.dart';
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
  NoteFind _find = NoteFind();
  List<String> _sourceLines = const [];
  RenderedNote? _sourceFor;

  /// Selected text per block, kept for blocks that scroll out of the list
  /// so a copy still includes them. See [selectedRange].
  final _selectedText = <int, String>{};
  int? _latestSelected;
  RenderedNote? _selectionFor;

  /// Set by the toolbar's Select all. Flutter selects only the blocks the
  /// list has built, so the screen remembers that the whole note was meant.
  bool _allSelected = false;
  bool _selecting = false;

  String get _title => rust.relPathToNoteName(relPath: widget.relPath);

  @override
  void dispose() {
    _findDebounce?.cancel();
    _findController.dispose();
    _scrollController.dispose();
    super.dispose();
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
        _jumpTo(blockIndexForRow(_rowsOf(note), widget.initialRow!));
      }
    });
  }

  List<int> _rowsOf(RenderedNote note) => [for (final b in note.blocks) b.row];

  void _toggleFind() {
    _findDebounce?.cancel();
    setState(() {
      _finding = !_finding;
      if (!_finding) {
        _findController.clear();
        _find = NoteFind();
      }
    });
  }

  void _onFindChanged(String value) {
    _findDebounce?.cancel();
    _findDebounce = Timer(const Duration(milliseconds: 150), () {
      _find = _find.withTerm(value.trim());
      _runFind(jump: true);
    });
  }

  /// Recompute which blocks contain the find term. Re-run when the note
  /// changes underneath, without moving the view.
  Future<void> _runFind({required bool jump}) async {
    final note = ref.read(renderedNoteProvider(widget.relPath)).value;
    final term = _find.term;
    if (note == null || term.isEmpty) {
      if (mounted) setState(() => _find = _find.withHits(const [], jump: true));
      return;
    }

    if (!await _ensureSource(note, 'Could not search this note')) return;
    if (!mounted || term != _find.term || note.blocks.isEmpty) return;

    final hits = findHitBlocks(_sourceLines, _rowsOf(note), term);
    setState(() => _find = _find.withHits(hits, jump: jump));
    if (jump && hits.isNotEmpty) _jumpTo(hits.first);
  }

  /// Loads the source lines [note] was rendered from, unless already loaded.
  Future<bool> _ensureSource(RenderedNote note, String failure) async {
    if (identical(_sourceFor, note)) return true;
    final workspace = await ref.read(workspaceProvider.future);
    if (workspace == null) return false;
    try {
      final content = await rust.readNote(
        root: workspace.root,
        relPath: widget.relPath,
      );
      _sourceLines = content.split('\n');
      _sourceFor = note;
      return true;
    } catch (e) {
      _toast('$failure: $e');
      return false;
    }
  }

  void _stepFind(int delta) {
    if (_find.hits.isEmpty) return;
    setState(() => _find = _find.stepped(delta));
    _jumpTo(_find.hits[_find.current]);
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
    if (uri == null ||
        !await launchUrl(uri, mode: LaunchMode.externalApplication)) {
      _toast('Could not open $url');
    }
  }

  /// Opens the editor on the block at the top of the screen, so editing
  /// picks up where reading was.
  void _editVisible() {
    final blocks = ref.read(renderedNoteProvider(widget.relPath)).value?.blocks;
    final first = _listController.isAttached
        ? _listController.unobstructedVisibleRange?.$1
        : null;
    final row = blocks != null && first != null && first < blocks.length
        ? blocks[first].row
        : null;
    EditorScreen.open(context, widget.relPath, row: row);
  }

  void _onBlockSelection(int index, String? text) {
    // A block re-selected after scrolling back reports the same text; any
    // other change means a handle moved or the selection was cleared.
    if (_allSelected && _selectedText[index] != text) _allSelected = false;
    if (text == null) {
      _selectedText.remove(index);
    } else {
      _selectedText[index] = text;
      _latestSelected = index;
    }
  }

  void _onBlockMounted(int index) {
    if (!_allSelected) _selectedText.remove(index);
  }

  (int, int)? _selectedRange(RenderedNote note) => _allSelected
      ? (0, note.blocks.length - 1)
      : selectedRange(
          _selectedText,
          _latestSelected,
          note.blocks.length,
          (i) => blockHasText(note.blocks[i]),
        );

  /// [texts] is null when the whole note is selected.
  Future<void> _copySelection(
    RenderedNote note,
    Map<int, String>? texts,
    (int, int) range,
  ) async {
    final (first, last) = range;
    final text = texts == null
        ? notePlainText(note.blocks)
        : [
            for (var i = first; i <= last; i++)
              ?texts[i] ?? (note.blocks[i].kind is BlockKind_Blank ? '' : null),
          ].join('\n');
    await Clipboard.setData(ClipboardData(text: text));
  }

  Future<void> _copySource(
    RenderedNote note,
    (int, int) range, {
    required bool markdown,
  }) async {
    if (!await _ensureSource(note, 'Could not copy')) return;
    final (first, last) = range;
    final source = blockSource(_sourceLines, _rowsOf(note), first, last);
    final text = markdown ? await rust.blockMarkdown(source: source) : source;
    await Clipboard.setData(ClipboardData(text: text));
    _toast(markdown ? 'Copied as Markdown' : 'Copied source');
  }

  TextSelectionToolbarAnchors _menuAnchors(SelectableRegionState region) {
    try {
      return region.contextMenuAnchors;
    } on TypeError {
      // contextMenuAnchors null-checks both ends of the selection, but an end
      // that scrolled out of the list has no point; anchor on the other one.
      final box = region.context.findRenderObject()! as RenderBox;
      final lineHeight = 24.0 * ref.read(fontScaleProvider);
      try {
        return TextSelectionToolbarAnchors.fromSelection(
          renderBox: box,
          startGlyphHeight: lineHeight,
          endGlyphHeight: lineHeight,
          selectionEndpoints: region.selectionEndpoints,
        );
      } on TypeError {
        return TextSelectionToolbarAnchors(
          primaryAnchor: box.localToGlobal(box.size.topCenter(Offset.zero)),
        );
      }
    }
  }

  Widget _selectionMenu(BuildContext context, SelectableRegionState region) {
    final note = ref.read(renderedNoteProvider(widget.relPath)).value;
    final system = region.contextMenuButtonItems;
    if (note == null) {
      return AdaptiveTextSelectionToolbar.buttonItems(
        anchors: _menuAnchors(region),
        buttonItems: system,
      );
    }

    // Read the selection before clearing it: clearing empties
    // [_selectedText] for every block still in the list.
    void run(void Function(Map<int, String>? texts, (int, int) range) action) {
      final range = note.blocks.isEmpty ? null : _selectedRange(note);
      final texts = _allSelected ? null : Map.of(_selectedText);
      region.hideToolbar();
      region.clearSelection();
      if (range != null) action(texts, range);
    }

    final copyAt = system.indexWhere(
      (item) => item.type == ContextMenuButtonType.copy,
    );
    // Right after Copy: the system's Share, Select all and process-text
    // items would otherwise push these into the overflow menu.
    final lead = copyAt < 0
        ? const <ContextMenuButtonItem>[]
        : system.take(copyAt);
    final rest = [
      for (final item in system.skip(copyAt + 1))
        item.type == ContextMenuButtonType.selectAll
            ? item.copyWith(
                onPressed: () {
                  region.selectAll(SelectionChangedCause.toolbar);
                  // After the call: it clears the selection first, which
                  // would reset the flag.
                  _allSelected = true;
                },
              )
            : item,
    ];

    return AdaptiveTextSelectionToolbar.buttonItems(
      anchors: _menuAnchors(region),
      buttonItems: [
        ...lead,
        // Replaces the system Copy, which joins the blocks with nothing in
        // between and only sees the blocks still in the list.
        ContextMenuButtonItem(
          type: ContextMenuButtonType.copy,
          onPressed: () =>
              run((texts, range) => _copySelection(note, texts, range)),
        ),
        ContextMenuButtonItem(
          label: 'Edit here',
          onPressed: () => run(
            (_, range) => EditorScreen.open(
              context,
              widget.relPath,
              row: note.blocks[range.$1].row,
            ),
          ),
        ),
        ContextMenuButtonItem(
          label: 'Copy as Markdown',
          onPressed: () =>
              run((_, range) => _copySource(note, range, markdown: true)),
        ),
        ContextMenuButtonItem(
          label: 'Copy source',
          onPressed: () =>
              run((_, range) => _copySource(note, range, markdown: false)),
        ),
        ...rest,
      ],
    );
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
        bottom: _finding
            ? FindBar(
                controller: _findController,
                countLabel: _find.countLabel,
                hasHits: _find.hits.isNotEmpty,
                onChanged: _onFindChanged,
                onStep: _stepFind,
              )
            : null,
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
            onPressed: _editVisible,
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
        data: (data) => _blockList(data, workspace?.root, textScale),
      ),
    );
  }

  Widget _blockList(RenderedNote data, String? root, double textScale) {
    _applyInitialJump(data);
    if (!identical(_selectionFor, data)) {
      _selectionFor = data;
      _selectedText.clear();
      _latestSelected = null;
      _allSelected = false;
    }
    if (_finding && _find.term.isNotEmpty && !identical(_sourceFor, data)) {
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
      onEmbed: (embed) => openEmbed(context, embed, root, openUrl: _openUrl),
    );

    return SelectionArea(
      contextMenuBuilder: _selectionMenu,
      onSelectionChanged: (content) {
        if ((content != null) != _selecting) {
          setState(() => _selecting = content != null);
        }
      },
      // Flutter scrolls a selection only once the dragged handle's text point
      // leaves the list, and that point sits well above the finger. With the
      // list reaching the screen bottom it never leaves, so a selection gets
      // a strip below the list to drag into.
      child: AnimatedPadding(
        duration: const Duration(milliseconds: 150),
        padding: EdgeInsets.only(bottom: _selecting ? 64 : 0),
        child: SuperListView.builder(
          listController: _listController,
          controller: _scrollController,
          extentEstimation: (_, _) => 30 * textScale,
          padding: const EdgeInsets.only(top: 8, bottom: 32),
          itemCount: data.blocks.length + 1,
          itemBuilder: (context, i) {
            if (i == data.blocks.length) {
              return NoteFooter(relPath: widget.relPath, errors: data.errors);
            }
            return RepaintBoundary(
              key: ValueKey(i),
              child: JoinedSelection(
                onChanged: (text) => _onBlockSelection(i, text),
                onMounted: () => _onBlockMounted(i),
                child: BlockWidget(
                  block: data.blocks[i],
                  actions: actions,
                  root: root,
                  textScale: textScale,
                  highlighted: i == _flashed,
                  matched: _find.hitSet.contains(i),
                  searchTerm: _finding ? _find.term : null,
                  onTaskTap: _changeTaskStatus,
                  onLongPress: (block) => EditorScreen.open(
                    context,
                    widget.relPath,
                    row: block.row,
                  ),
                ),
              ),
            );
          },
        ),
      ),
    );
  }
}
