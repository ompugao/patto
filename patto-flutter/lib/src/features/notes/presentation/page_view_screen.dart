import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../../core/theme/app_theme.dart';
import 'notes_provider.dart';
import 'widgets/ast_renderer.dart';

/// Page view screen - displays a single note (readonly)
class PageViewScreen extends ConsumerWidget {
  final String path;
  final String? title;

  const PageViewScreen({
    super.key,
    required this.path,
    this.title,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final contentAsync = ref.watch(noteContentProvider(path));

    return Scaffold(
      appBar: AppBar(
        title: Text(title ?? path.replaceAll('.pn', '').split('/').last),
        actions: [
          IconButton(
            icon: const Icon(Icons.share),
            tooltip: 'Share',
            onPressed: () {
              // TODO: Implement share functionality
            },
          ),
        ],
      ),
      body: contentAsync.when(
        data: (content) => _buildContent(context, ref, content),
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (error, stack) => _buildError(context, error),
      ),
    );
  }

  Widget _buildContent(BuildContext context, WidgetRef ref, String content) {
    // TODO: Parse content using Rust parser via flutter_rust_bridge
    // For now, use a simple text display with basic rendering

    return SingleChildScrollView(
      padding: const EdgeInsets.all(AppSpacing.md),
      child: AstRenderer(
        content: content,
        onWikiLinkTap: (name, anchor) => _handleWikiLink(context, ref, name, anchor),
        onUrlTap: (url) => _handleUrlTap(context, url),
      ),
    );
  }

  Widget _buildError(BuildContext context, Object error) {
    final theme = Theme.of(context);

    return Center(
      child: Padding(
        padding: const EdgeInsets.all(AppSpacing.lg),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.error_outline,
              size: 64,
              color: theme.colorScheme.error,
            ),
            const SizedBox(height: AppSpacing.md),
            Text(
              'Error Loading Note',
              style: theme.textTheme.titleLarge?.copyWith(
                color: theme.colorScheme.error,
              ),
            ),
            const SizedBox(height: AppSpacing.sm),
            Text(
              error.toString(),
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }

  void _handleWikiLink(BuildContext context, WidgetRef ref, String name, String? anchor) {
    // Find the note by name
    final notes = ref.read(notesProvider).valueOrNull ?? [];
    final linkedNote = notes.where((n) => n.name == name).firstOrNull;

    if (linkedNote != null) {
      final encodedPath = Uri.encodeComponent(linkedNote.path);
      context.push('/note/$encodedPath?title=${Uri.encodeComponent(linkedNote.name)}');
    } else {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Note "$name" not found'),
          duration: const Duration(seconds: 2),
        ),
      );
    }
  }

  Future<void> _handleUrlTap(BuildContext context, String url) async {
    final uri = Uri.tryParse(url);
    if (uri != null && await canLaunchUrl(uri)) {
      await launchUrl(uri, mode: LaunchMode.externalApplication);
    } else {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Cannot open: $url')),
        );
      }
    }
  }
}
