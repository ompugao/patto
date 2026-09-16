import 'package:flutter/material.dart';

import '../../src/rust/api/types.dart';
import '../notes/widgets/task_marker.dart';

/// Bottom sheet for picking a new task status.
class TaskStatusSheet extends StatelessWidget {
  const TaskStatusSheet({super.key, required this.current});

  final TaskStatus current;

  static Future<TaskStatus?> show(BuildContext context, TaskStatus current) {
    return showModalBottomSheet<TaskStatus>(
      context: context,
      builder: (_) => TaskStatusSheet(current: current),
    );
  }

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          for (final status in TaskStatus.values)
            ListTile(
              leading: TaskMarker(status: status),
              title: Text(_label(status)),
              trailing: status == current ? const Icon(Icons.check) : null,
              onTap: () => Navigator.pop(context, status),
            ),
        ],
      ),
    );
  }

  String _label(TaskStatus status) => switch (status) {
    TaskStatus.todo => 'To do',
    TaskStatus.doing => 'Doing',
    TaskStatus.paused => 'Paused',
    TaskStatus.done => 'Done',
  };
}
