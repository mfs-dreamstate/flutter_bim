import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/constants/lighting_defaults.dart';
import '../../core/providers/lighting_state.dart';

/// Bottom sheet for lighting settings.
class LightingSettingsSheet extends ConsumerWidget {
  const LightingSettingsSheet({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lighting = ref.watch(lightingStateProvider);
    final notifier = ref.read(lightingStateProvider.notifier);

    return DraggableScrollableSheet(
      initialChildSize: 0.6,
      minChildSize: 0.3,
      maxChildSize: 0.9,
      expand: false,
      builder: (context, scrollController) {
        return Container(
          padding: const EdgeInsets.all(16),
          child: ListView(
            controller: scrollController,
            children: [
              // Header
              Row(
                children: [
                  Icon(Icons.light_mode,
                      color: Theme.of(context).colorScheme.primary),
                  const SizedBox(width: 8),
                  Text(
                    'Lighting Settings',
                    style: Theme.of(context).textTheme.titleLarge?.copyWith(
                          fontWeight: FontWeight.bold,
                        ),
                  ),
                ],
              ),
              const SizedBox(height: 24),

              // Light Direction
              Text('Light Direction',
                  style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              _buildSlider(
                context,
                'X (Left/Right)',
                lighting.lightX,
                -1.0,
                1.0,
                (v) => notifier.setDirection(x: v),
              ),
              _buildSlider(
                context,
                'Y (Down/Up)',
                lighting.lightY,
                -1.0,
                1.0,
                (v) => notifier.setDirection(y: v),
              ),
              _buildSlider(
                context,
                'Z (Back/Front)',
                lighting.lightZ,
                -1.0,
                1.0,
                (v) => notifier.setDirection(z: v),
              ),
              const SizedBox(height: 16),

              // Light Intensity
              Text('Light Intensity',
                  style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              _buildSlider(
                context,
                'Intensity',
                lighting.intensity,
                0.0,
                3.0,
                (v) => notifier.setIntensity(v),
              ),
              const SizedBox(height: 16),

              // Light Color
              Text('Light Color',
                  style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              _buildColorPicker(
                context,
                lighting.lightColor,
                (color) => notifier.setLightColor(color),
              ),
              const SizedBox(height: 16),

              // Ambient Color
              Text('Ambient Light',
                  style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              _buildColorPicker(
                context,
                lighting.ambientColor,
                (color) => notifier.setAmbientColor(color),
              ),
              const SizedBox(height: 24),

              // Reset button
              OutlinedButton.icon(
                onPressed: () => notifier.resetToDefaults(),
                icon: const Icon(Icons.refresh),
                label: const Text('Reset to Defaults'),
              ),
            ],
          ),
        );
      },
    );
  }

  Widget _buildSlider(
    BuildContext context,
    String label,
    double value,
    double min,
    double max,
    ValueChanged<double> onChanged,
  ) {
    return Row(
      children: [
        SizedBox(
          width: 100,
          child:
              Text(label, style: Theme.of(context).textTheme.bodySmall),
        ),
        Expanded(
          child: Slider(
            value: value,
            min: min,
            max: max,
            onChanged: onChanged,
          ),
        ),
        SizedBox(
          width: 50,
          child: Text(
            value.toStringAsFixed(2),
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ),
      ],
    );
  }

  Widget _buildColorPicker(
    BuildContext context,
    Color currentColor,
    ValueChanged<Color> onColorChanged,
  ) {
    return Wrap(
      spacing: 8,
      runSpacing: 8,
      children: LightingDefaults.presetColors.map((color) {
        final isSelected = currentColor == color;
        return GestureDetector(
          onTap: () => onColorChanged(color),
          child: Container(
            width: 40,
            height: 40,
            decoration: BoxDecoration(
              color: color,
              shape: BoxShape.circle,
              border: Border.all(
                color: isSelected
                    ? Theme.of(context).colorScheme.primary
                    : Colors.grey.shade400,
                width: isSelected ? 3 : 1,
              ),
              boxShadow: [
                if (isSelected)
                  BoxShadow(
                    color: Theme.of(context)
                        .colorScheme
                        .primary
                        .withValues(alpha: 0.3),
                    blurRadius: 8,
                    spreadRadius: 2,
                  ),
              ],
            ),
          ),
        );
      }).toList(),
    );
  }
}

/// Show the lighting settings as a bottom sheet.
void showLightingSettings(BuildContext context) {
  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    builder: (context) => const LightingSettingsSheet(),
  );
}
