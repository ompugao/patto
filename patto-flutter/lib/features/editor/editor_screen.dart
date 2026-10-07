import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:re_editor/re_editor.dart';

import '../../core/embed_lookup.dart';
import '../../core/embed_metadata.dart';
import '../../core/providers.dart';
import '../../src/rust/api/types.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../conflicts/conflict_state.dart';
import 'attachments.dart';
import 'device_files.dart';
import 'indent_guides.dart';
import 'outline.dart';
import 'patto_editing_controller.dart';
import 'patto_spans.dart';
import 'trackpad.dart';

/// Full-screen plain-text editor.
///
/// re_editor lays out only the visible lines, so opening a very long note is
/// immediate; a plain text field has to lay out the whole document first.
class EditorScreen extends ConsumerStatefulWidget {
  const EditorScreen({super.key, required this.relPath, this.initialRow});

  final String relPath;
  final int? initialRow;

  static Future<void> open(
    BuildContext context,
    String relPath, {
    int? row,
  }) async {
    final conflicted = ProviderScope.containerOf(
      context,
      listen: false,
    ).read(conflictedPathsProvider);
    if (conflicted.contains(relPath)) {
      final edit = await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          title: const Text('Waiting to be merged'),
          content: const Text(
            'This note changed both here and on another device. New edits '
            'become part of your side and will need merging too.',
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(context, true),
              child: const Text('Edit anyway'),
            ),
          ],
        ),
      );
      if (edit != true || !context.mounted) return;
    }

    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => EditorScreen(relPath: relPath, initialRow: row),
      ),
    );
  }

  @override
  ConsumerState<EditorScreen> createState() => _EditorScreenState();
}

class _EditorScreenState extends ConsumerState<EditorScreen> {
  /// `[` followed by a partial name, excluding the block and decoration forms.
  static final _linkTrigger = RegExp(r'\[([^\[\]\s@*/`$_-]*)$');

  late final _controller = PattoEditingController(
    delegate: CodeLineEditingController(spanBuilder: _buildSpan),
    onIndent: () => _reindent(add: true),
    onOutdent: () => _reindent(add: false),
  );
  final _focusNode = FocusNode();
  Timer? _completionDebounce;

  bool _loading = true;
  String? _error;

  /// The text as loaded or last saved; compared on demand rather than tracked
  /// by a flag, so a missed rebuild can never disable saving.
  String _savedText = '';
  List<NoteMeta> _candidates = const [];
  ({int line, int start, int end})? _pendingLink;

  /// What has been typed after `[`, offered as a new link when no note matches.
  String? _linkQuery;

  CodeLines? _linesSource;
  List<String> _linesCache = const [];

  /// The indent column that shows which block the caret is in.
  ({int column, int start, int end})? _guide;

  /// The visible lines as last laid out, handed over through the gutter.
  CodeIndicatorValueNotifier? _layout;

  late final _trackpad = CaretTrackpad(
    controller: _controller,
    paragraphs: () => _layout?.value?.paragraphs,
  );

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _completionDebounce?.cancel();
    _controller.removeListener(_onChanged);
    _controller.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final workspace = await ref.read(workspaceProvider.future);
      if (workspace == null) {
        throw StateError('no workspace is active');
      }
      final content = await rust.readNote(
        root: workspace.root,
        relPath: widget.relPath,
      );
      if (!mounted) return;

      _savedText = content;
      _controller.text = content;
      // Loading the note is itself an edit, and undoing it would empty the
      // buffer.
      _controller.clearHistory();
      _controller.addListener(_onChanged);
      setState(() => _loading = false);

