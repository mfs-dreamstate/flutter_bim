import 'package:flutter/material.dart';
import '../core/bridge/api.dart' as rust;
import '../core/bridge/api/properties.dart' as rust_properties;
import '../core/bridge/api/selection.dart' as rust_selection;
import '../core/bridge/bim/model.dart';
import '../core/constants/bim_element_types.dart';
import 'properties_panel.dart';

/// Grouping mode for the element tree
enum ElementGrouping { byType, byStorey }

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
  Map<String, List<ElementInfo>> _elementsByStorey = {};
  Map<String, bool> _expandedGroups = {};
  String _searchQuery = '';
  bool _isLoading = true;
  String? _error;
  ElementGrouping _grouping = ElementGrouping.byType;
  String? _isolatedStorey;

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

    // Yield one frame so the loading indicator actually renders
    // before the sync FFI call blocks the UI thread
    await Future.delayed(Duration.zero);

    try {
      final elements = rust.getAllElementsFromAllModels();

      // Group elements by type
      final groupedByType = <String, List<ElementInfo>>{};
      for (final element in elements) {
        final type = element.elementType;
        groupedByType.putIfAbsent(type, () => []);
        groupedByType[type]!.add(element);
      }

      // Sort elements within each type by name
      for (final list in groupedByType.values) {
        list.sort((a, b) => a.name.compareTo(b.name));
      }

      // Group elements by storey
      final groupedByStorey = <String, List<ElementInfo>>{};
      try {
        final storeyMap = rust_properties.getElementsByStorey();
        // Build element lookup by id
        final elementById = <int, ElementInfo>{};
        for (final element in elements) {
          elementById[element.id] = element;
        }
        for (final entry in storeyMap.entries) {
          final storeyName = entry.key;
          final elementIds = entry.value;
          final storeyElements = <ElementInfo>[];
          for (final id in elementIds) {
            if (elementById.containsKey(id)) {
              storeyElements.add(elementById[id]!);
            }
          }
          if (storeyElements.isNotEmpty) {
            storeyElements.sort((a, b) => a.name.compareTo(b.name));
            groupedByStorey[storeyName] = storeyElements;
          }
        }
      } catch (e) {
        debugPrint('[ELEMENT_TREE] Error loading storey data: $e');
      }

      setState(() {
        _elementsByType = groupedByType;
        _elementsByStorey = groupedByStorey;
        _isLoading = false;
        // Expand first group by default if any
        final currentGroups = _currentGroups;
        if (currentGroups.isNotEmpty && _expandedGroups.isEmpty) {
          _expandedGroups[currentGroups.keys.first] = true;
        }
      });
    } catch (e) {
      setState(() {
        _error = e.toString();
        _isLoading = false;
      });
    }
  }

  Map<String, List<ElementInfo>> get _currentGroups {
    return _grouping == ElementGrouping.byType ? _elementsByType : _elementsByStorey;
  }

  List<ElementInfo> _getFilteredElements(String groupKey) {
    final elements = _currentGroups[groupKey] ?? [];
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
      return _currentGroups.values.fold(0, (sum, list) => sum + list.length);
    }
    return _currentGroups.keys
        .map((key) => _getFilteredElements(key).length)
        .fold(0, (sum, count) => sum + count);
  }

  void _toggleStoreyIsolation(String storeyName) {
    try {
      if (_isolatedStorey == storeyName) {
        // Un-isolate
        rust_selection.showAllStoreys();
        rust.reloadAllModelsMesh();
        setState(() => _isolatedStorey = null);
      } else {
        // Isolate this storey
        rust_selection.isolateStorey(storeyName: storeyName);
        rust.reloadAllModelsMesh();
        setState(() => _isolatedStorey = storeyName);
      }
    } catch (e) {
      debugPrint('[ELEMENT_TREE] Error toggling storey isolation: $e');
    }
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

          // Grouping toggle + Search
          Padding(
            padding: const EdgeInsets.fromLTRB(12, 12, 12, 0),
            child: Row(
              children: [
                Expanded(
                  child: SegmentedButton<ElementGrouping>(
                    segments: const [
                      ButtonSegment(
                        value: ElementGrouping.byType,
                        label: Text('By Type'),
                        icon: Icon(Icons.category, size: 16),
                      ),
                      ButtonSegment(
                        value: ElementGrouping.byStorey,
                        label: Text('By Storey'),
                        icon: Icon(Icons.layers, size: 16),
                      ),
                    ],
                    selected: {_grouping},
                    onSelectionChanged: (selection) {
                      // Clear storey isolation when switching modes
                      if (_isolatedStorey != null) {
                        try {
                          rust_selection.showAllStoreys();
                          rust.reloadAllModelsMesh();
                        } catch (_) {}
                      }
                      setState(() {
                        _grouping = selection.first;
                        _expandedGroups.clear();
                        _isolatedStorey = null;
                        final groups = _currentGroups;
                        if (groups.isNotEmpty) {
                          _expandedGroups[groups.keys.first] = true;
                        }
                      });
                    },
                    style: const ButtonStyle(
                      visualDensity: VisualDensity.compact,
                    ),
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

          // Storey isolation banner
          if (_isolatedStorey != null)
            Container(
              width: double.infinity,
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              color: colorScheme.tertiaryContainer,
              child: Row(
                children: [
                  Icon(Icons.filter_alt, size: 16, color: colorScheme.onTertiaryContainer),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      'Showing: $_isolatedStorey',
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: colorScheme.onTertiaryContainer,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ),
                  TextButton(
                    onPressed: () {
                      try {
                        rust_selection.showAllStoreys();
                        rust.reloadAllModelsMesh();
                        setState(() => _isolatedStorey = null);
                      } catch (e) {
                        debugPrint('[ELEMENT_TREE] Error showing all storeys: $e');
                      }
                    },
                    style: TextButton.styleFrom(
                      padding: const EdgeInsets.symmetric(horizontal: 8),
                      minimumSize: Size.zero,
                      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                    ),
                    child: Text(
                      'Show All',
                      style: TextStyle(color: colorScheme.onTertiaryContainer),
                    ),
                  ),
                ],
              ),
            ),

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

    final groups = _currentGroups;
    if (groups.isEmpty) {
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
              _grouping == ElementGrouping.byStorey
                  ? 'No storey assignments found'
                  : 'Load a model to see elements',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.outline,
              ),
            ),
          ],
        ),
      );
    }

    // Sort groups: for storeys, sort by name; for types, alphabetically
    final sortedKeys = groups.keys.toList()..sort((a, b) {
      // Push "Unassigned" to the end
      if (a == 'Unassigned') return 1;
      if (b == 'Unassigned') return -1;
      return a.compareTo(b);
    });

    return ListView.builder(
      padding: const EdgeInsets.symmetric(vertical: 8),
      itemCount: sortedKeys.length,
      itemBuilder: (context, index) {
        final key = sortedKeys[index];
        final elements = _getFilteredElements(key);

        if (elements.isEmpty && _searchQuery.isNotEmpty) {
          return const SizedBox.shrink();
        }

        final isStoreyMode = _grouping == ElementGrouping.byStorey;
        return _ElementGroupSection(
          groupName: key,
          groupIcon: isStoreyMode
              ? (key == 'Unassigned' ? Icons.help_outline : Icons.layers)
              : null,
          elements: elements,
          isExpanded: _expandedGroups[key] ?? false,
          selectedElementId: widget.selectedElementId,
          showTypeInList: isStoreyMode,
          isIsolated: _isolatedStorey == key,
          onToggleIsolate: isStoreyMode && key != 'Unassigned'
              ? () => _toggleStoreyIsolation(key)
              : null,
          onToggleExpand: () {
            setState(() {
              _expandedGroups[key] = !(_expandedGroups[key] ?? false);
            });
          },
          onElementTap: _selectElement,
        );
      },
    );
  }
}

