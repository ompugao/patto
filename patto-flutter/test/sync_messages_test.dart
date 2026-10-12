import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/sync/sync_messages.dart';
import 'package:patto_flutter/src/rust/api/error.dart';
import 'package:patto_flutter/src/rust/api/events.dart';
import 'package:patto_flutter/src/rust/api/git.dart';

SyncReport _report({
  bool committed = false,
  MergeOutcome merge = const MergeOutcome.upToDate(),
  bool pushed = false,
  bool conflictCleared = false,
}) => SyncReport(
  committed: committed,
  merge: merge,
  pushed: pushed,
  conflictCleared: conflictCleared,
  changedPaths: const [],
);

void main() {
  group('syncAdvice', () {
    test('a known git failure gets advice instead of the raw message', () {
      expect(
        syncAdvice(const Failure(message: 'x', gitKind: GitErrorKind.auth)),
        'The server rejected the credentials. Check the username and token.',
      );
      expect(
        syncAdvice(const Failure(message: 'x', gitKind: GitErrorKind.network)),
        'Could not reach the server. Check the connection.',
      );
    });

    test('a certificate failure keeps the message, which names the host', () {
      expect(
        syncAdvice(
          const Failure(message: 'bad cert', gitKind: GitErrorKind.certificate),
        ),
        'The server certificate could not be verified.\n\nbad cert',
      );
    });

    test('anything else shows the message as is', () {
      expect(syncAdvice(const Failure(message: 'disk full')), 'disk full');
    });
  });

  test('the receiving phase shows its progress', () {
    expect(
      syncPhaseLabel(
        GitProgress(
          phase: GitPhase.receiving,
          current: 3,
          total: 10,
          bytes: BigInt.zero,
        ),
      ),
      'Receiving 3/10',
    );
    expect(
      syncPhaseLabel(
        GitProgress(
          phase: GitPhase.pushing,
          current: 0,
          total: 0,
          bytes: BigInt.zero,
        ),
      ),
      'Pushing',
    );
  });

  group('syncReportSummary', () {
    test('lists what the sync did, separated by dots', () {
      expect(
        syncReportSummary(
          _report(
            committed: true,
            merge: const MergeOutcome.merged(),
            pushed: true,
          ),
        ),
        'Committed · Merged · Pushed',
      );
    });

    test('a sync with nothing to do says so', () {
      expect(syncReportSummary(_report()), 'Already up to date');
    });

    test('a cleared conflict is mentioned last', () {
      expect(
        syncReportSummary(
          _report(
            merge: const MergeOutcome.fastForward(),
            conflictCleared: true,
          ),
        ),
        'Fast-forwarded · Conflicts resolved',
      );
    });
  });
}
