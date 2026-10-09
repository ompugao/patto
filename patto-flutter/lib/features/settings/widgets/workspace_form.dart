import 'package:flutter/material.dart';

import '../../../core/workspace.dart';
import '../../editor/attachments.dart';

/// The text of the workspace form, and the workspace it describes.
class WorkspaceFormFields {
  WorkspaceFormFields([Workspace? existing]) {
    if (existing != null) {
      name.text = existing.name;
      repoUrl.text = existing.repoUrl;
      branch.text = existing.branch;
      username.text = existing.username;
      token.text = existing.token;
      attachmentsDir.text = existing.attachmentsDir;
    }
  }

  final name = TextEditingController();
  final repoUrl = TextEditingController();
  final branch = TextEditingController();
  final username = TextEditingController();
  final token = TextEditingController();
  final attachmentsDir = TextEditingController();

  /// Why the attachment folder cannot be used, or null when it can.
  String? get attachmentsDirError =>
      normalizeAttachmentsDir(attachmentsDir.text) == null
      ? 'The attachment folder may only use letters, digits, CJK, '
            '"-", "_" and "/".'
      : null;

  /// The workspace as the form describes it, under the given identity. A
  /// blank name is taken from the repository.
  Workspace toWorkspace({required String id, required String dirName}) {
    final url = repoUrl.text.trim();
    final typedName = name.text.trim();
    return Workspace(
      id: id,
      dirName: dirName,
      name: typedName.isEmpty ? Workspace.nameFromUrl(url) : typedName,
      repoUrl: url,
      branch: branch.text.trim(),
      username: username.text.trim(),
      token: token.text.trim(),
      attachmentsDir:
          normalizeAttachmentsDir(attachmentsDir.text) ?? defaultAttachmentsDir,
    );
  }

  void dispose() {
    for (final c in [name, repoUrl, branch, username, token, attachmentsDir]) {
      c.dispose();
    }
  }
}

/// The fields of the workspace form, with the first-run introduction.
class WorkspaceForm extends StatelessWidget {
  const WorkspaceForm({
    super.key,
    required this.fields,
    required this.onboarding,
  });

  final WorkspaceFormFields fields;
  final bool onboarding;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (onboarding)
          const Padding(
            padding: EdgeInsets.only(bottom: 16),
            child: Text(
              'Patto Notes keeps your notes in a git repository. Enter the '
              'repository to clone it onto this device. You can add more '
              'workspaces later.',
            ),
          ),
        TextField(
          controller: fields.name,
          decoration: const InputDecoration(
            labelText: 'Name',
            helperText: 'Defaults to the repository name',
          ),
        ),
        TextField(
          controller: fields.repoUrl,
          decoration: const InputDecoration(
            labelText: 'HTTPS URL',
            hintText: 'https://github.com/you/notes.git',
          ),
          keyboardType: TextInputType.url,
        ),
        TextField(
          controller: fields.branch,
          decoration: const InputDecoration(
            labelText: 'Branch',
            hintText: 'default branch',
          ),
        ),
        TextField(
          controller: fields.username,
          decoration: const InputDecoration(labelText: 'Username'),
        ),
        TextField(
          controller: fields.token,
          decoration: const InputDecoration(
            labelText: 'Access token',
            helperText: 'Stored in the device keystore',
          ),
          obscureText: true,
        ),
        TextField(
          controller: fields.attachmentsDir,
          decoration: const InputDecoration(
            labelText: 'Attachment folder',
            hintText: defaultAttachmentsDir,
            helperText:
                'Folder in the repository for pictures and files inserted '
                'from the editor. Changing it leaves earlier files where '
                'they are, outside the sync.',
          ),
        ),
      ],
    );
  }
}
