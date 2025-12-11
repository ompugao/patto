import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/theme/app_theme.dart';
import '../../settings/presentation/settings_provider.dart';
import 'git_provider.dart';
import '../domain/git_config.dart';

/// Settings screen for git configuration and app settings
class SettingsScreen extends ConsumerStatefulWidget {
  const SettingsScreen({super.key});

  @override
  ConsumerState<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends ConsumerState<SettingsScreen> {
  final _repoUrlController = TextEditingController();
  final _branchController = TextEditingController(text: 'main');
  final _usernameController = TextEditingController();
  final _tokenController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _loadConfig();
  }

  void _loadConfig() {
    final config = ref.read(gitProvider).config;
    if (config != null) {
      _repoUrlController.text = config.repoUrl;
      _branchController.text = config.branch;
      _usernameController.text = config.username ?? '';
      // Don't load token for security
    }
  }

  @override
  void dispose() {
    _repoUrlController.dispose();
    _branchController.dispose();
    _usernameController.dispose();
    _tokenController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final gitState = ref.watch(gitProvider);
    final themeMode = ref.watch(themeModeProvider);
    final theme = Theme.of(context);

    // Show error snackbar
    ref.listen<GitState>(gitProvider, (previous, next) {
      if (next.error != null && previous?.error != next.error) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(next.error!),
            backgroundColor: theme.colorScheme.error,
          ),
        );
        ref.read(gitProvider.notifier).clearError();
      }
    });

    return Scaffold(
      appBar: AppBar(
        title: const Text('Settings'),
      ),
      body: gitState.isLoading
          ? _buildLoading(gitState)
          : _buildContent(context, gitState, themeMode),
    );
  }

  Widget _buildLoading(GitState gitState) {
    final progress = gitState.progress;
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          const CircularProgressIndicator(),
          const SizedBox(height: AppSpacing.md),
          Text(progress?.phase ?? 'Please wait...'),
          if (progress != null && progress.total > 0) ...[
            const SizedBox(height: AppSpacing.sm),
            SizedBox(
              width: 200,
              child: LinearProgressIndicator(
                value: progress.percentage,
              ),
            ),
            const SizedBox(height: AppSpacing.xs),
            Text('${progress.current}/${progress.total}'),
          ],
        ],
      ),
    );
  }

  Widget _buildContent(
    BuildContext context,
    GitState gitState,
    AppThemeMode themeMode,
  ) {
    final theme = Theme.of(context);

    return ListView(
      padding: const EdgeInsets.all(AppSpacing.md),
      children: [
        // Appearance section
        const _SectionHeader(title: 'Appearance'),
        const SizedBox(height: AppSpacing.sm),
        SegmentedButton<AppThemeMode>(
          segments: const [
            ButtonSegment(
              value: AppThemeMode.system,
              label: Text('System'),
              icon: Icon(Icons.brightness_auto),
            ),
            ButtonSegment(
              value: AppThemeMode.light,
              label: Text('Light'),
              icon: Icon(Icons.light_mode),
            ),
            ButtonSegment(
              value: AppThemeMode.dark,
              label: Text('Dark'),
              icon: Icon(Icons.dark_mode),
            ),
          ],
          selected: {themeMode},
          onSelectionChanged: (selection) {
            ref.read(themeModeProvider.notifier).setThemeMode(selection.first);
          },
        ),
        const SizedBox(height: AppSpacing.lg),

        // Git repository section
         const _SectionHeader(title: 'Git Repository'),
        const SizedBox(height: AppSpacing.sm),
        TextField(
          controller: _repoUrlController,
          decoration: const InputDecoration(
            labelText: 'Repository URL *',
            hintText: 'https://github.com/user/notes.git',
            prefixIcon: Icon(Icons.link),
          ),
          keyboardType: TextInputType.url,
          autocorrect: false,
        ),
        const SizedBox(height: AppSpacing.md),
        TextField(
          controller: _branchController,
          decoration: const InputDecoration(
            labelText: 'Branch',
            hintText: 'main',
            prefixIcon: Icon(Icons.account_tree),
          ),
          autocorrect: false,
        ),
        const SizedBox(height: AppSpacing.lg),

        // Authentication section
         const _SectionHeader(title: 'Authentication'),
        const SizedBox(height: AppSpacing.xs),
        Text(
          'For private repositories, enter your GitHub username and a personal access token.',
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: AppSpacing.md),
        TextField(
          controller: _usernameController,
          decoration: const InputDecoration(
            labelText: 'Username',
            hintText: 'GitHub username',
            prefixIcon: Icon(Icons.person),
          ),
          autocorrect: false,
        ),
        const SizedBox(height: AppSpacing.md),
        TextField(
          controller: _tokenController,
          decoration: const InputDecoration(
            labelText: 'Personal Access Token',
            hintText: 'ghp_xxxxxxxxxxxx',
            prefixIcon: Icon(Icons.key),
          ),
          obscureText: true,
          autocorrect: false,
        ),
        const SizedBox(height: AppSpacing.lg),

        // Save/Clone button
        FilledButton.icon(
          onPressed: _handleSave,
          icon: Icon(gitState.isCloned ? Icons.save : Icons.download),
          label: Text(
            gitState.isCloned ? 'Save Configuration' : 'Clone Repository',
          ),
        ),

        if (gitState.isCloned) ...[
          const SizedBox(height: AppSpacing.lg),
          const Divider(),
          const SizedBox(height: AppSpacing.lg),

          // Sync section
           const _SectionHeader(title: 'Sync'),
          const SizedBox(height: AppSpacing.sm),
          OutlinedButton.icon(
            onPressed: _handleSync,
            icon: const Icon(Icons.sync),
            label: const Text('Sync Now'),
          ),
          const SizedBox(height: AppSpacing.lg),

          // Data section
           const _SectionHeader(title: 'Data'),
          const SizedBox(height: AppSpacing.sm),
          FilledButton.icon(
            onPressed: _handleClearData,
            icon: const Icon(Icons.delete_forever),
            label: const Text('Clear Local Data'),
            style: FilledButton.styleFrom(
              backgroundColor: theme.colorScheme.error,
              foregroundColor: theme.colorScheme.onError,
            ),
          ),
          const SizedBox(height: AppSpacing.xs),
          Text(
            'This will remove all locally stored notes. You will need to re-clone the repository.',
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
            textAlign: TextAlign.center,
          ),
        ],

        const SizedBox(height: AppSpacing.xl),
      ],
    );
  }

  Future<void> _handleSave() async {
    final repoUrl = _repoUrlController.text.trim();
    if (repoUrl.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Repository URL is required')),
      );
      return;
    }

    final config = GitConfig(
      repoUrl: repoUrl,
      branch: _branchController.text.trim().isEmpty
          ? 'main'
          : _branchController.text.trim(),
      username: _usernameController.text.trim().isEmpty
          ? null
          : _usernameController.text.trim(),
      password: _tokenController.text.trim().isEmpty
          ? null
          : _tokenController.text.trim(),
    );

    final notifier = ref.read(gitProvider.notifier);
    await notifier.saveConfig(config);

    if (!ref.read(gitProvider).isCloned) {
      await notifier.clone();
      if (mounted && ref.read(gitProvider).isCloned) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Repository cloned successfully')),
        );
        Navigator.of(context).pop();
      }
    } else {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Configuration saved')),
        );
      }
    }
  }

  Future<void> _handleSync() async {
    await ref.read(gitProvider.notifier).pull();
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Sync completed')),
      );
    }
  }

  Future<void> _handleClearData() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Clear Local Data'),
        content: const Text(
          'This will remove all local data. You will need to re-clone the repository.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(context).colorScheme.error,
            ),
            child: const Text('Clear'),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      await ref.read(gitProvider.notifier).clearLocalData();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Local data cleared')),
        );
      }
    }
  }
}

class _SectionHeader extends StatelessWidget {
  final String title;

  const _SectionHeader({required this.title});

  @override
  Widget build(BuildContext context) {
    return Text(
      title,
      style: Theme.of(context).textTheme.titleMedium?.copyWith(
            fontWeight: FontWeight.w600,
          ),
    );
  }
}
