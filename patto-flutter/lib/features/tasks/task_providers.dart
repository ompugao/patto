import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/dates.dart';
import '../../core/providers.dart';
import '../../src/rust/api/tasks.dart';
import '../../src/rust/frb_api.dart' as rust;

final pendingTasksProvider = FutureProvider<List<TaskItem>>((ref) {
  return indexedQuery(
    ref,
    empty: const [],
    query: (root) => rust.pendingTasks(root: root),
  );
});

/// Timeframe for the completed-tasks review.
final reviewTimeframeProvider = valueProvider<String>('today');
final reviewRangeProvider = valueProvider<DateTimeRange?>(null);

final completedTasksProvider = FutureProvider<List<TaskItem>>((ref) {
  return indexedQuery(
    ref,
    empty: const [],
    query: (root) {
      final timeframe = ref.watch(reviewTimeframeProvider);
      final range = ref.watch(reviewRangeProvider);
      return rust.completedTasks(
        root: root,
        timeframe: timeframe,
        from: range == null ? null : isoDate(range.start),
        to: range == null ? null : isoDate(range.end),
      );
    },
  );
});
