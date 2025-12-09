/// Git configuration model
class GitConfig {
  final String repoUrl;
  final String branch;
  final String? username;
  final String? password;

  const GitConfig({
    required this.repoUrl,
    this.branch = 'main',
    this.username,
    this.password,
  });

  GitConfig copyWith({
    String? repoUrl,
    String? branch,
    String? username,
    String? password,
  }) {
    return GitConfig(
      repoUrl: repoUrl ?? this.repoUrl,
      branch: branch ?? this.branch,
      username: username ?? this.username,
      password: password ?? this.password,
    );
  }

  Map<String, dynamic> toJson() => {
        'repoUrl': repoUrl,
        'branch': branch,
        'username': username,
        'password': password,
      };

  factory GitConfig.fromJson(Map<String, dynamic> json) => GitConfig(
        repoUrl: json['repoUrl'] as String,
        branch: json['branch'] as String? ?? 'main',
        username: json['username'] as String?,
        password: json['password'] as String?,
      );
}

/// Git progress information
class GitProgress {
  final String phase;
  final int current;
  final int total;

  const GitProgress({
    required this.phase,
    required this.current,
    required this.total,
  });

  double get percentage => total > 0 ? current / total : 0;
}
