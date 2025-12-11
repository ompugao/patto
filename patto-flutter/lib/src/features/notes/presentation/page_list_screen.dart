import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../core/theme/app_theme.dart';
import '../../git/presentation/git_provider.dart';
import '../domain/note_info.dart';
import 'notes_provider.dart';
import 'widgets/note_list_item.dart';
import 'widgets/sort_selector.dart';

/// Page list screen - displays all notes with search and sort
class PageListScreen extends ConsumerStatefulWidget {
  const PageListScreen({super.key});

  @override
  ConsumerState<PageListScreen> createState() => _PageListScreenState();
}

class _PageListScreenState extends ConsumerState<PageListScreen> {
  final _searchController = TextEditingController();

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final gitState = ref.watch(gitProvider);
    final notesAsync = ref.watch(filteredNotesProvider);
    final sortOption = ref.watch(sortOptionProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Patto Notes'),
        actions: [
          IconButton(
            icon: const Icon(Icons.settings),
            tooltip: 'Settings',
            onPressed: () => context.push('/settings'),
          ),
        ],
      ),
      body: !gitState.isCloned
          ? _buildNotConfigured(context)
          : Column(
              children: [
                // Search bar
                Padding(
                  padding: const EdgeInsets.all(AppSpacing.md),
                  child: SearchBar(
                    controller: _searchController,
                    hintText: 'Search notes...',
                    leading: const Padding(
                      padding: EdgeInsets.only(left: 8),
                      child: Icon(Icons.search),
                    ),
                    trailing: [
                      if (_searchController.text.isNotEmpty)
                        IconButton(
                          icon: const Icon(Icons.clear),
                          onPressed: () {
                            _searchController.clear();
                            ref.read(searchQueryProvider.notifier).state = '';
                          },
                        ),
                    ],
                    onChanged: (query) {
                      ref.read(searchQueryProvider.notifier).state = query;
                    },
                  ),
                ),

                // Sort selector
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: AppSpacing.md),
                  child: SortSelector(
                    value: sortOption,
                    onChanged: (option) {
                      ref.read(sortOptionProvider.notifier).state = option;
                    },
                  ),
                ),
                const SizedBox(height: AppSpacing.sm),

                // Notes list
                Expanded(
                  child: notesAsync.when(
                    data: (notes) => _buildNotesList(context, notes),
                    loading: () => const Center(
                      child: CircularProgressIndicator(),
                    ),
                    error: (error, stack) => Center(
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Icon(
                            Icons.error_outline,
                            size: 64,
                            color: Theme.of(context).colorScheme.error,
                          ),
                          const SizedBox(height: AppSpacing.md),
                          Text('Error: $error'),
                          const SizedBox(height: AppSpacing.md),
                          ElevatedButton(
                            onPressed: () {
                              ref.read(notesProvider.notifier).refresh();
                            },
                            child: const Text('Retry'),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ],
            ),
    );
  }

  Widget _buildNotConfigured(BuildContext context) {
    final theme = Theme.of(context);

    return Center(
      child: Padding(
        padding: const EdgeInsets.all(AppSpacing.lg),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.folder_off_outlined,
              size: 80,
              color: theme.colorScheme.outline,
            ),
            const SizedBox(height: AppSpacing.lg),
            Text(
              'No Repository Configured',
              style: theme.textTheme.headlineSmall?.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: AppSpacing.sm),
            Text(
              'Go to Settings to configure your git repository',
              style: theme.textTheme.bodyLarge?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: AppSpacing.lg),
            FilledButton.icon(
              onPressed: () => context.push('/settings'),
              icon: const Icon(Icons.settings),
              label: const Text('Open Settings'),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildNotesList(BuildContext context, List<NoteInfo> notes) {
    final searchQuery = ref.watch(searchQueryProvider);

    if (notes.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              searchQuery.isNotEmpty ? Icons.search_off : Icons.note_outlined,
              size: 64,
              color: Theme.of(context).colorScheme.outline,
            ),
            const SizedBox(height: AppSpacing.md),
            Text(
              searchQuery.isNotEmpty ? 'No matching notes' : 'No notes found',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: AppSpacing.xs),
            Text(
              searchQuery.isNotEmpty
                  ? 'Try a different search term'
                  : 'Pull to refresh or check your repository',
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
            ),
          ],
        ),
      );
    }

    return RefreshIndicator(
      onRefresh: () async {
        await ref.read(gitProvider.notifier).pull();
        await ref.read(notesProvider.notifier).refresh();
      },
      child: ListView.builder(
        itemCount: notes.length,
        padding: const EdgeInsets.only(bottom: AppSpacing.xl),
        itemBuilder: (context, index) {
          final note = notes[index];
          return NoteListItem(
            note: note,
            onTap: () {
              // Use query parameters to avoid path encoding issues
              final encodedPath = Uri.encodeComponent(note.path);
              final encodedTitle = Uri.encodeComponent(note.name);
              context.push('/note?path=$encodedPath&title=$encodedTitle');
            },
          );
        },
      ),
    );
  }
}
