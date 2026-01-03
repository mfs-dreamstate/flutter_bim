import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../core/bridge/bim/model.dart';
import '../core/bridge/bim/geometry.dart';
import '../core/bridge/lib.dart';

/// Properties panel that displays detailed element information
class PropertiesPanel extends StatelessWidget {
  final ElementInfo element;
  final VoidCallback? onClose;
  final VoidCallback? onFocusElement;

  const PropertiesPanel({
    super.key,
    required this.element,
    this.onClose,
    this.onFocusElement,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    // Calculate dimensions from bounding box
    final width = (element.bounds.max[0] - element.bounds.min[0]).abs();
    final height = (element.bounds.max[2] - element.bounds.min[2]).abs(); // Z is up
    final depth = (element.bounds.max[1] - element.bounds.min[1]).abs();

    return Container(
      decoration: BoxDecoration(
        color: colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.2),
            blurRadius: 16,
            offset: const Offset(0, -4),
          ),
        ],
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          // Drag handle
          Container(
            margin: const EdgeInsets.only(top: 12),
            width: 40,
            height: 4,
            decoration: BoxDecoration(
              color: colorScheme.outline.withValues(alpha: 0.3),
              borderRadius: BorderRadius.circular(2),
            ),
          ),

          // Header
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 16, 12, 8),
            child: Row(
              children: [
                // Element type badge
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                  decoration: BoxDecoration(
                    color: _getElementColor(element.elementType).withValues(alpha: 0.15),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(
                      color: _getElementColor(element.elementType).withValues(alpha: 0.3),
                    ),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        _getElementIcon(element.elementType),
                        size: 18,
                        color: _getElementColor(element.elementType),
                      ),
                      const SizedBox(width: 6),
                      Text(
                        element.elementType,
                        style: TextStyle(
                          fontWeight: FontWeight.bold,
                          color: _getElementColor(element.elementType),
                        ),
                      ),
                    ],
                  ),
                ),
                const Spacer(),
                if (onFocusElement != null)
                  IconButton(
                    icon: const Icon(Icons.center_focus_strong),
                    onPressed: onFocusElement,
                    tooltip: 'Focus on element',
                  ),
                if (onClose != null)
                  IconButton(
                    icon: const Icon(Icons.close),
                    onPressed: onClose,
                    tooltip: 'Close',
                  ),
              ],
            ),
          ),

          // Element name
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 20),
            child: Align(
              alignment: Alignment.centerLeft,
              child: Text(
                element.name.isEmpty ? 'Unnamed Element' : element.name,
                style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                  fontWeight: FontWeight.bold,
                ),
              ),
            ),
          ),

          const SizedBox(height: 16),
          const Divider(height: 1),

          // Properties list
          Flexible(
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(20),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // Identity section
                  _SectionHeader(title: 'Identity', icon: Icons.fingerprint),
                  const SizedBox(height: 8),
                  _PropertyRow(
                    label: 'Global ID',
                    value: element.globalId,
                    copyable: true,
                  ),
                  _PropertyRow(
                    label: 'Internal ID',
                    value: '#${element.id}',
                  ),

                  const SizedBox(height: 20),

                  // Dimensions section
                  _SectionHeader(title: 'Dimensions', icon: Icons.straighten),
                  const SizedBox(height: 8),
                  _PropertyRow(
                    label: 'Width (X)',
                    value: _formatDimension(width),
                  ),
                  _PropertyRow(
                    label: 'Depth (Y)',
                    value: _formatDimension(depth),
                  ),
                  _PropertyRow(
                    label: 'Height (Z)',
                    value: _formatDimension(height),
                  ),

                  const SizedBox(height: 20),

                  // Location section
                  _SectionHeader(title: 'Location', icon: Icons.place),
                  const SizedBox(height: 8),
                  _PropertyRow(
                    label: 'Min Point',
                    value: _formatPoint(element.bounds.min),
                  ),
                  _PropertyRow(
                    label: 'Max Point',
                    value: _formatPoint(element.bounds.max),
                  ),
                  _PropertyRow(
                    label: 'Center',
                    value: _formatCenter(element.bounds),
                  ),

                  const SizedBox(height: 20),

                  // Geometry section
                  _SectionHeader(title: 'Geometry', icon: Icons.view_in_ar),
                  const SizedBox(height: 8),
                  _PropertyRow(
                    label: 'Triangle Count',
                    value: '${element.triangleCount}',
                  ),
                  _PropertyRow(
                    label: 'Triangle Start',
                    value: '${element.triangleStart}',
                  ),

                  const SizedBox(height: 16),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  String _formatDimension(double value) {
    if (value < 0.01) return '${(value * 1000).toStringAsFixed(1)} mm';
    if (value < 1) return '${(value * 100).toStringAsFixed(1)} cm';
    return '${value.toStringAsFixed(2)} m';
  }

  String _formatPoint(F32Array3 point) {
    return '(${point[0].toStringAsFixed(2)}, ${point[1].toStringAsFixed(2)}, ${point[2].toStringAsFixed(2)})';
  }

  String _formatCenter(BoundingBox bounds) {
    final cx = (bounds.min[0] + bounds.max[0]) / 2;
    final cy = (bounds.min[1] + bounds.max[1]) / 2;
    final cz = (bounds.min[2] + bounds.max[2]) / 2;
    return '(${cx.toStringAsFixed(2)}, ${cy.toStringAsFixed(2)}, ${cz.toStringAsFixed(2)})';
  }

  IconData _getElementIcon(String elementType) {
    switch (elementType.toUpperCase()) {
      case 'WALL':
        return Icons.view_agenda;
      case 'SLAB':
        return Icons.layers;
      case 'COLUMN':
        return Icons.view_column;
      case 'BEAM':
        return Icons.horizontal_rule;
      case 'DOOR':
        return Icons.door_front_door;
      case 'WINDOW':
        return Icons.window;
      case 'ROOF':
        return Icons.roofing;
      case 'STAIR':
        return Icons.stairs;
      case 'PIPE':
      case 'PIPESEGMENT':
        return Icons.plumbing;
      case 'DUCT':
      case 'DUCTSEGMENT':
        return Icons.air;
      case 'FLOWTERMINAL':
        return Icons.hvac;
      case 'CABLECARRIERSEGMENT':
        return Icons.electrical_services;
      case 'FOOTING':
        return Icons.foundation;
      case 'BUILDINGELEMENTPROXY':
        return Icons.category;
      default:
        return Icons.architecture;
    }
  }

  Color _getElementColor(String elementType) {
    switch (elementType.toUpperCase()) {
      case 'WALL':
        return Colors.brown;
      case 'SLAB':
        return Colors.grey;
      case 'COLUMN':
        return Colors.blueGrey;
      case 'BEAM':
        return Colors.indigo;
      case 'DOOR':
        return Colors.amber;
      case 'WINDOW':
        return Colors.lightBlue;
      case 'ROOF':
        return Colors.deepOrange;
      case 'STAIR':
        return Colors.purple;
      case 'PIPE':
      case 'PIPESEGMENT':
        return Colors.teal;
      case 'DUCT':
      case 'DUCTSEGMENT':
        return Colors.cyan;
      case 'FLOWTERMINAL':
        return Colors.green;
      case 'CABLECARRIERSEGMENT':
        return Colors.orange;
      case 'FOOTING':
        return Colors.blueGrey;
      default:
        return Colors.grey;
    }
  }
}

