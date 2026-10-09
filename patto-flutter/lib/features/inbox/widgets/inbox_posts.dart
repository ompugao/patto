import 'package:flutter/material.dart';

import '../../../core/settings.dart';
import '../../../src/rust/api/types.dart';
import '../inbox_entry.dart';

class InboxHeader extends StatelessWidget {
  const InboxHeader({super.key, required this.name, required this.onOpenNote});

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

/// The posts in order, under a heading for each day.
class InboxPostList extends StatelessWidget {
  const InboxPostList({
    super.key,
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
              inboxDateLabel(date, DateTime.now()),
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
