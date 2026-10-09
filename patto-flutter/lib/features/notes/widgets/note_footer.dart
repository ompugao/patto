import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/providers.dart';
import '../../../src/rust/api/types.dart';
import '../../../src/rust/frb_api.dart' as rust;
import '../note_view_screen.dart';

/// Under the note: its syntax issues, backlinks and 2-hop links.
class NoteFooter extends ConsumerWidget {
  const NoteFooter({super.key, required this.relPath, required this.errors});

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
          const _Heading('Backlinks'),
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
          const _Heading('2-hop links'),
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
                                      onPressed: () =>
                                          _openByName(context, ref, name),
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

  Future<void> _openByName(
    BuildContext context,
    WidgetRef ref,
    String name,
  ) async {
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
