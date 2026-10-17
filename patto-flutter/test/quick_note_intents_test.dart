import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/core/quick_note_intents.dart';

void main() {
  test('reads the workspace a shortcut names', () {
    final request = QuickNoteRequest.fromMap({
      'text': '',
      'subject': null,
      'workspaceId': 'ws-a',
    });
    expect(request?.workspaceId, 'ws-a');
    expect(request?.withoutWorkspace().workspaceId, isNull);
  });

  test('a held request skips the current listener for the next one', () async {
    final intents = QuickNoteIntents();
    final first = <QuickNoteRequest>[];
    final sub = intents.requests.listen(first.add);

    intents.hold(const QuickNoteRequest(text: 'held'));
    await sub.cancel();
    final second = <QuickNoteRequest>[];
    intents.requests.listen(second.add);
    await pumpEventQueue();

    expect(first, isEmpty);
    expect(second.map((r) => r.text), ['held']);
  });

  test('release hands a held request to a listener that stayed', () async {
    final intents = QuickNoteIntents();
    final got = <QuickNoteRequest>[];
    intents.requests.listen(got.add);

    intents.hold(const QuickNoteRequest(text: 'held'));
    intents.release();
    await pumpEventQueue();

    expect(got.map((r) => r.text), ['held']);
  });
}
