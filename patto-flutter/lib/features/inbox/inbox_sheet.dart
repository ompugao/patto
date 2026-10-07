import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../../core/providers.dart';
import '../../core/settings.dart';
import '../../src/rust/api/types.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../editor/editor_screen.dart';

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
    _composer.text = draft.isEmpty
        ? _composer.text
        : _composer.text.isEmpty
        ? draft
        : '${_composer.text}\n$draft';
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
    final workspace = await ref.read(workspaceProvider.future);
    if (workspace == null || !mounted) return;
    final name = ref.read(inboxNoteNameProvider);
    final relPath = rust.noteNameToRelPath(name: name);
    if (row == null) {
      try {
        await rust.readNote(root: workspace.root, relPath: relPath);
      } catch (_) {
        await rust.createNote(
          root: workspace.root,
          name: name,
          initialContent: '',
        );
        ref.read(notesRevisionProvider.notifier).value++;
      }
      if (!mounted) return;
    }
    final navigator = Navigator.of(context, rootNavigator: true);
    navigator.pop();
    await EditorScreen.open(navigator.context, relPath, row: row);
  }

  @override
  Widget build(BuildContext context) {
    final posts = ref.watch(inboxPostsProvider);
    final name = ref.watch(inboxNoteNameProvider);
    final canSend = !_sending && _composer.text.trim().isNotEmpty;
    final inset = MediaQuery.viewInsetsOf(context).bottom;
    final screen = MediaQuery.sizeOf(context).height;

    return LayoutBuilder(
      builder: (context, constraints) {
        // The sheet sits above the keyboard; the list gives way to it.
        final height = math.min(screen * 0.85, constraints.maxHeight - inset);
        return Padding(
          padding: EdgeInsets.only(bottom: inset),
          child: SizedBox(
            height: height,
            child: Column(
              children: [
                _Header(name: name, onOpenNote: _openNote),
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
                      return _PostList(
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
                _Composer(
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

class _Header extends StatelessWidget {
  const _Header({required this.name, required this.onOpenNote});

  final String name;
  final VoidCallback onOpenNote;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 0, 8, 4),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Inbox', style: theme.textTheme.titleMedium),
                if (name != Settings.defaultInboxNoteName)
                  Text(
                    name,
                    style: theme.textTheme.bodySmall,
                    overflow: TextOverflow.ellipsis,
                  ),
              ],
            ),
          ),
          IconButton(
            icon: const Icon(Icons.open_in_new),
            tooltip: 'Open note',
            onPressed: onOpenNote,
          ),
        ],
      ),
    );
  }
}

class _PostList extends StatelessWidget {
  const _PostList({
    required this.posts,
    required this.controller,
    required this.onTap,
  });

  final List<InboxPost> posts;
  final ScrollController controller;
  final void Function(InboxPost) onTap;

  @override
  Widget build(BuildContext context) {
    if (posts.isEmpty) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Text(
            'Nothing here yet. Write a quick note below; sort it into your '
            'other notes later.',
            textAlign: TextAlign.center,
            style: Theme.of(context).textTheme.bodyMedium
                ?.copyWith(color: Theme.of(context).colorScheme.outline),
          ),
        ),
      );
    }

    final rows = <Widget>[];
    String? date;
    for (final post in posts) {
      if (post.date != date) {
        date = post.date;
        rows.add(_DateHeader(date: date));
      }
      rows.add(_PostTile(post: post, onTap: () => onTap(post)));
    }

    return ListView(
      controller: controller,
      padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
      children: rows,
    );
  }
}

class _DateHeader extends StatelessWidget {
  const _DateHeader({required this.date});

  final String date;

  static String label(String date, DateTime now) {
    final day = DateTime.tryParse(date);
    if (day == null) return date;
    // Calendar days are compared field by field: a Duration across a DST
    // change is not a whole number of days.
    final yesterday = DateTime(now.year, now.month, now.day - 1);
    if (_sameDay(day, now)) return 'Today';
    if (_sameDay(day, yesterday)) return 'Yesterday';
    return DateFormat('EEE, d MMM yyyy').format(day);
  }

  static bool _sameDay(DateTime a, DateTime b) =>
      a.year == b.year && a.month == b.month && a.day == b.day;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 16, 4, 6),
      child: Row(
        children: [
          Expanded(child: Divider(color: theme.colorScheme.outlineVariant)),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12),
            child: Text(
              label(date, DateTime.now()),
              style: theme.textTheme.labelMedium?.copyWith(
                color: theme.colorScheme.outline,
              ),
            ),
          ),
          Expanded(child: Divider(color: theme.colorScheme.outlineVariant)),
        ],
      ),
    );
  }
}

class _PostTile extends StatelessWidget {
  const _PostTile({required this.post, required this.onTap});

  final InboxPost post;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scale = theme.textTheme.bodyLarge!;

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Material(
        color: theme.colorScheme.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  post.time,
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: theme.colorScheme.outline,
                  ),
                ),
                const SizedBox(height: 2),
                Text(post.text, style: scale),
                for (final line in post.body)
                  Padding(
                    padding: const EdgeInsets.only(left: 12, top: 2),
                    child: Text(line, style: scale),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _Composer extends StatelessWidget {
  const _Composer({
    required this.controller,
    required this.focusNode,
    required this.canSend,
    required this.sending,
    required this.onChanged,
    required this.onSend,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final bool canSend;
  final bool sending;
  final VoidCallback onChanged;
  final VoidCallback onSend;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      color: theme.colorScheme.surface,
      elevation: 3,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 8, 8, 8),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            Expanded(
              child: TextField(
                controller: controller,
                focusNode: focusNode,
                autofocus: true,
                minLines: 1,
                maxLines: 5,
                textInputAction: TextInputAction.newline,
                keyboardType: TextInputType.multiline,
                textCapitalization: TextCapitalization.sentences,
                decoration: const InputDecoration(
                  hintText: 'Write a quick note',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
                onChanged: (_) => onChanged(),
              ),
            ),
            const SizedBox(width: 4),
            sending
                ? const Padding(
                    padding: EdgeInsets.all(12),
                    child: SizedBox.square(
                      dimension: 24,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                  )
                : IconButton(
                    icon: const Icon(Icons.send),
                    tooltip: 'Post',
                    onPressed: canSend ? onSend : null,
                  ),
          ],
        ),
      ),
    );
  }
}
