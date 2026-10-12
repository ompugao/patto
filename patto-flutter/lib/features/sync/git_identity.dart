import '../../core/workspace.dart';
import '../../src/rust/api/git.dart';

GitCreds gitCredsFor(Workspace workspace) =>
    GitCreds(username: workspace.username, token: workspace.token);
