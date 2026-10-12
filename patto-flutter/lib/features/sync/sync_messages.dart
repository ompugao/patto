import '../../src/rust/api/error.dart';
import '../../src/rust/api/events.dart';
import '../../src/rust/api/git.dart';

/// Turn a failure into something the user can act on.
String syncAdvice(Failure failure) => switch (failure.gitKind) {
  GitErrorKind.auth =>
    'The server rejected the credentials. Check the username and token.',
  GitErrorKind.network => 'Could not reach the server. Check the connection.',
  GitErrorKind.certificate =>
    'The server certificate could not be verified.\n\n${failure.message}',
  GitErrorKind.noRemote => 'This clone has no "origin" remote.',
  GitErrorKind.notARepo =>
    'The notes folder is not a git repository. Clone again.',
  GitErrorKind.nonFastForward =>
    'The remote moved on while syncing. Try again.',
  GitErrorKind.conflict =>
    'The merge could not be resolved here. Resolve it on the desktop.',
  GitErrorKind.stale =>
    'Something changed while you were resolving. Review the conflicts again.',
  _ => failure.message,
};

String syncPhaseLabel(GitProgress p) => switch (p.phase) {
  GitPhase.connecting => 'Connecting',
  GitPhase.counting => 'Counting objects',
  GitPhase.receiving => 'Receiving ${p.current}/${p.total}',
  GitPhase.resolving => 'Resolving',
  GitPhase.checkingOut => 'Checking out',
  GitPhase.committing => 'Committing',
  GitPhase.merging => 'Merging',
  GitPhase.pushing => 'Pushing',
  GitPhase.done => 'Done',
};

/// What a finished sync did, as `Committed · Merged · Pushed`.
String syncReportSummary(SyncReport report) => [
  if (report.committed) 'Committed',
  switch (report.merge) {
    MergeOutcome_UpToDate() => 'Already up to date',
    MergeOutcome_FastForward() => 'Fast-forwarded',
    MergeOutcome_Merged() => 'Merged',
    MergeOutcome_Conflicted() => 'Paused',
  },
  if (report.pushed) 'Pushed',
  if (report.conflictCleared) 'Conflicts resolved',
].join(' · ');
