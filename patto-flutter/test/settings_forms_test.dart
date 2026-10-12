import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/core/workspace.dart';
import 'package:patto_flutter/features/settings/inbox_note_name.dart';
import 'package:patto_flutter/features/settings/widgets/workspace_form.dart';

void main() {
  group('inboxNoteNameError', () {
    String? error(String name, {bool coreAccepts = true}) =>
        inboxNoteNameError(name, coreAccepts: (_) => coreAccepts);

    test('an empty name is allowed, meaning the default', () {
      expect(error(''), isNull);
      expect(error('   '), isNull);
    });

    test('an anchor separator is called out by name', () {
      expect(error('Inbox#1'), 'A note name cannot contain #');
    });

    test('a path the app or the core refuses is not a valid note name', () {
      expect(error('../x'), 'Not a valid note name');
      expect(error('fine', coreAccepts: false), 'Not a valid note name');
    });

    test('a usable name has no error', () {
      expect(error('journal/inbox'), isNull);
    });
  });

  group('WorkspaceFormFields', () {
    test('starts from the workspace being edited', () {
      const existing = Workspace(
        id: 'ws',
        name: 'Work',
        dirName: 'ws',
        repoUrl: 'https://example.com/w.git',
        branch: 'main',
        username: 'me',
        token: 't',
        attachmentsDir: 'media',
      );
      final fields = WorkspaceFormFields(existing);
      expect(fields.name.text, 'Work');
      expect(fields.repoUrl.text, 'https://example.com/w.git');
      expect(fields.branch.text, 'main');
      expect(fields.username.text, 'me');
      expect(fields.token.text, 't');
      expect(fields.attachmentsDir.text, 'media');
      fields.dispose();
    });

    test('a blank name is taken from the repository', () {
      final fields = WorkspaceFormFields()
        ..repoUrl.text = ' https://github.com/you/notes.git ';
      final workspace = fields.toWorkspace(id: 'ws-1', dirName: 'ws-1');
      expect(workspace.name, 'notes');
      expect(workspace.repoUrl, 'https://github.com/you/notes.git');
      expect(workspace.attachmentsDir, defaultAttachmentsDir);
      fields.dispose();
    });

    test('fields are trimmed and the identity is the one given', () {
      final fields = WorkspaceFormFields()
        ..name.text = ' Mine '
        ..branch.text = ' dev '
        ..username.text = ' u '
        ..token.text = ' t '
        ..attachmentsDir.text = '/files/';
      final workspace = fields.toWorkspace(id: 'id', dirName: 'dir');
      expect(workspace.id, 'id');
      expect(workspace.dirName, 'dir');
      expect(workspace.name, 'Mine');
      expect(workspace.branch, 'dev');
      expect(workspace.username, 'u');
      expect(workspace.token, 't');
      expect(workspace.attachmentsDir, 'files');
      fields.dispose();
    });

    test('an unspellable attachment folder is an error', () {
      final fields = WorkspaceFormFields()..attachmentsDir.text = 'my files';
      expect(fields.attachmentsDirError, contains('attachment folder'));
      fields.attachmentsDir.text = '';
      expect(fields.attachmentsDirError, isNull);
      fields.dispose();
    });
  });
}
