import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../../core/providers.dart';
import '../../core/settings.dart';
import '../../src/rust/api/types.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../editor/editor_screen.dart';

/// Quick posts into one note, newest at the bottom, like a chat with
/// yourself. Reorganising them into other notes is left to the desktop.
class InboxScreen extends ConsumerStatefulWidget {
  const InboxScreen({super.key});

  @override
  ConsumerState<InboxScreen> createState() => _InboxScreenState();
}

class _InboxScreenState extends ConsumerState<InboxScreen> {
  final _composer = TextEditingController();
  final _focus = FocusNode();
  final _scroll = ScrollController();
  bool _sending = false;
  bool _scrolledOnce = false;

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
    ref.read(inboxDraftProvider.notifier).value = null;
    _composer.text = draft.isEmpty
        ? _composer.text
        : _composer.text.isEmpty
        ? draft
        : '${_composer.text}\n$draft';
    setState(() {});
    // The tab switch that brought the draft lands in the same frame; focus
    // is only accepted once this screen is the visible child.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      _composer.selection = TextSelection.collapsed(
        offset: _composer.text.length,
      );
      _focus.requestFocus();
    });
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
    setState(() => _sending = true);
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
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text('Could not save the post: $e')));
    } finally {
      if (mounted) setState(() => _sending = false);
    }
  }

  Future<void> _openInEditor(InboxPost post) async {
    final workspace = await ref.read(workspaceProvider.future);
    if (workspace == null || !mounted) return;
    final relPath = rust.noteNameToRelPath(
      name: ref.read(inboxNoteNameProvider),
    );
    await EditorScreen.open(context, relPath, row: post.line);
  }

  @override
  Widget build(BuildContext context) {
    final posts = ref.watch(inboxPostsProvider);
    final name = ref.watch(inboxNoteNameProvider);
    final canSend = !_sending && _composer.text.trim().isNotEmpty;

    return Scaffold(
      appBar: AppBar(
        title: name == Settings.defaultInboxNoteName
            ? const Text('Inbox')
            : Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('Inbox'),
                  Text(
                    name,
                    style: Theme.of(context).textTheme.bodySmall,
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
      ),
      body: SafeArea(
        child: Column(
          children: [
            Expanded(
              child: posts.when(
                // Keep the list on screen while a post or an edit refreshes
                // it; a remount would land at the top.
                skipLoadingOnReload: true,
                loading: () => const Center(child: CircularProgressIndicator()),
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
                    onTap: _openInEditor,
                  );
                },
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
