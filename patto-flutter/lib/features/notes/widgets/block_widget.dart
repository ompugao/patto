import 'package:flutter/material.dart';

import '../../../src/rust/api/types.dart';
import 'code_block.dart';
import 'embed_card.dart';
import 'line_block.dart';
import 'spans_text.dart';
import 'table_block.dart';

/// One element of a note. Kept free of providers: the list rebuilds these
/// constantly while scrolling.
class BlockWidget extends StatelessWidget {
  const BlockWidget({
    super.key,
    required this.block,
    required this.actions,
    required this.root,
    this.textScale = 1.0,
    this.highlighted = false,
    this.matched = false,
    this.searchTerm,
    this.onTaskTap,
    this.onLongPress,
  });

  static const indentPerLevel = 18.0;

  final Block block;
  final SpanActions actions;
  final String? root;

  /// Multiplier from the appearance setting, applied to every size this block
  /// draws so code, math and tables grow with the prose.
  final double textScale;
  final bool highlighted;

  /// Contains the text being searched for in the note; tinted so matches stand
  /// out while scrolling.
  final bool matched;
  final String? searchTerm;
  final void Function(Block block)? onTaskTap;
  final void Function(Block block)? onLongPress;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final quoted = block.quoteDepth > 0;

    Widget content = switch (block.kind) {
      BlockKind_Blank() => SizedBox(height: 10 * textScale),
      BlockKind_Rule() => const Divider(height: 20),
      BlockKind_Line(:final spans) => LineBlock(
        block: block,
        spans: spans,
        actions: actions,
        root: root,
        textScale: textScale,
        searchTerm: searchTerm,
        onTaskTap: onTaskTap,
      ),
      BlockKind_Code(:final lang, :final lines) => CodeBlock(
        lang: lang,
        lines: lines,
        textScale: textScale,
      ),
      BlockKind_Math(:final tex) => MathBlock(tex: tex, textScale: textScale),
      BlockKind_Table(:final caption, :final rows) => TableBlock(
        caption: caption,
        rows: rows,
        actions: actions,
        root: root,
        textScale: textScale,
      ),
      BlockKind_Images(:final images) => ImagesBlock(
        images: images,
        root: root,
        textScale: textScale,
      ),
      BlockKind_Embed(:final embed) => EmbedCard(
        embed: embed,
        textScale: textScale,
        onTap: () => actions.onEmbed(embed),
      ),
    };

    if (quoted) {
      content = Container(
        margin: EdgeInsets.only(left: (block.quoteDepth - 1) * 10.0),
        padding: const EdgeInsets.only(left: 10),
        decoration: BoxDecoration(
          border: Border(
            left: BorderSide(color: theme.colorScheme.outlineVariant, width: 3),
          ),
        ),
        child: DefaultTextStyle.merge(
          style: TextStyle(color: theme.colorScheme.onSurfaceVariant),
          child: content,
        ),
      );
    }

    final body = Container(
      width: double.infinity,
      color: highlighted
          ? theme.colorScheme.primaryContainer
          : matched
          ? theme.colorScheme.tertiaryContainer.withValues(alpha: 0.35)
          : null,
      padding: EdgeInsets.fromLTRB(16 + block.depth * indentPerLevel, 2, 16, 2),
      child: content,
    );

    if (onLongPress == null) return body;
    return InkWell(onLongPress: () => onLongPress!(block), child: body);
  }
}
