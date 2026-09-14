import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:re_editor/re_editor.dart';

import '../../core/providers.dart';
import '../../src/rust/api/types.dart';
import '../../src/rust/frb_api.dart' as rust;

/// Full-screen plain-text editor.
///
/// re_editor lays out only the visible lines, so opening a very long note is
/// immediate; a plain text field has to lay out the whole document first.
class EditorScreen extends ConsumerStatefulWidget {
  const EditorScreen({super.key, required this.relPath, this.initialRow});

  final String relPath;
  final int? initialRow;

  static Future<void> open(BuildContext context, String relPath, {int? row}) {
    return Navigator.of(context).push(
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

  final _controller = CodeLineEditingController();
  Timer? _completionDebounce;

  bool _loading = true;
  String? _error;

  /// The text as loaded or last saved; compared on demand rather than tracked
  /// by a flag, so a missed rebuild can never disable saving.
  String _savedText = '';
  List<NoteMeta> _candidates = const [];
  ({int line, int start, int end})? _pendingLink;

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
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final workspace = await ref.read(workspaceProvider.future);
      final content = await rust.readNote(
        root: workspace.root,
        relPath: widget.relPath,
      );
      if (!mounted) return;

      _savedText = content;
      _controller.text = content;
      _controller.addListener(_onChanged);
      setState(() => _loading = false);

      final row = widget.initialRow;
      if (row != null && row < _controller.codeLines.length) {
        _controller.selection = CodeLineSelection.collapsed(
          index: row,
          offset: 0,
        );
        _controller.makeCursorCenterIfInvisible();
      }
    } catch (e) {
      if (mounted) setState(() => (_loading = false, _error = e.toString()));
    }
  }

  bool get _dirty => _controller.text != _savedText;

  void _onChanged() {
    _updateCompletion();
  }

  void _updateCompletion() {
    final selection = _controller.selection;
    if (!selection.isCollapsed) return _hideCompletion();

    final line = _controller.codeLines[selection.baseIndex].text;
    final before = line.substring(0, selection.baseOffset.clamp(0, line.length));
    final match = _linkTrigger.firstMatch(before);

    if (match == null || match.group(1)!.startsWith('http')) {
      return _hideCompletion();
    }

    _pendingLink = (
      line: selection.baseIndex,
      start: match.start,
      end: selection.baseOffset,
    );

    _completionDebounce?.cancel();
    _completionDebounce = Timer(const Duration(milliseconds: 120), () async {
      final workspace = await ref.read(workspaceProvider.future);
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
    if (_candidates.isNotEmpty) setState(() => _candidates = const []);
  }

  void _acceptCandidate(NoteMeta note) {
    final pending = _pendingLink;
    if (pending == null) return;

    _controller.selection = CodeLineSelection(
      baseIndex: pending.line,
      baseOffset: pending.start,
      extentIndex: pending.line,
      extentOffset: pending.end,
    );
    _controller.replaceSelection('[${note.name}]');
    _hideCompletion();
  }

  /// Soft keyboards have no Tab key, and patto nests with tabs, so the toolbar
  /// is the real way to indent. re_editor's own indent inserts spaces, which
  /// patto does not read as nesting.
  void _reindent({required bool add}) {
    final selection = _controller.selection;
    if (add && selection.isCollapsed) {
      _controller.replaceSelection('\t');
      return;
    }

    final lines = CodeLines.from(_controller.codeLines);
    for (var i = selection.startIndex; i <= selection.endIndex && i < lines.length; i++) {
      final text = lines[i].text;
      if (add) {
        lines[i] = lines[i].copyWith(text: '\t$text');
      } else if (text.startsWith('\t')) {
        lines[i] = lines[i].copyWith(text: text.substring(1));
      }
    }
    _controller.codeLines = lines;
  }

  void _insert(String text) => _controller.replaceSelection(text);

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
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Could not save: $e')),
        );
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
                          padding: const EdgeInsets.all(12),
                          style: CodeEditorStyle(
                            fontFamily: 'monospace',
                            fontSize: 14,
                            fontHeight: 1.45,
                            textColor: Theme.of(context).colorScheme.onSurface,
                          ),
                        ),
                      ),
                      if (_candidates.isNotEmpty)
                        _CandidateBar(
                          candidates: _candidates,
                          onPick: _acceptCandidate,
                        ),
                      _Toolbar(
                        onIndent: () => _reindent(add: true),
                        onOutdent: () => _reindent(add: false),
                        onInsert: _insert,
                        today: _today,
                      ),
                    ],
                  ),
      ),
    );
  }
}

class _CandidateBar extends StatelessWidget {
  const _CandidateBar({required this.candidates, required this.onPick});

  final List<NoteMeta> candidates;
  final void Function(NoteMeta) onPick;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      height: 44,
      color: theme.colorScheme.surfaceContainerHighest,
      child: ListView.builder(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 8),
        itemCount: candidates.length,
        itemBuilder: (context, i) => Padding(
          padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 6),
          child: ActionChip(
            label: Text(candidates[i].name),
            onPressed: () => onPick(candidates[i]),
          ),
        ),
      ),
    );
  }
}

class _Toolbar extends StatelessWidget {
  const _Toolbar({
    required this.onIndent,
    required this.onOutdent,
    required this.onInsert,
    required this.today,
  });

  final VoidCallback onIndent;
  final VoidCallback onOutdent;
  final void Function(String) onInsert;
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
          child: ListView(
            scrollDirection: Axis.horizontal,
            children: [
              IconButton(
                icon: const Icon(Icons.format_indent_increase),
                tooltip: 'Indent',
                onPressed: onIndent,
              ),
              IconButton(
                icon: const Icon(Icons.format_indent_decrease),
                tooltip: 'Outdent',
                onPressed: onOutdent,
              ),
              const VerticalDivider(width: 8),
              TextButton(onPressed: () => onInsert('[]'), child: const Text('[ ]')),
              TextButton(
                onPressed: () => onInsert('{@task status=todo}'),
                child: const Text('task'),
              ),
              TextButton(
                onPressed: () => onInsert('!${today()}'),
                child: const Text('due'),
              ),
              TextButton(
                onPressed: () => onInsert('[@code ]'),
                child: const Text('code'),
              ),
              TextButton(
                onPressed: () => onInsert('[@quote]'),
                child: const Text('quote'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
