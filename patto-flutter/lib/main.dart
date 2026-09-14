import 'package:flutter/material.dart';

import 'src/rust/frb_generated.dart';
import 'src/rust/frb_api.dart';
import 'src/rust/api/types.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const PattoApp());
}

class PattoApp extends StatelessWidget {
  const PattoApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Patto Notes',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorSchemeSeed: const Color(0xFF3B6EA5),
        useMaterial3: true,
      ),
      darkTheme: ThemeData(
        colorSchemeSeed: const Color(0xFF3B6EA5),
        brightness: Brightness.dark,
        useMaterial3: true,
      ),
      home: const _BridgeCheckScreen(),
    );
  }
}

/// Temporary screen proving the Rust core is reachable from Flutter.
/// Replaced by the note list in the next milestone.
class _BridgeCheckScreen extends StatelessWidget {
  const _BridgeCheckScreen();

  static const _sample = '''
Welcome to [Patto Notes]
\tnested line with [* bold] text
\tship the app {@task status=todo due=2026-12-31}
[@code rust]
\tfn main() {}
''';

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Patto Notes')),
      body: FutureBuilder<RenderedNote>(
        future: renderNote(content: _sample),
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            return Center(child: Text('Rust call failed: ${snapshot.error}'));
          }
          if (!snapshot.hasData) {
            return const Center(child: CircularProgressIndicator());
          }

          final blocks = snapshot.data!.blocks;
          return ListView.builder(
            itemCount: blocks.length,
            itemBuilder: (context, i) => ListTile(
              dense: true,
              title: Text(_describe(blocks[i])),
              subtitle: Text('row ${blocks[i].row}  depth ${blocks[i].depth}'),
            ),
          );
        },
      ),
    );
  }

  String _describe(Block block) => switch (block.kind) {
    BlockKind_Line(:final spans) => spans.map(_spanText).join(),
    BlockKind_Code(:final lang, :final lines) =>
      'code($lang): ${lines.join(" / ")}',
    BlockKind_Math(:final tex) => 'math: $tex',
    BlockKind_Table() => 'table',
    BlockKind_Images(:final images) =>
      'images: ${images.map((i) => i.src).join(", ")}',
    BlockKind_Rule() => '-----',
    BlockKind_Blank() => '',
  };

  String _spanText(NoteSpan span) => switch (span) {
    NoteSpan_Text(:final text) => text,
    NoteSpan_Decoration(:final children) => children.map(_spanText).join(),
    NoteSpan_WikiLink(:final name) => '[$name]',
    NoteSpan_Url(:final url, :final title) => title ?? url,
    NoteSpan_InlineCode(:final code) => code,
    NoteSpan_InlineMath(:final tex) => tex,
    NoteSpan_Image(:final image) => image.src,
    NoteSpan_Embed(:final url) => url,
  };
}
