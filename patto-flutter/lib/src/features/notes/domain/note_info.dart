/// Note information model
class NoteInfo {
  final String path;
  final String name;
  final DateTime modified;
  final int linkCount;

  const NoteInfo({
    required this.path,
    required this.name,
    required this.modified,
    this.linkCount = 0,
  });

  NoteInfo copyWith({
    String? path,
    String? name,
    DateTime? modified,
    int? linkCount,
  }) {
    return NoteInfo(
      path: path ?? this.path,
      name: name ?? this.name,
      modified: modified ?? this.modified,
      linkCount: linkCount ?? this.linkCount,
    );
  }
}

/// Sort options for note list
enum SortOption {
  recent('Most Recent'),
  linked('Most Linked'),
  title('Title A-Z');

  final String label;
  const SortOption(this.label);
}
