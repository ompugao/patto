import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/core/settings.dart';
import 'package:patto_flutter/core/workspace.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('workspace', () {
    test('survives a round trip through json', () {
      const workspace = Workspace(
        id: 'ws-1',
        name: 'Work',
        dirName: 'ws-1',
        repoUrl: 'https://example.com/notes.git',
        branch: 'main',
        username: 'someone',
        token: 'secret',
      );

      final restored = Workspace.fromJson(workspace.toJson());

      expect(restored.id, workspace.id);
      expect(restored.name, workspace.name);
      expect(restored.dirName, workspace.dirName);
      expect(restored.repoUrl, workspace.repoUrl);
      expect(restored.branch, workspace.branch);
      expect(restored.username, workspace.username);
    });

    test('keeps the token out of the stored json', () {
      const workspace = Workspace(
        id: 'ws-1',
        name: 'Work',
        dirName: 'ws-1',
        token: 'secret',
      );

      expect(jsonEncode(workspace.toJson()), isNot(contains('secret')));
    });

    test('names itself after the repository', () {
      expect(Workspace.nameFromUrl('https://github.com/you/notes.git'), 'notes');
      expect(Workspace.nameFromUrl('https://github.com/you/notes/'), 'notes');
      expect(Workspace.nameFromUrl(''), 'Notes');
    });
  });

  group('settings', () {
    test('the active workspace falls back to the first one', () {
      const a = Workspace(id: 'a', name: 'A', dirName: 'a');
      const b = Workspace(id: 'b', name: 'B', dirName: 'b');

      expect(const Settings(workspaces: [a, b]).active?.id, 'a');
      expect(
        const Settings(workspaces: [a, b], activeWorkspaceId: 'b').active?.id,
        'b',
      );
      expect(const Settings().active, isNull);
    });

    test('adding a workspace replaces one with the same id', () {
      const first = Workspace(id: 'a', name: 'A', dirName: 'a');
      const renamed = Workspace(id: 'a', name: 'Renamed', dirName: 'a');

      final settings = const Settings().withWorkspace(first).withWorkspace(
        renamed,
      );

      expect(settings.workspaces.length, 1);
      expect(settings.workspaces.single.name, 'Renamed');
      expect(settings.activeWorkspaceId, 'a');
    });

    test('removing the active workspace activates another', () {
      const a = Workspace(id: 'a', name: 'A', dirName: 'a');
      const b = Workspace(id: 'b', name: 'B', dirName: 'b');
      final settings = const Settings(
        workspaces: [a, b],
        activeWorkspaceId: 'a',
      ).withoutWorkspace('a');

      expect(settings.workspaces.single.id, 'b');
      expect(settings.activeWorkspaceId, 'b');
    });

    test('removing the last workspace leaves none active', () {
      const a = Workspace(id: 'a', name: 'A', dirName: 'a');
      final settings = const Settings(
        workspaces: [a],
        activeWorkspaceId: 'a',
      ).withoutWorkspace('a');

      expect(settings.workspaces, isEmpty);
      expect(settings.activeWorkspaceId, isNull);
      expect(settings.active, isNull);
    });
  });

  group('loading', () {
    test('an install with no settings has no workspaces', () async {
      SharedPreferences.setMockInitialValues({});

      final settings = await SettingsStore().load();

      expect(settings.workspaces, isEmpty);
      expect(settings.active, isNull);
    });

    test('a single-workspace install becomes one workspace', () async {
      SharedPreferences.setMockInitialValues({
        'flutter.repoUrl': 'https://github.com/you/notes.git',
        'flutter.branch': 'main',
        'flutter.username': 'you',
        'flutter.authorName': 'You',
        'flutter.themeMode': 'dark',
        'flutter.fontScale': 1.3,
      });

      final settings = await SettingsStore().load();
      final workspace = settings.active!;

      expect(settings.workspaces.length, 1);
      expect(workspace.name, 'notes');
      expect(workspace.repoUrl, 'https://github.com/you/notes.git');
      expect(workspace.branch, 'main');
      expect(workspace.username, 'you');
      // The clone already on disk is kept where it is.
      expect(workspace.dirName, WorkspaceStorage.legacyDirName);
      // Settings that are not workspace-specific carry over.
      expect(settings.authorName, 'You');
      expect(settings.themeMode, ThemeMode.dark);
      expect(settings.fontScale, 1.3);
    });

    /// The migration drops the keys it read, so it must persist what it built.
    test('a migrated workspace is still there on the next launch', () async {
      SharedPreferences.setMockInitialValues({
        'flutter.repoUrl': 'https://github.com/you/notes.git',
      });

      final store = SettingsStore();
      final first = await store.load();
      final second = await store.load();

      expect(first.workspaces.length, 1);
      expect(second.workspaces.length, 1);
      expect(second.active?.dirName, WorkspaceStorage.legacyDirName);
    });

    test('saving then loading keeps the workspaces', () async {
      SharedPreferences.setMockInitialValues({});
      final store = SettingsStore();

      await store.save(
        const Settings(
          workspaces: [
            Workspace(
              id: 'ws-1',
              name: 'Personal',
              dirName: 'ws-1',
              repoUrl: 'https://example.com/a.git',
            ),
            Workspace(id: 'ws-2', name: 'Work', dirName: 'ws-2'),
          ],
          activeWorkspaceId: 'ws-2',
        ),
      );

      final loaded = await store.load();

      expect(loaded.workspaces.map((w) => w.name), ['Personal', 'Work']);
      expect(loaded.active?.id, 'ws-2');
    });

    test('a clamped font scale never leaves the allowed range', () async {
      SharedPreferences.setMockInitialValues({'flutter.fontScale': 99.0});

      final settings = await SettingsStore().load();

      expect(settings.fontScale, Settings.maxFontScale);
    });

    test('corrupt workspace json does not stop the app starting', () async {
      SharedPreferences.setMockInitialValues({
        'flutter.workspaces': 'not json at all',
      });

      final settings = await SettingsStore().load();

      expect(settings.workspaces, isEmpty);
    });
  });
}
