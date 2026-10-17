import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'workspace.dart';

/// Text another app shared with us, or an empty request from a launcher or
/// home-screen shortcut.
class QuickNoteRequest {
  const QuickNoteRequest({required this.text, this.subject, this.workspaceId});

  final String text;
  final String? subject;

  /// The workspace a home-screen shortcut was made for; null means the
  /// active one.
  final String? workspaceId;

  QuickNoteRequest withoutWorkspace() =>
      QuickNoteRequest(text: text, subject: subject);

  static QuickNoteRequest? fromMap(Object? raw) {
    if (raw is! Map) return null;
    return QuickNoteRequest(
      text: raw['text'] as String? ?? '',
      subject: raw['subject'] as String?,
      workspaceId: raw['workspaceId'] as String?,
    );
  }
}

/// Quick-note intents from MainActivity: the one the app was launched with is
/// fetched once with `consume`; later ones arrive as `quickNote` calls.
///
/// A request that arrives while nobody listens (onboarding, a workspace that
/// is not cloned yet) is kept and handed to the next listener.
class QuickNoteIntents {
  QuickNoteIntents() {
    _requests.onListen = () {
      final held = _undelivered;
      _undelivered = null;
      if (held != null) _requests.add(held);
    };
  }

  static const _channel = MethodChannel('com.sifi.patto/quick_note');

  final _requests = StreamController<QuickNoteRequest>.broadcast();
  QuickNoteRequest? _undelivered;
  bool _started = false;

  Stream<QuickNoteRequest> get requests => _requests.stream;

  Future<void> start() async {
    if (_started) return;
    _started = true;
    _channel.setMethodCallHandler((call) async {
      if (call.method == 'quickNote') {
        _add(QuickNoteRequest.fromMap(call.arguments));
      }
    });
    try {
      _add(QuickNoteRequest.fromMap(await _channel.invokeMethod('consume')));
    } on MissingPluginException {
      // Not running inside MainActivity (tests, other platforms).
    }
  }

  /// Keeps [request] for the next listener rather than the current one, which
  /// is about to go away, e.g. across a workspace switch.
  void hold(QuickNoteRequest request) => _undelivered = request;

  /// Hands a held request to the current listener, for when the one expected
  /// to replace it never came.
  void release() {
    final held = _undelivered;
    if (held == null || !_requests.hasListener) return;
    _undelivered = null;
    _requests.add(held);
  }

  void _add(QuickNoteRequest? request) {
    if (request == null) return;
    if (_requests.hasListener) {
      _requests.add(request);
    } else {
      _undelivered = request;
    }
  }

  /// Asks the launcher for a home-screen icon that opens the inbox of
  /// [workspace]. False when the launcher cannot pin shortcuts.
  static Future<bool> pinWorkspaceShortcut(Workspace workspace) async {
    try {
      final pinned = await _channel.invokeMethod<bool>('pinWorkspaceShortcut', {
        'id': workspace.id,
        'name': workspace.name,
      });
      return pinned ?? false;
    } on MissingPluginException {
      return false;
    } on PlatformException {
      return false;
    }
  }

  static List<(String, String)>? _published;

  /// Offers one quick-note entry per workspace in the launcher's long-press
  /// menu. Only sent when an id or a name changed.
  static Future<void> setWorkspaceShortcuts(List<Workspace> workspaces) async {
    final next = [for (final w in workspaces) (w.id, w.name)];
    if (listEquals(next, _published)) return;
    _published = next;
    try {
      await _channel.invokeMethod<void>('setWorkspaceShortcuts', {
        'workspaces': [
          for (final (id, name) in next) {'id': id, 'name': name},
        ],
      });
    } on MissingPluginException {
      // Not running inside MainActivity.
    } on PlatformException {
      _published = null;
    }
  }

  void dispose() {
    _channel.setMethodCallHandler(null);
    _requests.close();
  }
}
