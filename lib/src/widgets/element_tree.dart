import 'package:flutter/material.dart';
import '../core/bridge/api.dart' as rust;
import '../core/bridge/bim/model.dart';
import 'properties_panel.dart';

/// Element Tree View showing hierarchical list of model elements
class ElementTreeDrawer extends StatefulWidget {
  final Function(ElementInfo)? onElementSelected;
  final int? selectedElementId;

  const ElementTreeDrawer({
    super.key,
    this.onElementSelected,
    this.selectedElementId,
  });

  @override
  State<ElementTreeDrawer> createState() => _ElementTreeDrawerState();
}

class _ElementTreeDrawerState extends State<ElementTreeDrawer> {
  Map<String, List<ElementInfo>> _elementsByType = {};
  Map<String, bool> _expandedTypes = {};
  String _searchQuery = '';
  bool _isLoading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _loadElements();
  }

  Future<void> _loadElements() async {
    setState(() {
      _isLoading = true;
      _error = null;
    });

    try {
      final elements = rust.getAllElementsFromAllModels();

      // Group elements by type
      final grouped = <String, List<ElementInfo>>{};
      for (final element in elements) {
        final type = element.elementType;
        grouped.putIfAbsent(type, () => []);
        grouped[type]!.add(element);
      }

      // Sort elements within each type by name
      for (final list in grouped.values) {
        list.sort((a, b) => a.name.compareTo(b.name));
      }

      setState(() {
        _elementsByType = grouped;
        _isLoading = false;
        // Expand first type by default if any
        if (grouped.isNotEmpty && _expandedTypes.isEmpty) {
          _expandedTypes[grouped.keys.first] = true;
        }
      });
    } catch (e) {
      setState(() {
        _error = e.toString();
        _isLoading = false;
      });
    }
  }

  List<ElementInfo> _getFilteredElements(String type) {
    final elements = _elementsByType[type] ?? [];
    if (_searchQuery.isEmpty) return elements;

    final query = _searchQuery.toLowerCase();
    return elements.where((e) {
      return e.name.toLowerCase().contains(query) ||
          e.globalId.toLowerCase().contains(query) ||
          e.id.toString().contains(query);
    }).toList();
  }

  int _getTotalFilteredCount() {
    if (_searchQuery.isEmpty) {
      return _elementsByType.values.fold(0, (sum, list) => sum + list.length);
    }
    return _elementsByType.keys
        .map((type) => _getFilteredElements(type).length)
        .fold(0, (sum, count) => sum + count);
  }

  void _selectElement(ElementInfo element) {
    widget.onElementSelected?.call(element);

    // Update renderer selection
    try {
      rust.setSelectedElement(elementId: element.id);
      rust.reloadAllModelsMesh();
    } catch (e) {
      debugPrint('[ELEMENT_TREE] Error selecting element: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final totalCount = _getTotalFilteredCount();

    return Drawer(
      child: Column(
        children: [
          // Header
          Container(
            width: double.infinity,
            padding: EdgeInsets.only(
              top: MediaQuery.of(context).padding.top + 16,
              left: 16,
              right: 16,
              bottom: 16,
            ),
            decoration: BoxDecoration(
              color: colorScheme.primaryContainer,
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Icon(Icons.account_tree, color: colorScheme.onPrimaryContainer),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        'Element Tree',
                        style: Theme.of(context).textTheme.titleLarge?.copyWith(
                          color: colorScheme.onPrimaryContainer,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                    IconButton(
                      icon: Icon(Icons.refresh, color: colorScheme.onPrimaryContainer),
                      onPressed: _loadElements,
                      tooltip: 'Refresh',
                    ),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  '$totalCount element${totalCount == 1 ? '' : 's'}',
                  style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    color: colorScheme.onPrimaryContainer.withValues(alpha: 0.8),
                  ),
                ),
              ],
            ),
          ),

          // Search bar
          Padding(
            padding: const EdgeInsets.all(12),
            child: TextField(
              decoration: InputDecoration(
                hintText: 'Search elements...',
                prefixIcon: const Icon(Icons.search),
                suffixIcon: _searchQuery.isNotEmpty
                    ? IconButton(
                        icon: const Icon(Icons.clear),
                        onPressed: () {
                          setState(() {
                            _searchQuery = '';
                          });
                        },
                      )
                    : null,
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(12),
                ),
                contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
              ),
              onChanged: (value) {
                setState(() {
                  _searchQuery = value;
                });
              },
            ),
          ),

          const Divider(height: 1),

          // Content
          Expanded(
            child: _buildContent(context),
          ),
        ],
      ),
    );
  }

  Widget _buildContent(BuildContext context) {
    if (_isLoading) {
      return const Center(
        child: CircularProgressIndicator(),
      );
    }

    if (_error != null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                Icons.error_outline,
                size: 48,
                color: Theme.of(context).colorScheme.error,
              ),
              const SizedBox(height: 16),
              Text(
                'Failed to load elements',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 8),
              Text(
                _error!,
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.error,
                ),
              ),
              const SizedBox(height: 16),
              ElevatedButton.icon(
                onPressed: _loadElements,
                icon: const Icon(Icons.refresh),
                label: const Text('Retry'),
              ),
            ],
          ),
        ),
      );
    }

    if (_elementsByType.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.folder_open,
              size: 64,
              color: Theme.of(context).colorScheme.outline,
            ),
            const SizedBox(height: 16),
            Text(
              'No elements found',
              style: Theme.of(context).textTheme.titleMedium?.copyWith(
                color: Theme.of(context).colorScheme.outline,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              'Load a model to see elements',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.outline,
              ),
            ),
          ],
        ),
      );
    }

    // Sort types alphabetically
    final sortedTypes = _elementsByType.keys.toList()..sort();

    return ListView.builder(
      padding: const EdgeInsets.symmetric(vertical: 8),
      itemCount: sortedTypes.length,
      itemBuilder: (context, index) {
        final type = sortedTypes[index];
        final elements = _getFilteredElements(type);

        if (elements.isEmpty && _searchQuery.isNotEmpty) {
          return const SizedBox.shrink();
        }

        return _ElementTypeSection(
          type: type,
          elements: elements,
          isExpanded: _expandedTypes[type] ?? false,
          selectedElementId: widget.selectedElementId,
          onToggleExpand: () {
            setState(() {
              _expandedTypes[type] = !(_expandedTypes[type] ?? false);
            });
          },
          onElementTap: _selectElement,
        );
      },
    );
  }
}

