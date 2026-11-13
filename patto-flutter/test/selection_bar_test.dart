import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:patto_flutter/features/notes/widgets/selection_bar.dart';

void main() {
  testWidgets('each button runs its action', (tester) async {
    final pressed = <String>[];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          bottomNavigationBar: SelectionBar(
            onCopy: () => pressed.add('copy'),
            onCopyMarkdown: () => pressed.add('markdown'),
            onCopySource: () => pressed.add('source'),
            onEdit: () => pressed.add('edit'),
            onClose: () => pressed.add('close'),
          ),
        ),
      ),
    );

    for (final label in ['Copy', 'Markdown', 'Source', 'Edit']) {
      await tester.tap(find.text(label));
    }
    await tester.tap(find.byTooltip('Clear selection'));

    expect(pressed, ['copy', 'markdown', 'source', 'edit', 'close']);
  });
}
