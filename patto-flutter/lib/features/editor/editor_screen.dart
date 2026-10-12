import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:re_editor/re_editor.dart';

import '../../core/dates.dart';
import '../../core/embed_lookup.dart';
import '../../core/embed_metadata.dart';
import '../../core/providers.dart';
import '../../src/rust/api/types.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../conflicts/conflict_state.dart';
import 'attachments.dart';
import 'caret_keys.dart';
import 'device_files.dart';
import 'editor_dialogs.dart';
import 'indent_guides.dart';
import 'link_completion.dart';
import 'outline.dart';
import 'outline_edits.dart';
import 'patto_editing_controller.dart';
import 'patto_spans.dart';
import 'trackpad.dart';
import 'widgets/editor_selection_menu.dart';
import 'widgets/editor_toolbar.dart';
import 'widgets/link_candidate_bar.dart';

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
      final edit = await confirmEditingConflicted(context);
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
  late final PattoEditingController _controller = PattoEditingController(
    delegate: CodeLineEditingController(spanBuilder: _buildSpan),
    onIndent: () => _edits.reindent(add: true),
    onOutdent: () => _edits.reindent(add: false),
  );
  late final OutlineEdits _edits = OutlineEdits(_controller);
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
        _edits.jumpTo(row);
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
    _guide = activeGuide(_edits.lines, _controller.selection.extentIndex);
    _updateCompletion();
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
      verbatim: _edits.inVerbatimBlock(index),
    );
  }

  void _updateCompletion() {
    final selection = _controller.selection;
    if (!selection.isCollapsed) return _hideCompletion();

    final line = _controller.codeLines[selection.baseIndex].text;
    final pending = pendingLink(line, selection.baseOffset);
    if (pending == null) return _hideCompletion();

    _pendingLink = (
      line: selection.baseIndex,
      start: pending.start,
      end: selection.baseOffset,
    );
    final query = pending.query;
    if (query != _linkQuery) setState(() => _linkQuery = query);

    _completionDebounce?.cancel();
    _completionDebounce = Timer(const Duration(milliseconds: 120), () async {
      final workspace = await ref.read(workspaceProvider.future);
      if (workspace == null) return;
      try {
        final hits = await rust.searchNotes(
          root: workspace.root,
          query: query,
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

    final line = _controller.codeLines[pending.line].text;
    _controller.selection = CodeLineSelection(
      baseIndex: pending.line,
      baseOffset: pending.start,
      extentIndex: pending.line,
      extentOffset: linkReplaceEnd(line, pending.end),
    );
    _controller.replaceSelection('[$name]');
    _hideCompletion();
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
      _edits.insert(text);
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
      if (mounted) _edits.insert(attachmentSnippet(relPath));
    } catch (e) {
      _notify('Could not save the file: $e');
    }
  }

  Future<void> _insertUrl(String url) async {
    final plain = urlSnippet(url);
    if (!plain.startsWith('[http')) return _edits.insert(plain);

    final notice = ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('Looking up the page title…'),
        duration: EmbedLookup.timeout,
      ),
    );
    try {
      final meta = await LinkPreviews.lookup(url);
      if (mounted) _edits.insert(urlSnippet(url, title: meta?.title));
    } finally {
      notice.close();
    }
  }

  void _notify(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(context)
        .showSnackBar(SnackBar(content: Text(message)));
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

    final choice = await askAboutUnsavedChanges(context);
    if (!mounted || choice == null || choice == UnsavedChoice.cancel) return;
    if (choice == UnsavedChoice.save && !await _save()) return;
    if (mounted) Navigator.of(context).pop();
  }

  Future<void> _saveAndClose() async {
    final saved = await _save();
    if (!saved || !mounted) return;
    Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final name = rust.relPathToNoteName(relPath: widget.relPath);

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
              onPressed: _saveAndClose,
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
                  Expanded(child: _editor(context)),
                  // Without this the editor treats a tap on the bars below
                  // as a tap outside itself, drops focus and closes the
                  // keyboard, taking the caret with it.
                  CodeEditorTapRegion(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        if (_candidates.isNotEmpty ||
                            (_linkQuery?.isNotEmpty ?? false))
                          LinkCandidateBar(
                            candidates: _candidates,
                            query: _linkQuery ?? '',
                            onPick: _acceptLink,
                          ),
                        EditorToolbar(
                          trackpad: _trackpad,
                          onIndent: () => _edits.reindent(add: true),
                          onOutdent: () => _edits.reindent(add: false),
                          onInsert: _edits.insert,
                          onAttach: _attachFile,
                          onPaste: _pasteRich,
                          onMoveBlock: (up) => _edits.moveBlock(up: up),
                          onSelectBlock: _edits.selectBlock,
                          onUndo: _controller.undo,
                          onRedo: _controller.redo,
                          today: () => isoDate(DateTime.now()),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
      ),
    );
  }

  Widget _editor(BuildContext context) {
    final fontSize = 14.0 * ref.watch(fontScaleProvider);
    final tabSize = tabWidth(
      TextStyle(fontFamily: 'monospace', fontSize: fontSize),
    );

    return CaretKeys(
      controller: _controller,
      child: CodeEditor(
        controller: _controller,
        wordWrap: true,
        autofocus: false,
        focusNode: _focusNode,
        // Pairing is for code; in prose it doubles every apostrophe.
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
            lines: () => _edits.lines,
            guide: () => _guide,
            tabWidth: tabSize,
            repaint: _controller,
          );
        },
        toolbarController: MobileSelectionToolbarController(
          builder: buildEditorSelectionMenu,
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
    );
  }
}