class _ElementTypeSection extends StatelessWidget {
  final String type;
  final List<ElementInfo> elements;
  final bool isExpanded;
  final int? selectedElementId;
  final VoidCallback onToggleExpand;
  final Function(ElementInfo) onElementTap;

  const _ElementTypeSection({
    required this.type,
    required this.elements,
    required this.isExpanded,
    required this.selectedElementId,
    required this.onToggleExpand,
    required this.onElementTap,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final typeColor = _getTypeColor(type);

    return Column(
      children: [
        // Type header
        InkWell(
          onTap: onToggleExpand,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
            child: Row(
              children: [
                Icon(
                  isExpanded ? Icons.expand_more : Icons.chevron_right,
                  color: colorScheme.onSurfaceVariant,
                ),
                const SizedBox(width: 8),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                  decoration: BoxDecoration(
                    color: typeColor.withValues(alpha: 0.15),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(_getTypeIcon(type), size: 16, color: typeColor),
                      const SizedBox(width: 4),
                      Text(
                        type,
                        style: TextStyle(
                          fontWeight: FontWeight.bold,
                          color: typeColor,
                        ),
                      ),
                    ],
                  ),
                ),
                const Spacer(),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                  decoration: BoxDecoration(
                    color: colorScheme.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Text(
                    '${elements.length}',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),

        // Element list
        if (isExpanded)
          ...elements.map((element) => _ElementListTile(
            element: element,
            isSelected: element.id == selectedElementId,
            onTap: () => onElementTap(element),
          )),

        if (isExpanded) const Divider(height: 1),
      ],
    );
  }

  IconData _getTypeIcon(String type) {
    switch (type.toUpperCase()) {
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

  Color _getTypeColor(String type) {
    switch (type.toUpperCase()) {
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

class _ElementListTile extends StatelessWidget {
  final ElementInfo element;
  final bool isSelected;
  final VoidCallback onTap;

  const _ElementListTile({
    required this.element,
    required this.isSelected,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return InkWell(
      onTap: onTap,
      onLongPress: () {
        showPropertiesPanel(
          context,
          element: element,
        );
      },
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        margin: const EdgeInsets.only(left: 40),
        decoration: BoxDecoration(
          color: isSelected ? colorScheme.primaryContainer : null,
          border: Border(
            left: BorderSide(
              color: isSelected ? colorScheme.primary : Colors.transparent,
              width: 3,
            ),
          ),
        ),
        child: Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    element.name.isEmpty ? 'Unnamed' : element.name,
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  Text(
                    '#${element.id}',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                      fontFamily: 'monospace',
                    ),
                  ),
                ],
              ),
            ),
            Icon(
              Icons.info_outline,
              size: 18,
              color: colorScheme.onSurfaceVariant,
            ),
          ],
        ),
      ),
    );
  }
}
