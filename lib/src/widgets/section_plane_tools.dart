import 'package:flutter/material.dart';
import '../core/bridge/api.dart' as rust;

/// Section plane tools widget
/// Provides UI for creating and controlling section planes
class SectionPlaneTools extends StatefulWidget {
  final Function()? onPlaneChanged;

  const SectionPlaneTools({
    super.key,
    this.onPlaneChanged,
  });

  @override
  State<SectionPlaneTools> createState() => _SectionPlaneToolsState();
}

class _SectionPlaneToolsState extends State<SectionPlaneTools> {
  int _selectedAxis = 1; // 0=X, 1=Y, 2=Z
  double _position = 0.0;
  bool _planeActive = false;
  final double _minPosition = -50.0;
  final double _maxPosition = 50.0;

  @override
  void initState() {
    super.initState();
    _checkPlaneStatus();
  }

  void _checkPlaneStatus() {
    try {
      _planeActive = rust.isSectionPlaneActive();
      setState(() {});
    } catch (e) {
      debugPrint('[SECTION] Check status failed: $e');
    }
  }

  void _setPlane() {
    try {
      rust.setSectionPlaneFromAxis(axis: _selectedAxis, position: _position);
      setState(() {
        _planeActive = true;
      });
      widget.onPlaneChanged?.call();
    } catch (e) {
      debugPrint('[SECTION] Set plane failed: $e');
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Failed to set section plane: $e')),
        );
      }
    }
  }

  void _clearPlane() {
    try {
      rust.clearSectionPlane();
      setState(() {
        _planeActive = false;
      });
      widget.onPlaneChanged?.call();
    } catch (e) {
      debugPrint('[SECTION] Clear plane failed: $e');
    }
  }

  void _togglePlane(bool enabled) {
    try {
      rust.setSectionPlaneEnabled(enabled: enabled);
      setState(() {
        _planeActive = enabled;
      });
      widget.onPlaneChanged?.call();
    } catch (e) {
      debugPrint('[SECTION] Toggle plane failed: $e');
    }
  }

  String _getAxisName(int axis) {
    switch (axis) {
      case 0:
        return 'X';
      case 1:
        return 'Y';
      case 2:
        return 'Z';
      default:
        return '';
    }
  }

  Color _getAxisColor(int axis) {
    switch (axis) {
      case 0:
        return Colors.red;
      case 1:
        return Colors.green;
      case 2:
        return Colors.blue;
      default:
        return Colors.grey;
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
                'Section Plane',
                style: theme.textTheme.titleLarge?.copyWith(
                  fontWeight: FontWeight.bold,
                ),
              ),
              Row(
                children: [
                  if (_planeActive)
                    Switch(
                      value: _planeActive,
                      onChanged: _togglePlane,
                    ),
                  IconButton(
                    icon: const Icon(Icons.close),
                    onPressed: () => Navigator.pop(context),
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 16),

          // Axis selection
          Text(
            'Cutting Axis:',
            style: theme.textTheme.titleSmall?.copyWith(
              fontWeight: FontWeight.bold,
            ),
          ),
          const SizedBox(height: 8),
          Row(
            children: [
              for (int i = 0; i < 3; i++) ...[
                Expanded(
                  child: _AxisButton(
                    axis: i,
                    selected: _selectedAxis == i,
                    onTap: () {
                      setState(() {
                        _selectedAxis = i;
                      });
                    },
                  ),
                ),
                if (i < 2) const SizedBox(width: 8),
              ],
            ],
          ),

          const SizedBox(height: 24),

          // Position slider
          Text(
            'Position along ${_getAxisName(_selectedAxis)}-axis: ${_position.toStringAsFixed(1)}m',
            style: theme.textTheme.titleSmall?.copyWith(
              fontWeight: FontWeight.bold,
            ),
          ),
          const SizedBox(height: 8),
          Slider(
            value: _position,
            min: _minPosition,
            max: _maxPosition,
            divisions: 100,
            label: '${_position.toStringAsFixed(1)}m',
            onChanged: (value) {
              setState(() {
                _position = value;
              });
              if (_planeActive) {
                _setPlane(); // Update in real-time if active
              }
            },
          ),

          const SizedBox(height: 16),

          // Action buttons
          Row(
            children: [
              Expanded(
                child: FilledButton.icon(
                  onPressed: _setPlane,
                  icon: const Icon(Icons.cut),
                  label: const Text('Apply Section'),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: OutlinedButton.icon(
                  onPressed: _planeActive ? _clearPlane : null,
                  icon: const Icon(Icons.clear),
                  label: const Text('Clear'),
                ),
              ),
            ],
          ),

          const SizedBox(height: 8),

          // Info text
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: colorScheme.primaryContainer.withValues(alpha: 0.5),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Row(
              children: [
                Icon(
                  Icons.info_outline,
                  size: 16,
                  color: colorScheme.onPrimaryContainer,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Section planes cut through the model to reveal interior structures',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: colorScheme.onPrimaryContainer,
                    ),
                  ),
                ),
              ],
            ),
          ),

          const SizedBox(height: 8),
        ],
      ),
    );
  }
}

class _AxisButton extends StatelessWidget {
  final int axis;
  final bool selected;
  final VoidCallback onTap;

  const _AxisButton({
    required this.axis,
    required this.selected,
    required this.onTap,
  });

  String get _label {
    switch (axis) {
      case 0:
        return 'X';
      case 1:
        return 'Y';
      case 2:
        return 'Z';
      default:
        return '';
    }
  }

  Color get _color {
    switch (axis) {
      case 0:
        return Colors.red;
      case 1:
        return Colors.green;
      case 2:
        return Colors.blue;
      default:
        return Colors.grey;
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Material(
      color: selected ? _color : theme.colorScheme.surfaceContainerHighest,
      borderRadius: BorderRadius.circular(8),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(8),
        child: Container(
          padding: const EdgeInsets.symmetric(vertical: 16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                _label,
                style: theme.textTheme.titleLarge?.copyWith(
                  color: selected ? Colors.white : _color,
                  fontWeight: FontWeight.bold,
                ),
              ),
              const SizedBox(height: 4),
              Text(
                selected ? 'Selected' : 'Axis',
                style: theme.textTheme.bodySmall?.copyWith(
                  color: selected
                      ? Colors.white.withValues(alpha: 0.8)
                      : theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Show section plane tools bottom sheet
void showSectionPlaneTools(
  BuildContext context, {
  Function()? onPlaneChanged,
}) {
  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    backgroundColor: Colors.transparent,
    builder: (context) => SectionPlaneTools(
      onPlaneChanged: onPlaneChanged,
    ),
  );
}