      final row = widget.initialRow;
      if (row != null && row < _controller.codeLines.length) {
        _jumpTo(row);
      }
      // Opening the editor is asking to type; don't make it take another tap.
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _focusNode.requestFocus();
      });
    } catch (e) {
      if (mounted) setState(() => (_loading = false, _error = e.toString()));
    }
  }

  bool get _dirty => _controller.text != _savedText;

  void _onChanged() {
    // Read by the guide painter, which repaints on every change.
    _guide = activeGuide(_lines, _controller.selection.extentIndex);
    _updateCompletion();
  }

  /// The visible lines' text; a folded block is its one visible line.
  List<String> get _lines {
    final codeLines = _controller.codeLines;
    if (!identical(codeLines, _linesSource)) {
      _linesSource = codeLines;
      _linesCache = [
        for (var i = 0; i < codeLines.length; i++) codeLines[i].text,
      ];
    }
    return _linesCache;
  }

  TextSpan _buildSpan({
    required BuildContext context,
    required int index,
    required CodeLine codeLine,
    required TextSpan textSpan,
    required TextStyle style,
  }) {
    return pattoLineSpan(
      text: codeLine.text,
      style: style,
      styles: PattoSpanStyles(Theme.of(context).colorScheme),
      verbatim: _inVerbatimBlock(index),
    );
  }

  bool _inVerbatimBlock(int row) {
    final lines = _lines;
    if (row >= lines.length) return false;
    for (int? p = parentOf(lines, row); p != null; p = parentOf(lines, p)) {
      if (opensVerbatim(lines[p])) return true;
    }
    return false;
  }

  void _updateCompletion() {
    final selection = _controller.selection;
    if (!selection.isCollapsed) return _hideCompletion();

    final line = _controller.codeLines[selection.baseIndex].text;
    final before = line.substring(
      0,
      selection.baseOffset.clamp(0, line.length),
    );
    final match = _linkTrigger.firstMatch(before);

    if (match == null || match.group(1)!.startsWith('http')) {
      return _hideCompletion();
    }

    _pendingLink = (
      line: selection.baseIndex,
      start: match.start,
      end: selection.baseOffset,
    );
    final query = match.group(1)!;
    if (query != _linkQuery) setState(() => _linkQuery = query);

    _completionDebounce?.cancel();
    _completionDebounce = Timer(const Duration(milliseconds: 120), () async {
      final workspace = await ref.read(workspaceProvider.future);
      if (workspace == null) return;
      try {
        final hits = await rust.searchNotes(
          root: workspace.root,
          query: match.group(1)!,
          limit: 12,
        );
        if (mounted) setState(() => _candidates = hits);
      } catch (_) {
        if (mounted) setState(() => _candidates = const []);
      }
    });
  }

  void _hideCompletion() {
    _completionDebounce?.cancel();
    _pendingLink = null;
    if (_candidates.isNotEmpty || _linkQuery != null) {
      setState(() => (_candidates = const [], _linkQuery = null));
    }
  }

  void _acceptLink(String name) {
    final pending = _pendingLink;
    if (pending == null) return;

    // The toolbar's `[]` leaves its closing `]` after the caret; take it
    // along rather than leave it dangling after the link.
    final line = _controller.codeLines[pending.line].text;
    final end = pending.end < line.length && line[pending.end] == ']'
        ? pending.end + 1
        : pending.end;
    _controller.selection = CodeLineSelection(
      baseIndex: pending.line,
      baseOffset: pending.start,
      extentIndex: pending.line,
      extentOffset: end,
    );
    _controller.replaceSelection('[$name]');
    _hideCompletion();
  }

  /// Indents or outdents the selected lines together with their children.
  /// Soft keyboards have no Tab key, so the toolbar is the way to nest.
  void _reindent({required bool add}) {
    final selection = _controller.selection;
    final lines = _lines;
    final start = selection.startIndex;
    var last = selection.endIndex;
    // A selection that ends at the start of a line does not take that line.
    if (last > start && selection.endOffset == 0) last--;
    var end = last + 1;
    for (var i = start; i <= last; i++) {
      if (!isBlank(lines[i])) end = math.max(end, blockEnd(lines, i));
    }
    // Outdenting a top-level block would only flatten its children.
    if (!add && depthOf(lines[start]) == 0) return;

    final codeLines = CodeLines.from(_controller.codeLines);
    final shift = <int, int>{};
    for (var i = start; i < end; i++) {
      final text = lines[i];
      // Leave empty lines empty instead of filling them with tabs.
      if (text.isEmpty) continue;
      if (!add && !text.startsWith('\t')) continue;
      codeLines[i] = _shifted(codeLines[i], add: add);
      shift[i] = add ? 1 : -1;
    }
    if (shift.isEmpty) return;

    int offsetOn(int index, int offset) =>
        (offset + (shift[index] ?? 0)).clamp(0, codeLines[index].length);
    _edit(
      codeLines,
      selection.copyWith(
        baseOffset: offsetOn(selection.baseIndex, selection.baseOffset),
        extentOffset: offsetOn(selection.extentIndex, selection.extentOffset),
      ),
    );
  }

  /// [line] one level deeper or shallower, along with any lines folded into it.
  static CodeLine _shifted(CodeLine line, {required bool add}) {
    final text = line.text;
    return CodeLine(
      add
          ? (text.isEmpty ? text : '\t$text')
          : (text.startsWith('\t') ? text.substring(1) : text),
      [for (final chunk in line.chunks) _shifted(chunk, add: add)],
    );
  }

  /// Swaps the caret's block with the sibling block above or below it.
  void _moveBlock({required bool up}) {
    final selection = _controller.selection;
    final lines = _lines;
    final row = selection.startIndex;
    if (isBlank(lines[row])) return;
    final moved = moveBlock(lines, row, up: up);
    if (moved == null) return;

    final old = _controller.codeLines;
    final codeLines = CodeLines.of([for (final i in moved.order) old[i]]);
    _edit(
      codeLines,
      selection.copyWith(
        baseIndex: moved.order.indexOf(selection.baseIndex),
        extentIndex: moved.order.indexOf(selection.extentIndex),
      ),
    );
    _controller.makeCursorCenterIfInvisible();
  }

  /// Selects the caret's block; once it is selected, widens to its parent's.
  void _selectBlock() {
    final selection = _controller.selection;
    final lines = _lines;
    var start = selection.startIndex;
    var end = isBlank(lines[start]) ? start + 1 : blockEnd(lines, start);

    bool covers(int s, int e) =>
        selection.startIndex == s &&
        selection.startOffset == 0 &&
        selection.endIndex == e - 1 &&
        selection.endOffset == lines[e - 1].length;
    if (covers(start, end)) {
      final parent = parentOf(lines, start);
      if (parent == null) return;
      start = parent;
      end = blockEnd(lines, parent);
    }
    _controller.selection = CodeLineSelection(
      baseIndex: start,
      baseOffset: 0,
      extentIndex: end - 1,
      extentOffset: lines[end - 1].length,
    );
  }

  /// Puts the caret at the start of the text on [row].
  void _jumpTo(int row) {
    _controller.selection = CodeLineSelection.collapsed(
      index: row,
      offset: depthOf(_lines[row]),
    );
    _controller.makeCursorCenterIfInvisible();
  }

  /// Replaces the lines as a single undoable step.
  void _edit(CodeLines codeLines, CodeLineSelection selection) {
    _controller.runRevocableOp(() {
      _controller.value = _controller.value.copyWith(
        codeLines: codeLines,
        selection: selection,
      );
    });
  }

  /// Inserts [text] and leaves the caret [back] characters before its end,
  /// inside the brackets where there is something to type.
  void _insert(String text, [int back = 0]) {
    _controller.replaceSelection(text);
    if (back == 0) return;
    final caret = _controller.selection.extent;
    _controller.selection = CodeLineSelection.collapsed(
      index: caret.index,
      offset: caret.offset - back,
    );
  }

  /// Copies a file the user picks into the workspace and inserts a reference
  /// to it: an image, an embedded PDF, or a link for anything else.
  Future<void> _attachFile() => _guarded(() async {
    final picked = await DeviceFiles.pickFile();
    if (!mounted) return;
    if (picked == null) return _notify('No file was picked');
    await _insertAttachment(
      picked.name,
      (root, dir) => saveAttachmentFile(root, dir, picked.name, picked.file),
    );
  });

  /// Inserts what the clipboard holds in the form the note understands: an
  /// image is saved into the workspace, a web address becomes a link, an
  /// embed or an image by what it points at, and other text is pasted as is.
  Future<void> _pasteRich() => _guarded(() async {
    final image = await DeviceFiles.clipboardImage();
    if (!mounted) return;
    if (image != null) {
      final name = pastedImageName(
        widget.relPath,
        image.bytes,
        DateTime.now(),
        mimeType: image.mimeType,
      );
      await _insertAttachment(
        name,
        (root, dir) => saveAttachment(root, dir, name, image.bytes),
      );
      return;
    }
    final text = (await Clipboard.getData(Clipboard.kTextPlain))?.text;
    if (!mounted) return;
    if (text == null || text.isEmpty) {
      _notify('Nothing to paste');
    } else if (isWebUrl(text)) {
      await _insertUrl(text.trim());
    } else {
      _insert(text);
    }
  });

  bool _inserting = false;

  /// One insertion at a time: a second tap while a picker or lookup is open
  /// would insert twice.
  Future<void> _guarded(Future<void> Function() action) async {
    if (_inserting) return;
    _inserting = true;
    try {
      await action();
    } finally {
      _inserting = false;
    }
  }

  /// Runs [save] against the active workspace and inserts a reference to the
  /// path it returns.
  Future<void> _insertAttachment(
    String name,
    Future<String> Function(String root, String attachmentsDir) save,
  ) async {
    final workspace = await ref.read(workspaceProvider.future);
    if (!mounted) return;
    if (workspace == null) return _notify('No workspace is active');
    try {
      final relPath = await save(
        workspace.root,
        workspace.config.attachmentsDir,
      );
      if (mounted) _insert(attachmentSnippet(relPath));
    } catch (e) {
      _notify('Could not save the file: $e');
    }
  }

  Future<void> _insertUrl(String url) async {
    final plain = urlSnippet(url);
    if (!plain.startsWith('[http')) return _insert(plain);

    final notice = ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('Looking up the page title…'),
        duration: EmbedLookup.timeout,
      ),
    );
    try {
      final meta = await LinkPreviews.lookup(url);
      if (mounted) _insert(urlSnippet(url, title: meta?.title));
    } finally {
      notice.close();
    }
  }

  void _notify(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(context)
        .showSnackBar(SnackBar(content: Text(message)));
  }

  /// re_editor implements the long-press menu but leaves the widget to the
  /// application, so without this there is no cut, copy or paste.
  Widget _buildSelectionMenu({
    required BuildContext context,
    required TextSelectionToolbarAnchors anchors,
    required CodeLineEditingController controller,
    required VoidCallback onDismiss,
    required VoidCallback onRefresh,
  }) {
    void run(void Function() action) {
      action();
      onDismiss();
    }

    return AdaptiveTextSelectionToolbar.buttonItems(
      anchors: anchors,
      buttonItems: [
        if (!controller.selection.isCollapsed) ...[
          ContextMenuButtonItem(
            type: ContextMenuButtonType.cut,
            onPressed: () => run(controller.cut),
          ),
          ContextMenuButtonItem(
            type: ContextMenuButtonType.copy,
            onPressed: () => run(controller.copy),
          ),
        ],
        ContextMenuButtonItem(
          type: ContextMenuButtonType.paste,
          onPressed: () => run(controller.paste),
        ),
        ContextMenuButtonItem(
          type: ContextMenuButtonType.selectAll,
          onPressed: () {
            controller.selectAll();
            onRefresh();
          },
        ),
      ],
    );
  }

  String _today() {
    final now = DateTime.now();
    return '${now.year.toString().padLeft(4, '0')}-'
        '${now.month.toString().padLeft(2, '0')}-'
        '${now.day.toString().padLeft(2, '0')}';
  }

  Future<bool> _save() async {
    final text = _controller.text;
    if (text == _savedText) return true;

    final workspace = await ref.read(workspaceProvider.future);
    if (workspace == null) return false;
    try {
      await rust.writeNote(
        root: workspace.root,
        relPath: widget.relPath,
        content: text,
      );
      _savedText = text;
      ref.read(notesRevisionProvider.notifier).value++;
      return true;
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context)
            .showSnackBar(SnackBar(content: Text('Could not save: $e')));
      }
      return false;
    }
  }

  Future<void> _confirmExit() async {
    if (!_dirty) {
      if (mounted) Navigator.of(context).pop();
      return;
    }

    final choice = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Unsaved changes'),
        content: const Text('Save before leaving?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, 'cancel'),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context, 'discard'),
            child: const Text('Discard'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, 'save'),
            child: const Text('Save'),
          ),
        ],
      ),
    );

    if (!mounted || choice == null || choice == 'cancel') return;
    if (choice == 'save' && !await _save()) return;
    if (mounted) Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final name = rust.relPathToNoteName(relPath: widget.relPath);
    final fontSize = 14.0 * ref.watch(fontScaleProvider);
    final tabSize = tabWidth(
      TextStyle(fontFamily: 'monospace', fontSize: fontSize),
    );

    return PopScope(
      canPop: !_dirty,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) _confirmExit();
      },
      child: Scaffold(
        appBar: AppBar(
          title: Text(name, overflow: TextOverflow.ellipsis),
          actions: [
            IconButton(
              icon: const Icon(Icons.check),
              tooltip: 'Save',
              onPressed: () async {
                final saved = await _save();
                if (!saved || !mounted) return;
                Navigator.of(this.context).pop();
              },
            ),
          ],
        ),
        body: _loading
            ? const Center(child: CircularProgressIndicator())
            : _error != null
            ? Center(
                child: Padding(
                  padding: const EdgeInsets.all(24),
                  child: Text('Could not open this note.\n\n$_error'),
                ),
              )
            : Column(
                children: [
                  Expanded(
                    child: CodeEditor(
                      controller: _controller,
                      wordWrap: true,
                      autofocus: false,
                      focusNode: _focusNode,
                      // Pairing is for code; in prose it doubles every
                      // apostrophe.
                      autocompleteSymbols: false,
                      padding: const EdgeInsets.fromLTRB(4, 12, 12, 12),
                      chunkAnalyzer: const PattoIndentChunkAnalyzer(),
                      // Fold markers, and the indent guides beside them.
                      indicatorBuilder: (context, editing, chunks, notifier) {
                        _layout = notifier;
                        return IndentGuideGutter(
                          width: 20,
                          chunks: chunks,
                          notifier: notifier,
                          lines: () => _lines,
                          guide: () => _guide,
                          tabWidth: tabSize,
                          repaint: _controller,
                        );
                      },
                      toolbarController: MobileSelectionToolbarController(
                        builder: _buildSelectionMenu,
                      ),
                      style: CodeEditorStyle(
                        fontFamily: 'monospace',
                        fontSize: fontSize,
                        fontHeight: 1.45,
                        textColor: Theme.of(context).colorScheme.onSurface,
                        cursorLineColor: Theme.of(context).colorScheme.primary
                            .withValues(alpha: 0.06),
                      ),
                    ),
                  ),
                  // Without this the editor treats a tap on the bars below
                  // as a tap outside itself, drops focus and closes the
                  // keyboard, taking the caret with it.
                  CodeEditorTapRegion(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        if (_candidates.isNotEmpty ||
                            (_linkQuery?.isNotEmpty ?? false))
                          _CandidateBar(
                            candidates: _candidates,
                            query: _linkQuery ?? '',
                            onPick: _acceptLink,
                          ),
                        _Toolbar(
                          trackpad: _trackpad,
                          onIndent: () => _reindent(add: true),
                          onOutdent: () => _reindent(add: false),
                          onInsert: _insert,
                          onAttach: _attachFile,
                          onPaste: _pasteRich,
                          onMoveBlock: (up) => _moveBlock(up: up),
                          onSelectBlock: _selectBlock,
                          onUndo: _controller.undo,
                          onRedo: _controller.redo,
                          today: _today,
                        ),
                      ],
                    ),
                  ),
                ],
              ),
      ),
    );
  }
}