class _SectionHeader extends StatelessWidget {
  final String title;
  final IconData icon;

  const _SectionHeader({required this.title, required this.icon});

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Icon(
          icon,
          size: 18,
          color: Theme.of(context).colorScheme.primary,
        ),
        const SizedBox(width: 8),
        Text(
          title,
          style: Theme.of(context).textTheme.titleMedium?.copyWith(
            fontWeight: FontWeight.bold,
            color: Theme.of(context).colorScheme.primary,
          ),
        ),
      ],
    );
  }
}

class _PropertyRow extends StatelessWidget {
  final String label;
  final String value;
  final bool copyable;

  const _PropertyRow({
    required this.label,
    required this.value,
    this.copyable = false,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 100,
            child: Text(
              label,
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(
            child: Row(
              children: [
                Expanded(
                  child: SelectableText(
                    value,
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w500,
                      fontFamily: copyable ? 'monospace' : null,
                    ),
                  ),
                ),
                if (copyable)
                  IconButton(
                    icon: const Icon(Icons.copy, size: 16),
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(),
                    onPressed: () {
                      Clipboard.setData(ClipboardData(text: value));
                      ScaffoldMessenger.of(context).showSnackBar(
                        SnackBar(
                          content: Text('Copied: $value'),
                          duration: const Duration(seconds: 1),
                        ),
                      );
                    },
                    tooltip: 'Copy to clipboard',
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// Show the properties panel as a bottom sheet
void showPropertiesPanel(
  BuildContext context, {
  required ElementInfo element,
  VoidCallback? onClose,
  VoidCallback? onFocusElement,
}) {
  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    backgroundColor: Colors.transparent,
    builder: (context) => DraggableScrollableSheet(
      initialChildSize: 0.5,
      minChildSize: 0.3,
      maxChildSize: 0.85,
      expand: false,
      builder: (context, scrollController) => PropertiesPanel(
        element: element,
        onClose: () => Navigator.pop(context),
        onFocusElement: onFocusElement != null
            ? () {
                Navigator.pop(context);
                onFocusElement();
              }
            : null,
      ),
    ),
  );
}
