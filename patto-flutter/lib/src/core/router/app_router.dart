import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../features/notes/presentation/page_list_screen.dart';
import '../../features/notes/presentation/page_view_screen.dart';
import '../../features/git/presentation/settings_screen.dart';

/// Router provider
final routerProvider = Provider<GoRouter>((ref) {
  return GoRouter(
    initialLocation: '/',
    debugLogDiagnostics: true,
    routes: [
      // Page list (home)
      GoRoute(
        path: '/',
        name: 'home',
        builder: (context, state) => const PageListScreen(),
      ),

      // Page view - capture everything after /note/
      GoRoute(
        path: '/note',
        name: 'note',
        builder: (context, state) {
          // Get path from query parameter instead
          final path = state.uri.queryParameters['path'] ?? '';
          final title = state.uri.queryParameters['title'];

          print('Opening note - path: $path, title: $title');

          return PageViewScreen(
            path: path,
            title: title,
          );
        },
      ),

      // Settings
      GoRoute(
        path: '/settings',
        name: 'settings',
        builder: (context, state) => const SettingsScreen(),
      ),
    ],

    // Error page
    errorBuilder: (context, state) => Scaffold(
      appBar: AppBar(title: const Text('Error')),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Icon(Icons.error_outline, size: 64, color: Colors.red),
            const SizedBox(height: 16),
            Text(
              'Page not found',
              style: Theme.of(context).textTheme.headlineSmall,
            ),
            const SizedBox(height: 8),
            Text(
              state.uri.toString(),
              style: Theme.of(context).textTheme.bodyMedium,
            ),
            const SizedBox(height: 24),
            ElevatedButton(
              onPressed: () => context.go('/'),
              child: const Text('Go Home'),
            ),
          ],
        ),
      ),
    ),
  );
});