class _CandidateBar extends StatelessWidget {
  const _CandidateBar({
    required this.candidates,
    required this.query,
    required this.onPick,
  });

  final List<NoteMeta> candidates;
  final String query;
  final void Function(String name) onPick;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final offerNew =
        query.isNotEmpty && !candidates.any((c) => c.name == query);
    return Container(
      height: 44,
      color: theme.colorScheme.surfaceContainerHighest,
      child: ListView(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 8),
        children: [
          for (final c in candidates)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 6),
              child: ActionChip(
                label: Text(c.name),
                onPressed: () => onPick(c.name),
              ),
            ),
          // Links to a note that does not exist yet are how new notes start.
          if (offerNew)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 6),
              child: ActionChip(
                avatar: const Icon(Icons.add, size: 16),
                label: Text(query),
                onPressed: () => onPick(query),
              ),
            ),
        ],
      ),
    );
  }
}

class _Toolbar extends StatelessWidget {
  const _Toolbar({
    required this.trackpad,
    required this.onIndent,
    required this.onOutdent,
    required this.onInsert,
    required this.onAttach,
    required this.onPaste,
    required this.onMoveBlock,
    required this.onSelectBlock,
    required this.onUndo,
    required this.onRedo,
    required this.today,
  });

  final CaretTrackpad trackpad;
  final VoidCallback onIndent;
  final VoidCallback onOutdent;
  final void Function(String text, [int back]) onInsert;
  final VoidCallback onAttach;
  final VoidCallback onPaste;
  final void Function(bool up) onMoveBlock;
  final VoidCallback onSelectBlock;
  final VoidCallback onUndo;
  final VoidCallback onRedo;
  final String Function() today;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      color: theme.colorScheme.surfaceContainer,
      child: SafeArea(
        top: false,
        child: SizedBox(
          height: 48,
          // Holding the bar, or dragging its handle, moves the caret.
          child: TrackpadRegion(
            trackpad: trackpad,
            child: Row(
              children: [
                TrackpadHandle(trackpad: trackpad),
                IconButton(
                  icon: const Icon(Icons.format_indent_increase),
                  tooltip: 'Indent block',
                  onPressed: onIndent,
                ),
                IconButton(
                  icon: const Icon(Icons.format_indent_decrease),
                  tooltip: 'Outdent block',
                  onPressed: onOutdent,
                ),
                const VerticalDivider(width: 8),
                Expanded(
                  child: ListView(
                    scrollDirection: Axis.horizontal,
                    children: [
                      IconButton(
                        icon: const Icon(Icons.move_up),
                        tooltip: 'Move block up',
                        onPressed: () => onMoveBlock(true),
                      ),
                      IconButton(
                        icon: const Icon(Icons.move_down),
                        tooltip: 'Move block down',
                        onPressed: () => onMoveBlock(false),
                      ),
                      IconButton(
                        icon: const Icon(Icons.highlight_alt),
                        tooltip: 'Select block',
                        onPressed: onSelectBlock,
                      ),
                      const VerticalDivider(width: 8),
                      IconButton(
                        icon: const Icon(Icons.undo),
                        tooltip: 'Undo',
                        onPressed: onUndo,
                      ),
                      IconButton(
                        icon: const Icon(Icons.redo),
                        tooltip: 'Redo',
                        onPressed: onRedo,
                      ),
                      const VerticalDivider(width: 8),
                      IconButton(
                        icon: const Icon(Icons.attach_file),
                        tooltip: 'Attach a file',
                        onPressed: onAttach,
                      ),
                      IconButton(
                        icon: const Icon(Icons.content_paste_go),
                        tooltip: 'Paste as image, link or embed',
                        onPressed: onPaste,
                      ),
                      const VerticalDivider(width: 8),
                      TextButton(
                        onPressed: () => onInsert('[]', 1),
                        child: const Text('[ ]'),
                      ),
                      TextButton(
                        onPressed: () => onInsert('{@task status=todo}'),
                        child: const Text('task'),
                      ),
                      TextButton(
                        onPressed: () => onInsert('!${today()}'),
                        child: const Text('due'),
                      ),
                      TextButton(
                        onPressed: () => onInsert('[@code ]', 1),
                        child: const Text('code'),
                      ),
                      TextButton(
                        onPressed: () => onInsert('[@quote]'),
                        child: const Text('quote'),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
