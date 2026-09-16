import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'core/providers.dart';
import 'features/notes/note_list_screen.dart';
import 'features/settings/settings_screen.dart';
import 'features/settings/workspace_editor.dart';
import 'features/tasks/tasks_screen.dart';
import 'features/workspaces/workspace_switcher.dart';
import 'core/workspace.dart';

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
        if (data == null) {
          return const WorkspaceEditorScreen(onboarding: true);
        }
        if (!data.exists) {
          // Chosen but never cloned: offer to fetch it rather than pretending
          // this is a first run, which would hide the other workspaces.
          return _WorkspaceNotReady(workspace: data);
        }

        // Switching workspace lands here with a new root, which is what
        // triggers indexing the one just chosen.
        if (_indexedRoot != data.root) {
          _indexedRoot = data.root;
          WidgetsBinding.instance.addPostFrameCallback((_) {
            ref.read(indexProvider.notifier).ensureBuilt(data.root);
          });
        }

        return const RootShell();
      },
    );
  }
}

/// Shown when the active workspace has no notes folder on the device yet.
class _WorkspaceNotReady extends ConsumerWidget {
  const _WorkspaceNotReady({required this.workspace});

  final ActiveWorkspace workspace;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Scaffold(
      appBar: AppBar(title: Text(workspace.config.name)),
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(Icons.cloud_download_outlined, size: 48),
              const SizedBox(height: 16),
              Text(
                'This workspace has not been cloned onto this device yet.',
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 24),
              FilledButton(
                onPressed: () => WorkspaceEditorScreen.open(
                  context,
                  existing: workspace.config,
                ),
                child: const Text('Clone it'),
              ),
              TextButton(
                onPressed: () => WorkspaceSwitcher.show(context),
                child: const Text('Switch workspace'),
              ),
            ],
          ),
        ),
      ),
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
