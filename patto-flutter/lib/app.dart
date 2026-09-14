import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'core/providers.dart';
import 'features/notes/note_list_screen.dart';
import 'features/settings/settings_screen.dart';
import 'features/tasks/tasks_screen.dart';

class PattoApp extends ConsumerWidget {
  const PattoApp({super.key});

  static const _seed = Color(0xFF3B6EA5);

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode =
        ref.watch(settingsProvider).value?.themeMode ?? ThemeMode.system;

    return MaterialApp(
      title: 'Patto Notes',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(colorSchemeSeed: _seed, useMaterial3: true),
      darkTheme: ThemeData(
        colorSchemeSeed: _seed,
        brightness: Brightness.dark,
        useMaterial3: true,
      ),
      themeMode: themeMode,
      home: const _Bootstrap(),
    );
  }
}

/// Decides between onboarding and the app, and kicks off the first index build.
class _Bootstrap extends ConsumerStatefulWidget {
  const _Bootstrap();

  @override
  ConsumerState<_Bootstrap> createState() => _BootstrapState();
}

class _BootstrapState extends ConsumerState<_Bootstrap> {
  String? _indexedRoot;

  @override
  Widget build(BuildContext context) {
    final workspace = ref.watch(workspaceProvider);

    return workspace.when(
      loading: () => const Scaffold(
        body: Center(child: CircularProgressIndicator()),
      ),
      error: (e, _) => Scaffold(
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Text('Could not open the notes folder.\n\n$e'),
          ),
        ),
      ),
      data: (data) {
        if (!data.exists) {
          return const SettingsScreen(onboarding: true);
        }

        if (_indexedRoot != data.root) {
          _indexedRoot = data.root;
          WidgetsBinding.instance.addPostFrameCallback((_) {
            ref.read(indexProvider.notifier).rebuild(data.root);
          });
        }

        return const RootShell();
      },
    );
  }
}

/// Bottom navigation over screens kept alive, so tab switches preserve scroll.
class RootShell extends StatefulWidget {
  const RootShell({super.key});

  @override
  State<RootShell> createState() => _RootShellState();
}

class _RootShellState extends State<RootShell> {
  int _tab = 0;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: IndexedStack(
        index: _tab,
        children: const [
          NoteListScreen(),
          TasksScreen(),
          SettingsScreen(),
        ],
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _tab,
        onDestinationSelected: (i) => setState(() => _tab = i),
        destinations: const [
          NavigationDestination(
            icon: Icon(Icons.description_outlined),
            selectedIcon: Icon(Icons.description),
            label: 'Notes',
          ),
          NavigationDestination(
            icon: Icon(Icons.check_circle_outline),
            selectedIcon: Icon(Icons.check_circle),
            label: 'Tasks',
          ),
          NavigationDestination(
            icon: Icon(Icons.settings_outlined),
            selectedIcon: Icon(Icons.settings),
            label: 'Settings',
          ),
        ],
      ),
    );
  }
}
