import 'dart:async';

import 'package:flutter/services.dart';

/// Text another app shared with us, or an empty request from the launcher
/// shortcut.
class QuickNoteRequest {
  const QuickNoteRequest({required this.text, this.subject});

  final String text;
  final String? subject;

  static QuickNoteRequest? fromMap(Object? raw) {
    if (raw is! Map) return null;
    return QuickNoteRequest(
      text: raw['text'] as String? ?? '',
      subject: raw['subject'] as String?,
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

  void _add(QuickNoteRequest? request) {
    if (request == null) return;
    if (_requests.hasListener) {
      _requests.add(request);
    } else {
      _undelivered = request;
    }
  }

  void dispose() {
    _channel.setMethodCallHandler(null);
    _requests.close();
  }
}
