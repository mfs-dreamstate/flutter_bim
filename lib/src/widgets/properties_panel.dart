import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../core/bridge/bim/model.dart';
import '../core/bridge/bim/geometry.dart';
import '../core/bridge/lib.dart';
import '../core/bridge/api/properties.dart' as rust_properties;
import '../core/constants/bim_element_types.dart';

/// Properties panel that displays detailed element information
class PropertiesPanel extends StatefulWidget {
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
  State<PropertiesPanel> createState() => _PropertiesPanelState();
}

class _PropertiesPanelState extends State<PropertiesPanel> {
  List<rust_properties.PropertySetInfo> _propertySets = [];
  String? _storeyName;
  rust_properties.MaterialData? _material;
  rust_properties.TypeObjectData? _typeObject;
  final Set<String> _expandedSets = {};

  @override
  void initState() {
    super.initState();
    _loadProperties();
  }

  @override
  void didUpdateWidget(PropertiesPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.element.id != widget.element.id) {
      _loadProperties();
    }
  }

  void _loadProperties() {
    try {
      _propertySets = rust_properties.getElementPropertiesAllModels(
        elementId: widget.element.id,
      );
      _storeyName = rust_properties.getElementStorey(
        elementId: widget.element.id,
      );
      _material = rust_properties.getElementMaterial(
        elementId: widget.element.id,
      );
      _typeObject = rust_properties.getElementTypeInfo(
        elementId: widget.element.id,
      );
    } catch (e) {
      debugPrint('[PROPERTIES] Error loading properties: $e');
    }
    if (mounted) setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    // Calculate dimensions from bounding box
    final width = (widget.element.bounds.max[0] - widget.element.bounds.min[0]).abs();
    final height = (widget.element.bounds.max[2] - widget.element.bounds.min[2]).abs(); // Z is up
    final depth = (widget.element.bounds.max[1] - widget.element.bounds.min[1]).abs();

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
                    color: BimElementVisuals.colorFor(widget.element.elementType).withValues(alpha: 0.15),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(
                      color: BimElementVisuals.colorFor(widget.element.elementType).withValues(alpha: 0.3),
                    ),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        BimElementVisuals.iconFor(widget.element.elementType),
                        size: 18,
                        color: BimElementVisuals.colorFor(widget.element.elementType),
                      ),
                      const SizedBox(width: 6),
                      Text(
                        widget.element.elementType,
                        style: TextStyle(
                          fontWeight: FontWeight.bold,
                          color: BimElementVisuals.colorFor(widget.element.elementType),
                        ),
                      ),
                    ],
                  ),
                ),
                const Spacer(),
                if (widget.onFocusElement != null)
                  IconButton(
                    icon: const Icon(Icons.center_focus_strong),
                    onPressed: widget.onFocusElement,
                    tooltip: 'Focus on element',
                  ),
                if (widget.onClose != null)
                  IconButton(
                    icon: const Icon(Icons.close),
                    onPressed: widget.onClose,
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
                widget.element.name.isEmpty ? 'Unnamed Element' : widget.element.name,
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
                    value: widget.element.globalId,
                    copyable: true,
                  ),
                  _PropertyRow(
                    label: 'Internal ID',
                    value: '#${widget.element.id}',
                  ),
                  if (_storeyName != null)
                    _PropertyRow(
                      label: 'Storey',
                      value: _storeyName!,
                    ),
                  if (_typeObject != null) ...[
                    _PropertyRow(
                      label: 'Type',
                      value: _typeObject!.typeName,
                    ),
                    _PropertyRow(
                      label: 'IFC Type',
                      value: _typeObject!.ifcType,
                    ),
                  ],

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
                    value: _formatPoint(widget.element.bounds.min),
                  ),
                  _PropertyRow(
                    label: 'Max Point',
                    value: _formatPoint(widget.element.bounds.max),
                  ),
                  _PropertyRow(
                    label: 'Center',
                    value: _formatCenter(widget.element.bounds),
                  ),

                  const SizedBox(height: 20),

                  // Geometry section
                  _SectionHeader(title: 'Geometry', icon: Icons.view_in_ar),
                  const SizedBox(height: 8),
                  _PropertyRow(
                    label: 'Triangle Count',
                    value: '${widget.element.triangleCount}',
                  ),
                  _PropertyRow(
                    label: 'Triangle Start',
                    value: '${widget.element.triangleStart}',
                  ),

                  // Material section
                  if (_material != null) ...[
                    const SizedBox(height: 20),
                    _SectionHeader(title: 'Material', icon: Icons.texture),
                    const SizedBox(height: 8),
                    _PropertyRow(
                      label: 'Name',
                      value: _material!.name,
                    ),
                    if (_material!.category != null)
                      _PropertyRow(
                        label: 'Category',
                        value: _material!.category!,
                      ),
                    if (_material!.layers.isNotEmpty)
                      ..._material!.layers.asMap().entries.map((entry) {
                        final i = entry.key;
                        final layer = entry.value;
                        final thickness = layer.thickness != null
                            ? ' (${_formatDimension(layer.thickness!)})'
                            : '';
                        return _PropertyRow(
                          label: 'Layer ${i + 1}',
                          value: '${layer.materialName}$thickness',
                        );
                      }),
                  ],

                  // IFC Property Sets
                  if (_propertySets.isNotEmpty) ...[
                    const SizedBox(height: 20),
                    const Divider(height: 1),
                    const SizedBox(height: 20),
                    ..._propertySets.map((pset) => _PropertySetSection(
                      pset: pset,
                      isExpanded: _expandedSets.contains(pset.name),
                      onToggle: () {
                        setState(() {
                          if (_expandedSets.contains(pset.name)) {
                            _expandedSets.remove(pset.name);
                          } else {
                            _expandedSets.add(pset.name);
                          }
                        });
                      },
                    )),
                  ],

                  // Type Property Sets
                  if (_typeObject != null && _typeObject!.propertySets.isNotEmpty) ...[
                    const SizedBox(height: 20),
                    const Divider(height: 1),
                    const SizedBox(height: 12),
                    Text(
                      'Type Properties (${_typeObject!.typeName})',
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                    const SizedBox(height: 8),
                    ..._typeObject!.propertySets.map((pset) => _PropertySetSection(
                      pset: pset,
                      isExpanded: _expandedSets.contains('type:${pset.name}'),
                      onToggle: () {
                        setState(() {
                          final key = 'type:${pset.name}';
                          if (_expandedSets.contains(key)) {
                            _expandedSets.remove(key);
                          } else {
                            _expandedSets.add(key);
                          }
                        });
                      },
                    )),
                  ],

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

}

/// Expandable section for an IFC property set
class _PropertySetSection extends StatelessWidget {
  final rust_properties.PropertySetInfo pset;
  final bool isExpanded;
  final VoidCallback onToggle;

  const _PropertySetSection({
    required this.pset,
    required this.isExpanded,
    required this.onToggle,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        InkWell(
          onTap: onToggle,
          borderRadius: BorderRadius.circular(8),
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Row(
              children: [
                Icon(
                  isExpanded ? Icons.expand_more : Icons.chevron_right,
                  size: 20,
                  color: colorScheme.primary,
                ),
                const SizedBox(width: 4),
                Icon(
                  Icons.list_alt,
                  size: 18,
                  color: colorScheme.primary,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    pset.name,
                    style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.bold,
                      color: colorScheme.primary,
                    ),
                  ),
                ),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                  decoration: BoxDecoration(
                    color: colorScheme.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Text(
                    '${pset.properties.length}',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
        if (isExpanded)
          Padding(
            padding: const EdgeInsets.only(left: 28, bottom: 8),
            child: Column(
              children: pset.properties
                  .map((prop) => _PropertyRow(
                        label: prop.name,
                        value: prop.value,
                      ))
                  .toList(),
            ),
          ),
      ],
    );
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
