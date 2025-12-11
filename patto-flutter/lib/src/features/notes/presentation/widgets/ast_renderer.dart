import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../../../../core/theme/app_theme.dart';

/// AST Renderer widget - renders Patto content
///
/// This is a simplified renderer that works with raw content.
/// In the full implementation, this will use the Rust parser via flutter_rust_bridge.
class AstRenderer extends StatelessWidget {
  final String content;
  final void Function(String name, String? anchor)? onWikiLinkTap;
  final void Function(String url)? onUrlTap;

  const AstRenderer({
    super.key,
    required this.content,
    this.onWikiLinkTap,
    this.onUrlTap,
  });

  @override
  Widget build(BuildContext context) {
    final lines = content.split('\n');
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: lines.map((line) => _buildLine(context, line)).toList(),
    );
  }

  Widget _buildLine(BuildContext context, String line) {
    final theme = Theme.of(context);

    // Skip empty lines but add spacing
    if (line.trim().isEmpty) {
      return const SizedBox(height: AppSpacing.sm);
    }

    // Calculate indent
    final indent = line.length - line.trimLeft().length;
    final depth = indent ~/ 2;

    // Check for horizontal rule
    if (RegExp(r'^-{5,}$').hasMatch(line.trim())) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: AppSpacing.sm),
        child: Divider(color: theme.colorScheme.outline),
      );
    }

    // Check for code block start
    if (line.trim().startsWith('[@code')) {
      return _buildCodeBlockHeader(context, line);
    }

    // Check for math block start
    if (line.trim().startsWith('[@math')) {
      return _buildMathBlockHeader(context, line);
    }

    // Check for quote block start
    if (line.trim().startsWith('[@quote')) {
      return _buildQuoteBlockHeader(context, line);
    }

    // Parse inline content
    final spans = _parseInlineContent(context, line.trimLeft());

    return Padding(
      padding: EdgeInsets.only(
        left: depth * 16.0,
        top: AppSpacing.xs,
        bottom: AppSpacing.xs,
      ),
      child: Text.rich(
        TextSpan(children: spans),
        style: theme.textTheme.bodyLarge,
        softWrap: true,
        textWidthBasis: TextWidthBasis.parent,
      ),
    );
  }

  Widget _buildCodeBlockHeader(BuildContext context, String line) {
    final theme = Theme.of(context);
    final langMatch = RegExp(r'\[@code\s+(\w+)').firstMatch(line);
    final language = langMatch?.group(1) ?? '';

    return Container(
      margin: const EdgeInsets.only(top: AppSpacing.sm),
      padding: const EdgeInsets.all(AppSpacing.sm),
      decoration: BoxDecoration(
        color: theme.brightness == Brightness.dark
            ? AppColors.darkCode
            : AppColors.lightCode,
        borderRadius: const BorderRadius.only(
          topLeft: Radius.circular(8),
          topRight: Radius.circular(8),
        ),
      ),
      child: Row(
        children: [
          Icon(
            Icons.code,
            size: 16,
            color: theme.colorScheme.onSurfaceVariant,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              language.isNotEmpty ? language.toUpperCase() : 'CODE',
              style: theme.textTheme.labelSmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildMathBlockHeader(BuildContext context, String line) {
    final theme = Theme.of(context);

    return Container(
      margin: const EdgeInsets.only(top: AppSpacing.sm),
      padding: const EdgeInsets.all(AppSpacing.sm),
      decoration: const BoxDecoration(
        color: Color(0xFFFFF9E6),
        borderRadius: BorderRadius.only(
          topLeft: Radius.circular(8),
          topRight: Radius.circular(8),
        ),
      ),
      child: Row(
        children: [
          Icon(
            Icons.functions,
            size: 16,
            color: theme.colorScheme.onSurfaceVariant,
          ),
          const SizedBox(width: 8),
          Text(
            'MATH',
            style: theme.textTheme.labelSmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildQuoteBlockHeader(BuildContext context, String line) {
    final theme = Theme.of(context);
    final citeMatch = RegExp(r'\[@quote\s+"([^"]+)"').firstMatch(line);
    final cite = citeMatch?.group(1);

    return Container(
      margin: const EdgeInsets.only(top: AppSpacing.sm),
      padding: const EdgeInsets.all(AppSpacing.sm),
      decoration: BoxDecoration(
        border: Border(
          left: BorderSide(
            color: theme.colorScheme.primary,
            width: 4,
          ),
        ),
      ),
      child: Row(
        children: [
          Icon(
            Icons.format_quote,
            size: 16,
            color: theme.colorScheme.onSurfaceVariant,
          ),
          if (cite != null) ...[
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                '— $cite',
                style: theme.textTheme.labelSmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                  fontStyle: FontStyle.italic,
                ),
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ],
      ),
    );
  }

  List<InlineSpan> _parseInlineContent(BuildContext context, String text) {
    final theme = Theme.of(context);
    final spans = <InlineSpan>[];
    var remaining = text;

    // Pattern definitions
    final patterns = [
      // Inline code: [`code`]
      (
        regex: RegExp(r'\[`([^`]*)`\]'),
        builder: (Match m) => TextSpan(
              text: m.group(1),
              style: TextStyle(
                fontFamily: 'monospace',
                backgroundColor: theme.brightness == Brightness.dark
                    ? AppColors.darkCode
                    : AppColors.lightCode,
                color: theme.brightness == Brightness.dark
                    ? AppColors.darkCodeText
                    : AppColors.lightCodeText,
              ),
            ),
      ),
      // Inline math: [$formula$]
      (
        regex: RegExp(r'\[\$([^$]*)\$\]'),
        builder: (Match m) => TextSpan(
              text: m.group(1),
               style: const TextStyle(
                 fontFamily: 'monospace',
                 backgroundColor: Color(0xFFFFF9E6),
               ),
             ),
      ),
      // Bold: [* text]
      (
        regex: RegExp(r'\[\*\s+([^\]]+)\]'),
        builder: (Match m) => TextSpan(
              text: m.group(1),
              style: const TextStyle(fontWeight: FontWeight.bold),
            ),
      ),
      // Italic: [/ text]
      (
        regex: RegExp(r'\[\/\s+([^\]]+)\]'),
        builder: (Match m) => TextSpan(
              text: m.group(1),
              style: const TextStyle(fontStyle: FontStyle.italic),
            ),
      ),
      // Underline: [_ text]
      (
        regex: RegExp(r'\[_\s+([^\]]+)\]'),
        builder: (Match m) => TextSpan(
              text: m.group(1),
              style: const TextStyle(decoration: TextDecoration.underline),
            ),
      ),
      // Deleted: [- text]
      (
        regex: RegExp(r'\[-\s+([^\]]+)\]'),
        builder: (Match m) => TextSpan(
              text: m.group(1),
              style: TextStyle(
                decoration: TextDecoration.lineThrough,
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
      ),
      // URL link: [title url] where url contains ://
      (
        regex: RegExp(r'\[([^\]]*?)\s+(https?://[^\]]+)\]'),
        builder: (Match m) => TextSpan(
              text: m.group(1)?.isNotEmpty == true ? m.group(1) : m.group(2),
              style: TextStyle(
                color: theme.colorScheme.primary,
                decoration: TextDecoration.underline,
              ),
              recognizer: TapGestureRecognizer()
                ..onTap = () => onUrlTap?.call(m.group(2)!),
            ),
      ),
      // URL link without title: [url]
      (
        regex: RegExp(r'\[(https?://[^\]]+)\]'),
        builder: (Match m) => TextSpan(
              text: m.group(1),
              style: TextStyle(
                color: theme.colorScheme.primary,
                decoration: TextDecoration.underline,
              ),
              recognizer: TapGestureRecognizer()
                ..onTap = () => onUrlTap?.call(m.group(1)!),
            ),
      ),
      // Bare URL: https://... or http://...
      (
        regex: RegExp(r'https?://[^\s\[\]]+'),
        builder: (Match m) => TextSpan(
              text: m.group(0),
              style: TextStyle(
                color: theme.colorScheme.primary,
                decoration: TextDecoration.underline,
              ),
              recognizer: TapGestureRecognizer()
                ..onTap = () => onUrlTap?.call(m.group(0)!),
            ),
      ),
      // Wiki link with anchor: [PageName#anchor]
      (
        regex: RegExp(r'\[([^\[\]@`$\/\s][^\[\]#]*?)#([^\[\]]+)\]'),
        builder: (Match m) => TextSpan(
              text: '[${m.group(1)}#${m.group(2)}]',
              style: TextStyle(
                color: theme.colorScheme.primary,
                decoration: TextDecoration.underline,
              ),
              recognizer: TapGestureRecognizer()
                ..onTap = () => onWikiLinkTap?.call(m.group(1)!, m.group(2)),
            ),
      ),
      // Wiki link: [PageName]
      (
        regex: RegExp(r'\[([^\[\]@`$\/\s][^\[\]]*?)\]'),
        builder: (Match m) {
          final name = m.group(1)!.trim();
          // Skip if it looks like a URL or special syntax
          if (name.contains('://') ||
              name.startsWith('@') ||
              name.startsWith('`') ||
              name.startsWith('\$')) {
            return TextSpan(text: m.group(0));
          }
          return TextSpan(
            text: '[$name]',
            style: TextStyle(
              color: theme.colorScheme.primary,
              decoration: TextDecoration.underline,
            ),
            recognizer: TapGestureRecognizer()
              ..onTap = () => onWikiLinkTap?.call(name, null),
          );
        },
      ),
    ];

    // Simple parsing: find matches and build spans
    while (remaining.isNotEmpty) {
      // Find the earliest match
      Match? earliestMatch;
      late TextSpan Function(Match) matchBuilder;
      int earliestIndex = remaining.length;

      for (final pattern in patterns) {
        final match = pattern.regex.firstMatch(remaining);
        if (match != null && match.start < earliestIndex) {
          earliestMatch = match;
          earliestIndex = match.start;
          matchBuilder = pattern.builder;
        }
      }

      if (earliestMatch != null) {
        // Add text before match
        if (earliestIndex > 0) {
          spans.add(TextSpan(text: remaining.substring(0, earliestIndex)));
        }
        // Add matched span
        spans.add(matchBuilder(earliestMatch));
        // Continue with remaining text
        remaining = remaining.substring(earliestMatch.end);
      } else {
        // No more matches, add remaining text
        spans.add(TextSpan(text: remaining));
        break;
      }
    }

    return spans;
  }
}
