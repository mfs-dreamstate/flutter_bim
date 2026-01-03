import 'package:flutter/material.dart';
import '../core/bridge/api.dart' as rust;

/// Measurement tools widget
/// Provides UI for distance, area, and volume measurements
class MeasurementTools extends StatefulWidget {
  final Function(String measurementType)? onMeasurementStarted;
  final Function()? onMeasurementCleared;

  const MeasurementTools({
    super.key,
    this.onMeasurementStarted,
    this.onMeasurementCleared,
  });

  @override
  State<MeasurementTools> createState() => _MeasurementToolsState();
}

class _MeasurementToolsState extends State<MeasurementTools> {
  String? _activeMeasurement;
  rust.MeasurementResult? _result;
  int _pointCount = 0;

  void _startMeasurement(String type) {
    try {
      rust.startMeasurement(measurementType: type);
      setState(() {
        _activeMeasurement = type;
        _result = null;
        _pointCount = 0;
      });
      widget.onMeasurementStarted?.call(type);
    } catch (e) {
      debugPrint('[MEASUREMENT] Start failed: $e');
    }
  }

  void _updateMeasurement() {
    try {
      final count = rust.getMeasurementPointCount();
      setState(() {
        _pointCount = count;
      });

      // Try to get result if we have enough points
      try {
        final result = rust.getMeasurementResult();
        setState(() {
          _result = result;
        });
      } catch (e) {
        // Not enough points yet, ignore
      }
    } catch (e) {
      debugPrint('[MEASUREMENT] Update failed: $e');
    }
  }

  void _clearMeasurement() {
    try {
      rust.clearMeasurement();
      setState(() {
        _activeMeasurement = null;
        _result = null;
        _pointCount = 0;
      });
      widget.onMeasurementCleared?.call();
    } catch (e) {
      debugPrint('[MEASUREMENT] Clear failed: $e');
    }
  }

  String _getMeasurementIcon(String type) {
    switch (type) {
      case 'distance':
        return '📏';
      case 'area':
        return '📐';
      case 'volume':
        return '📦';
      default:
        return '📊';
    }
  }

  String _getMeasurementLabel(String type) {
    switch (type) {
      case 'distance':
        return 'Distance';
      case 'area':
        return 'Area';
      case 'volume':
        return 'Volume';
      default:
        return type;
    }
  }

  int _getMinPoints(String type) {
    switch (type) {
      case 'distance':
        return 2;
      case 'area':
        return 3;
      case 'volume':
        return 4;
      default:
        return 2;
    }
  }

  String _formatValue(double value, String unit) {
    if (value < 0.01) {
      return '${(value * 1000).toStringAsFixed(2)} ${unit == "m" ? "mm" : unit}';
    } else if (value < 1.0) {
      return '${(value * 100).toStringAsFixed(2)} ${unit == "m" ? "cm" : unit}';
    } else {
      return '${value.toStringAsFixed(2)} $unit';
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(16)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Header
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                'Measurement Tools',
                style: theme.textTheme.titleLarge?.copyWith(
                  fontWeight: FontWeight.bold,
                ),
              ),
              IconButton(
                icon: const Icon(Icons.close),
                onPressed: () => Navigator.pop(context),
              ),
            ],
          ),
          const SizedBox(height: 16),

          // Measurement type buttons
          if (_activeMeasurement == null) ...[
            Text(
              'Select measurement type:',
              style: theme.textTheme.bodyMedium?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: _MeasurementButton(
                    label: 'Distance',
                    icon: '📏',
                    onPressed: () => _startMeasurement('distance'),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: _MeasurementButton(
                    label: 'Area',
                    icon: '📐',
                    onPressed: () => _startMeasurement('area'),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: _MeasurementButton(
                    label: 'Volume',
                    icon: '📦',
                    onPressed: () => _startMeasurement('volume'),
                  ),
                ),
              ],
            ),
          ],

          // Active measurement
          if (_activeMeasurement != null) ...[
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: colorScheme.primaryContainer,
                borderRadius: BorderRadius.circular(12),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Text(
                        _getMeasurementIcon(_activeMeasurement!),
                        style: const TextStyle(fontSize: 24),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          _getMeasurementLabel(_activeMeasurement!),
                          style: theme.textTheme.titleMedium?.copyWith(
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                      ),
                      IconButton(
                        icon: const Icon(Icons.close),
                        onPressed: _clearMeasurement,
                        tooltip: 'Cancel measurement',
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'Points: $_pointCount / ${_getMinPoints(_activeMeasurement!)} minimum',
                    style: theme.textTheme.bodyMedium,
                  ),
                  if (_result != null) ...[
                    const SizedBox(height: 12),
                    const Divider(),
                    const SizedBox(height: 12),
                    Row(
                      children: [
                        const Icon(Icons.straighten, size: 20),
                        const SizedBox(width: 8),
                        Text(
                          'Result: ',
                          style: theme.textTheme.bodyMedium?.copyWith(
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                        Text(
                          _formatValue(_result!.value, _result!.unit),
                          style: theme.textTheme.titleMedium?.copyWith(
                            color: colorScheme.primary,
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                      ],
                    ),
                  ],
                ],
              ),
            ),
            const SizedBox(height: 16),
            Text(
              'Tap on the model to add measurement points',
              style: theme.textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
                fontStyle: FontStyle.italic,
              ),
              textAlign: TextAlign.center,
            ),
          ],

          const SizedBox(height: 8),
        ],
      ),
    );
  }

  /// Called when a point is added from outside (e.g., tap on 3D model)
  void addPoint(double x, double y, double z) {
    if (_activeMeasurement == null) return;

    try {
      rust.addMeasurementPoint(x: x.toDouble(), y: y.toDouble(), z: z.toDouble());
      _updateMeasurement();
    } catch (e) {
      debugPrint('[MEASUREMENT] Add point failed: $e');
    }
  }
}

class _MeasurementButton extends StatelessWidget {
  final String label;
  final String icon;
  final VoidCallback onPressed;

  const _MeasurementButton({
    required this.label,
    required this.icon,
    required this.onPressed,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return ElevatedButton(
      onPressed: onPressed,
      style: ElevatedButton.styleFrom(
        backgroundColor: colorScheme.primaryContainer,
        foregroundColor: colorScheme.onPrimaryContainer,
        padding: const EdgeInsets.symmetric(vertical: 16),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(icon, style: const TextStyle(fontSize: 32)),
          const SizedBox(height: 4),
          Text(label),
        ],
      ),
    );
  }
}

/// Show measurement tools bottom sheet
void showMeasurementTools(
  BuildContext context, {
  Function(String measurementType)? onMeasurementStarted,
  Function()? onMeasurementCleared,
}) {
  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    backgroundColor: Colors.transparent,
    builder: (context) => MeasurementTools(
      onMeasurementStarted: onMeasurementStarted,
      onMeasurementCleared: onMeasurementCleared,
    ),
  );
}
