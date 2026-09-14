import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers.dart';
import '../../src/rust/api/tasks.dart';
import '../../src/rust/api/types.dart';
import '../../src/rust/frb_api.dart' as rust;
import '../notes/note_view_screen.dart';
import '../notes/widgets/task_marker.dart';
import 'task_status_sheet.dart';

class TasksScreen extends ConsumerWidget {
  const TasksScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return DefaultTabController(
      length: 2,
      child: Scaffold(
        appBar: AppBar(
          title: const Text('Tasks'),
          bottom: const TabBar(
            tabs: [Tab(text: 'Pending'), Tab(text: 'Completed')],
          ),
        ),
        body: const TabBarView(
          children: [_PendingTab(), _CompletedTab()],
        ),
      ),
    );
  }
}

/// Fixed display order for the pending buckets.
const _groupOrder = [
  PendingGroup.overdue,
  PendingGroup.today,
  PendingGroup.thisWeek,
  PendingGroup.later,
  PendingGroup.noDue,
];

String _groupLabel(PendingGroup group) => switch (group) {
  PendingGroup.overdue => 'Overdue',
  PendingGroup.today => 'Today',
  PendingGroup.thisWeek => 'This week',
  PendingGroup.later => 'Later',
  PendingGroup.noDue => 'No deadline',
};

class _PendingTab extends ConsumerWidget {
  const _PendingTab();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tasks = ref.watch(pendingTasksProvider);
    final indexing = ref.watch(indexProvider).building;

    return tasks.when(
      loading: () => const Center(child: CircularProgressIndicator()),
      error: (e, _) => _Message('Could not load tasks.\n\n$e'),
      data: (list) {
        if (list.isEmpty) {
          return _Message(indexing ? 'Indexing…' : 'No pending tasks.');
        }

        final rows = <Object>[];
        for (final group in _groupOrder) {
          final inGroup = list.where((t) => t.group == group).toList();
          if (inGroup.isEmpty) continue;
          rows.add(_Header('${_groupLabel(group)} (${inGroup.length})'));
          rows.addAll(inGroup);
        }

        return ListView.builder(
          itemCount: rows.length,
          itemBuilder: (context, i) {
            final row = rows[i];
            if (row is _Header) return row;
            return _TaskTile(task: row as TaskItem);
          },
        );
      },
    );
  }
}

class _CompletedTab extends ConsumerWidget {
  const _CompletedTab();

  static const _timeframes = {
    'today': 'Today',
    'yesterday': 'Yesterday',
    'this_week': 'This week',
    'last_week': 'Last week',
    'this_month': 'This month',
  };

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final timeframe = ref.watch(reviewTimeframeProvider);
    final tasks = ref.watch(completedTasksProvider);

    return Column(
      children: [
        SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          child: Row(
            children: [
              for (final entry in _timeframes.entries)
                Padding(
                  padding: const EdgeInsets.only(right: 6),
                  child: ChoiceChip(
                    label: Text(entry.value),
                    selected: timeframe == entry.key,
                    onSelected: (_) {
                      ref.read(reviewTimeframeProvider.notifier).value = entry.key;
                      ref.read(reviewRangeProvider.notifier).value = null;
                    },
                  ),
                ),
              Padding(
                padding: const EdgeInsets.only(right: 6),
                child: ChoiceChip(
                  label: const Text('Custom'),
                  selected: timeframe == 'custom',
                  onSelected: (_) => _pickRange(context, ref),
                ),
              ),
            ],
          ),
        ),
        Expanded(
          child: tasks.when(
            loading: () => const Center(child: CircularProgressIndicator()),
            error: (e, _) => _Message('Could not load tasks.\n\n$e'),
            data: (list) {
              if (list.isEmpty) {
                return const _Message('Nothing completed in this period.');
              }

              final rows = <Object>[];
              String? currentDate;
              for (final task in list) {
                if (task.completedOn != currentDate) {
                  currentDate = task.completedOn;
                  rows.add(_Header(currentDate ?? 'Unknown date'));
                }
                rows.add(task);
              }

              return ListView.builder(
                itemCount: rows.length,
                itemBuilder: (context, i) {
                  final row = rows[i];
                  if (row is _Header) return row;
                  return _TaskTile(task: row as TaskItem);
                },
              );
            },
          ),
        ),
      ],
    );
  }

  Future<void> _pickRange(BuildContext context, WidgetRef ref) async {
    final now = DateTime.now();
    final range = await showDateRangePicker(
      context: context,
      firstDate: DateTime(now.year - 5),
      lastDate: DateTime(now.year + 1),
    );
    if (range == null) return;
    ref.read(reviewRangeProvider.notifier).value = range;
    ref.read(reviewTimeframeProvider.notifier).value = 'custom';
  }
}

class _TaskTile extends ConsumerWidget {
  const _TaskTile({required this.task});

  final TaskItem task;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final due = task.info.due;

    return ListTile(
      leading: TaskMarker(
        status: task.info.status,
        onTap: () => _change(context, ref),
      ),
      title: Text(
        task.text.isEmpty ? '(no text)' : task.text,
        maxLines: 2,
        overflow: TextOverflow.ellipsis,
        style: task.info.status == TaskStatus.done
            ? TextStyle(
                decoration: TextDecoration.lineThrough,
                color: theme.colorScheme.outline,
              )
            : null,
      ),
      subtitle: Text(task.noteName, overflow: TextOverflow.ellipsis),
      trailing: due == null ? null : DueChip(due: due),
      onTap: () => NoteViewScreen.open(context, task.relPath, row: task.row),
      onLongPress: () => _change(context, ref),
    );
  }

  Future<void> _change(BuildContext context, WidgetRef ref) async {
    final next = await TaskStatusSheet.show(context, task.info.status);
    if (next == null || !context.mounted) return;

    final workspace = await ref.read(workspaceProvider.future);
    try {
      await rust.setTaskStatus(
        root: workspace.root,
        relPath: task.relPath,
        row: task.row,
        status: next,
      );
      ref.read(notesRevisionProvider.notifier).value++;
    } catch (e) {
      if (!context.mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Could not update the task: $e')),
      );
    }
  }
}

class _Header extends StatelessWidget {
  const _Header(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      width: double.infinity,
      color: theme.colorScheme.surfaceContainerHighest,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
      child: Text(text, style: theme.textTheme.labelLarge),
    );
  }
}

class _Message extends StatelessWidget {
  const _Message(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Text(text, textAlign: TextAlign.center),
      ),
    );
  }
}
