import 'package:flutter/material.dart';

import '../../../core/settings.dart';

/// Note text size, with a sample so the effect is visible before leaving the
/// screen. The slider is debounced: dragging it writes on release, not on every
/// frame.
class FontSizeSetting extends StatefulWidget {
  const FontSizeSetting({
    super.key,
    required this.scale,
    required this.onChanged,
  });

  final double scale;
  final void Function(double) onChanged;

  @override
  State<FontSizeSetting> createState() => _FontSizeSettingState();
}

class _FontSizeSettingState extends State<FontSizeSetting> {
  double? _dragging;

  double get _value => _dragging ?? widget.scale;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final body = theme.textTheme.bodyLarge!;
    final sample = body.copyWith(fontSize: (body.fontSize ?? 16) * _value);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text('Note text size', style: theme.textTheme.bodyMedium),
            const Spacer(),
            Text(
              '${(_value * 100).round()}%',
              style: theme.textTheme.labelMedium,
            ),
          ],
        ),
        Row(
          children: [
            const Icon(Icons.text_fields, size: 16),
            Expanded(
              child: Slider(
                value: _value,
                min: Settings.minFontScale,
                max: Settings.maxFontScale,
                // 10% steps: fine enough to tune, coarse enough to land on.
                divisions:
                    ((Settings.maxFontScale - Settings.minFontScale) * 10)
                        .round(),
                label: '${(_value * 100).round()}%',
                onChanged: (v) => setState(() => _dragging = v),
                onChangeEnd: (v) {
                  setState(() => _dragging = null);
                  widget.onChanged(v);
                },
              ),
            ),
            const Icon(Icons.text_fields, size: 24),
          ],
        ),
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: theme.colorScheme.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(8),
          ),
          child: Text(
            'A nested note line with [a link] and a task.',
            style: sample,
          ),
        ),
      ],
    );
  }
}
