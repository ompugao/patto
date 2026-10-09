import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../../core/providers.dart';
import '../../src/rust/api/error.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../editor/editor_screen.dart';
import 'inbox_entry.dart';
import 'widgets/inbox_composer.dart';
import 'widgets/inbox_posts.dart';

/// Quick posts into one note, newest at the bottom, like a chat with
/// yourself, opened as a sheet over the notes list. Reorganising the posts
/// into other notes is left to the desktop.
class InboxSheet extends ConsumerStatefulWidget {
  const InboxSheet({super.key});

  static bool _showing = false;

  /// Opens the sheet with the composer focused. A [draft] is put into the
  /// composer; when the sheet is already open it is appended there instead.
  static Future<void> show(BuildContext context, {String? draft}) async {
    final container = ProviderScope.containerOf(context, listen: false);
    if (draft != null) {
      container.read(inboxDraftProvider.notifier).value = draft;
    }
    if (_showing) return;
    _showing = true;
    try {
      await showModalBottomSheet<void>(
        context: context,
        isScrollControlled: true,
        useSafeArea: true,
        useRootNavigator: true,
        showDragHandle: true,
        builder: (_) => const InboxSheet(),
      );
    } finally {
      _showing = false;
    }
    // A draft that arrived while the sheet was closing is still waiting.
    if (container.read(inboxDraftProvider) != null && context.mounted) {
      await show(context);
    }
  }

  @override
  ConsumerState<InboxSheet> createState() => _InboxSheetState();
}

class _InboxSheetState extends ConsumerState<InboxSheet> {
  final _composer = TextEditingController();
  final _focus = FocusNode();
  final _scroll = ScrollController();
  bool _sending = false;
  bool _scrolledOnce = false;
  bool _opening = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    ref.listenManual(inboxDraftProvider, (_, draft) => _takeDraft(draft));
    // Scroll down when posts were added, not when a save merely started.
    ref.listenManual(inboxPostsProvider, (previous, next) {
      final before = previous?.value?.length ?? 0;
      final after = next.value?.length ?? 0;
      if (after > before && before > 0) _scrollToBottom();
    });
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _takeDraft(ref.read(inboxDraftProvider));
    });
  }

  @override
  void dispose() {
    _composer.dispose();
    _focus.dispose();
    _scroll.dispose();
    super.dispose();
  }

  void _takeDraft(String? draft) {
    if (draft == null || !mounted) return;
    // While the sheet is closing the draft stays put; `show` reopens for it.
    if (ModalRoute.of(context)?.isCurrent != true) return;
    ref.read(inboxDraftProvider.notifier).value = null;
    _composer.text = appendDraft(_composer.text, draft);
    _composer.selection = TextSelection.collapsed(
      offset: _composer.text.length,
    );
    setState(() {});
    _focus.requestFocus();
  }

  void _scrollToBottom({bool animate = true}) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_scroll.hasClients) return;
      final end = _scroll.position.maxScrollExtent;
      if (animate) {
        _scroll.animateTo(
          end,
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOut,
        );
      } else {
        _scroll.jumpTo(end);
      }
    });
  }

  Future<void> _send() async {
    final text = _composer.text;
    if (text.trim().isEmpty || _sending) return;
    setState(() {
      _sending = true;
      _error = null;
    });
    final name = ref.read(inboxNoteNameProvider);
    final revision = ref.read(notesRevisionProvider.notifier);
    final workspaceFuture = ref.read(workspaceProvider.future);
    final now = DateTime.now();

    try {
      final workspace = await workspaceFuture;
      if (workspace == null) return;
      await rust.inboxAppend(
        root: workspace.root,
        name: name,
        date: DateFormat('yyyy-MM-dd').format(now),
        time: DateFormat('HH:mm').format(now),
        text: text,
      );
      revision.value++;
      if (!mounted) return;
      _composer.clear();
      _focus.requestFocus();
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = 'Could not save the post: $e');
    } finally {
      if (mounted) setState(() => _sending = false);
    }
  }

  /// Closes the sheet and opens the inbox note in the editor, at [row] when
  /// given. The whole note is created first if no post has made it yet.
  Future<void> _openNote({int? row}) async {
    if (_opening) return;
    _opening = true;
    try {
      final workspace = await ref.read(workspaceProvider.future);
      if (workspace == null || !mounted) return;
      final name = ref.read(inboxNoteNameProvider);
      final relPath = rust.noteNameToRelPath(name: name);
      if (row == null) {
        try {
          await rust.readNote(root: workspace.root, relPath: relPath);
        } on PattoError_NotFound {
          await rust.createNote(
            root: workspace.root,
            name: name,
            initialContent: '',
          );
          ref.read(notesRevisionProvider.notifier).value++;
        }
      }
      // The sheet may have been swiped away while the note was read.
      if (!mounted || ModalRoute.of(context)?.isCurrent != true) return;
      final navigator = Navigator.of(context, rootNavigator: true);
      navigator.pop();
      await EditorScreen.open(navigator.context, relPath, row: row);
    } catch (e) {
      if (mounted) setState(() => _error = 'Could not open the note: $e');
    } finally {
      _opening = false;
    }
  }

  @override
  Widget build(BuildContext context) {
    final posts = ref.watch(inboxPostsProvider);
    final name = ref.watch(inboxNoteNameProvider);
    final canSend = !_sending && _composer.text.trim().isNotEmpty;
    final media = MediaQuery.of(context);
    // The modal sheet only insets the top; below it is either the keyboard
    // or the system navigation bar, whichever is showing.
    final bottom = math.max(media.viewInsets.bottom, media.padding.bottom);
    final screen = media.size.height;

    return LayoutBuilder(
      builder: (context, constraints) {
        final height = math.min(
          screen * 0.85,
          math.max(0.0, constraints.maxHeight - bottom),
        );
        return Padding(
          padding: EdgeInsets.only(bottom: bottom),
          child: SizedBox(
            height: height,
            child: Column(
              children: [
                InboxHeader(name: name, onOpenNote: _openNote),
                Expanded(
                  child: posts.when(
                    // Keep the list on screen while a post or an edit
                    // refreshes it; a remount would land at the top.
                    skipLoadingOnReload: true,
                    loading: () =>
                        const Center(child: CircularProgressIndicator()),
                    error: (e, _) => Center(
                      child: Padding(
                        padding: const EdgeInsets.all(24),
                        child: Text('Could not read the inbox.\n\n$e'),
                      ),
                    ),
                    data: (items) {
                      if (!_scrolledOnce && items.isNotEmpty) {
                        _scrolledOnce = true;
                        _scrollToBottom(animate: false);
                      }
                      return InboxPostList(
                        posts: items,
                        controller: _scroll,
                        onTap: (post) => _openNote(row: post.line),
                      );
                    },
                  ),
                ),
                if (_error != null)
                  Padding(
                    padding: const EdgeInsets.fromLTRB(12, 0, 12, 4),
                    child: Text(
                      _error!,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: Theme.of(context).colorScheme.error,
                      ),
                    ),
                  ),
                InboxComposer(
                  controller: _composer,
                  focusNode: _focus,
                  canSend: canSend,
                  sending: _sending,
                  onChanged: () => setState(() {}),
                  onSend: _send,
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}