class _ElementGroupSection extends StatelessWidget {
  final String groupName;
  final IconData? groupIcon;
  final List<ElementInfo> elements;
  final bool isExpanded;
  final int? selectedElementId;
  final bool showTypeInList;
  final bool isIsolated;
  final VoidCallback? onToggleIsolate;
  final VoidCallback onToggleExpand;
  final Function(ElementInfo) onElementTap;

  const _ElementGroupSection({
    required this.groupName,
    this.groupIcon,
    required this.elements,
    required this.isExpanded,
    required this.selectedElementId,
    this.showTypeInList = false,
    this.isIsolated = false,
    this.onToggleIsolate,
    required this.onToggleExpand,
    required this.onElementTap,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final typeColor = groupIcon != null
        ? colorScheme.primary
        : BimElementVisuals.colorFor(groupName);

    return Column(
      children: [
        // Group header
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
                      Icon(
                        groupIcon ?? BimElementVisuals.iconFor(groupName),
                        size: 16,
                        color: typeColor,
                      ),
                      const SizedBox(width: 4),
                      Text(
                        groupName,
                        style: TextStyle(
                          fontWeight: FontWeight.bold,
                          color: typeColor,
                        ),
                      ),
                    ],
                  ),
                ),
                const Spacer(),
                if (onToggleIsolate != null)
                  SizedBox(
                    width: 28,
                    height: 28,
                    child: IconButton(
                      icon: Icon(
                        isIsolated ? Icons.visibility : Icons.visibility_outlined,
                        size: 16,
                        color: isIsolated ? colorScheme.primary : colorScheme.onSurfaceVariant,
                      ),
                      padding: EdgeInsets.zero,
                      onPressed: onToggleIsolate,
                      tooltip: isIsolated ? 'Show all storeys' : 'Isolate storey',
                    ),
                  ),
                const SizedBox(width: 4),
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
            showType: showTypeInList,
            onTap: () => onElementTap(element),
          )),

        if (isExpanded) const Divider(height: 1),
      ],
    );
  }

}

class _ElementListTile extends StatelessWidget {
  final ElementInfo element;
  final bool isSelected;
  final bool showType;
  final VoidCallback onTap;

  const _ElementListTile({
    required this.element,
    required this.isSelected,
    this.showType = false,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final displayName = element.name.isEmpty ? 'Unnamed' : element.name;

    return Semantics(
      label: 'Element: $displayName, Type: ${element.elementType}',
      button: true,
      selected: isSelected,
      child: InkWell(
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
                      displayName,
                      style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    Row(
                      children: [
                        Text(
                          '#${element.id}',
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                            fontFamily: 'monospace',
                          ),
                        ),
                        if (showType) ...[
                          const SizedBox(width: 8),
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                            decoration: BoxDecoration(
                              color: BimElementVisuals.colorFor(element.elementType).withValues(alpha: 0.1),
                              borderRadius: BorderRadius.circular(3),
                            ),
                            child: Text(
                              element.elementType,
                              style: Theme.of(context).textTheme.labelSmall?.copyWith(
                                color: BimElementVisuals.colorFor(element.elementType),
                              ),
                            ),
                          ),
                        ],
                      ],
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
      ),
    );
  }
}
