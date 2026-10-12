import '../../src/rust/api/conflict.dart';
import 'conflict_state.dart';

/// The merged content of every clashing note, ready to commit. Throws when
/// a note has no finished draft, or its draft was made against versions that
/// have since changed.
Future<List<Resolution>> collectResolutions(
  PendingConflict pending,
  Map<String, NoteDraft> drafts,
  Future<ConflictDetail> Function(String relPath) detailOf,
) async {
  final resolutions = <Resolution>[];
  for (final file in pending.files) {
    final detail = await detailOf(file.path);
    final draft = drafts[file.path];
    if (draft == null || !draft.matches(detail) || !draft.done) {
      throw StateError('${file.path} changed since it was reviewed');
    }
    resolutions.add(
      Resolution(
        path: file.path,
        oursId: detail.oursId,
        theirsId: detail.theirsId,
        content: mergedContent(detail, draft),
      ),
    );
  }
  return resolutions;
}
