import 'package:flutter/material.dart';
import '../core/bridge/api/selection.dart' as selection;
import '../core/bridge/api.dart' as rust;

/// Selection actions panel — hide/isolate, selection sets, search by property.
class SelectionActionsPanel extends StatefulWidget {
  const SelectionActionsPanel({super.key});

  @override
  State<SelectionActionsPanel> createState() => _SelectionActionsPanelState();
}

class _SelectionActionsPanelState extends State<SelectionActionsPanel> {
  int _selectionCount = 0;
  int _hiddenCount = 0;
  List<String> _selectionSets = [];
  List<String> _smartGroups = [];

  // Search by property
  final _propNameController = TextEditingController();
  final _propValueController = TextEditingController();

  // Smart group creation
  final _smartGroupNameController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  @override
  void dispose() {
    _propNameController.dispose();
    _propValueController.dispose();
    _smartGroupNameController.dispose();
    super.dispose();
  }

  void _refresh() {
    try {
      setState(() {
        _selectionCount = selection.getSelectionCount().toInt();
        _hiddenCount = selection.getHiddenElementCount().toInt();
        _selectionSets = selection.getSelectionSetNames();
        _smartGroups = selection.getSmartGroupNames();
      });
    } catch (e) {
      debugPrint('[SELECTION] refresh failed: $e');
    }
  }

  void _hideSelected() {
    try {
      selection.hideSelected();
      rust.reloadAllModelsMesh();
      _refresh();
    } catch (e) {
      debugPrint('[SELECTION] hideSelected failed: $e');
    }
  }

  void _isolateSelected() {
    try {
      selection.isolateSelected();
      rust.reloadAllModelsMesh();
      _refresh();
    } catch (e) {
      debugPrint('[SELECTION] isolateSelected failed: $e');
    }
  }

  void _showAll() {
    try {
      selection.showAllElements();
      rust.reloadAllModelsMesh();
      _refresh();
    } catch (e) {
      debugPrint('[SELECTION] showAll failed: $e');
    }
  }

  void _clearSelection() {
    try {
      selection.clearSelection();
      rust.reloadAllModelsMesh();
      _refresh();
    } catch (e) {
      debugPrint('[SELECTION] clearSelection failed: $e');
    }
  }

  void _searchByProperty() {
    final name = _propNameController.text.trim();
    final value = _propValueController.text.trim();
    if (name.isEmpty || value.isEmpty) return;
    try {
      final count = selection.selectByProperty(
        propertyName: name,
        propertyValue: value,
      );
      rust.reloadAllModelsMesh();
      _refresh();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Selected $count elements')),
        );
      }
    } catch (e) {
      debugPrint('[SELECTION] searchByProperty failed: $e');
    }
  }

  void _saveSelectionSet() {
    final name = 'Selection ${_selectionSets.length + 1}';
    try {
      selection.saveSelectionSet(name: name);
      _refresh();
      if (mounted) {
        ScaffoldMessenger.of(context)
            .showSnackBar(SnackBar(content: Text('Saved "$name"')));
      }
    } catch (e) {
      debugPrint('[SELECTION] saveSet failed: $e');
    }
  }

  void _restoreSelectionSet(String name) {
    try {
      selection.restoreSelectionSet(name: name);
      rust.reloadAllModelsMesh();
      _refresh();
    } catch (e) {
      debugPrint('[SELECTION] restoreSet failed: $e');
    }
  }

  void _deleteSelectionSet(String name) {
    try {
      selection.deleteSelectionSet(name: name);
      _refresh();
    } catch (e) {
      debugPrint('[SELECTION] deleteSet failed: $e');
    }
  }

  void _applySmartGroup(String name) {
    try {
      final count = selection.applySmartGroup(name: name);
      rust.reloadAllModelsMesh();
      _refresh();
      if (mounted) {
        ScaffoldMessenger.of(context)
            .showSnackBar(SnackBar(content: Text('$count elements matched')));
      }
    } catch (e) {
      debugPrint('[SELECTION] applySmartGroup failed: $e');
    }
  }

  void _deleteSmartGroup(String name) {
    try {
      selection.deleteSmartGroup(name: name);
      _refresh();
    } catch (e) {
      debugPrint('[SELECTION] deleteSmartGroup failed: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Container(
      constraints: BoxConstraints(
        maxHeight: MediaQuery.of(context).size.height * 0.85,
      ),
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(16)),
      ),
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text('Selection & Filters',
                    style: theme.textTheme.titleLarge
                        ?.copyWith(fontWeight: FontWeight.bold)),
                IconButton(
                  icon: const Icon(Icons.close),
                  onPressed: () => Navigator.pop(context),
                ),
              ],
            ),
            const SizedBox(height: 8),

            // Status bar
            Container(
              padding: const EdgeInsets.all(10),
              decoration: BoxDecoration(
                color: colorScheme.primaryContainer,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceAround,
                children: [
                  _statChip('Selected', _selectionCount, colorScheme.primary),
                  _statChip('Hidden', _hiddenCount, colorScheme.error),
                ],
              ),
            ),
            const SizedBox(height: 12),

            // Quick actions
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                _actionChip('Hide Selected', Icons.visibility_off,
                    _selectionCount > 0 ? _hideSelected : null),
                _actionChip('Isolate Selected', Icons.filter_center_focus,
                    _selectionCount > 0 ? _isolateSelected : null),
                _actionChip('Show All', Icons.visibility,
                    _hiddenCount > 0 ? _showAll : null),
                _actionChip('Clear Selection', Icons.deselect,
                    _selectionCount > 0 ? _clearSelection : null),
              ],
            ),

            const Divider(height: 24),

            // Search by property
            _sectionHeader('Search by Property', Icons.search),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _propNameController,
                    decoration: InputDecoration(
                      hintText: 'Property name',
                      border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(8)),
                      isDense: true,
                      contentPadding: const EdgeInsets.symmetric(
                          horizontal: 10, vertical: 8),
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: TextField(
                    controller: _propValueController,
                    decoration: InputDecoration(
                      hintText: 'Value',
                      border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(8)),
                      isDense: true,
                      contentPadding: const EdgeInsets.symmetric(
                          horizontal: 10, vertical: 8),
                    ),
                    onSubmitted: (_) => _searchByProperty(),
                  ),
                ),
                const SizedBox(width: 8),
                IconButton.filled(
                  icon: const Icon(Icons.search, size: 20),
                  onPressed: _searchByProperty,
                ),
              ],
            ),

            const Divider(height: 24),

            // Selection sets
            _sectionHeader('Selection Sets', Icons.bookmark),
            const SizedBox(height: 8),
            if (_selectionCount > 0)
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: OutlinedButton.icon(
                  onPressed: _saveSelectionSet,
                  icon: const Icon(Icons.add, size: 18),
                  label: const Text('Save Current Selection'),
                ),
              ),
            if (_selectionSets.isEmpty)
              Padding(
                padding: const EdgeInsets.all(8),
                child: Text('No saved selection sets',
                    style: theme.textTheme.bodySmall
                        ?.copyWith(color: colorScheme.onSurfaceVariant)),
              )
            else
              ..._selectionSets.map((name) => ListTile(
                    dense: true,
                    leading: const Icon(Icons.bookmark_outline, size: 20),
                    title: Text(name),
                    trailing: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        IconButton(
                          icon: const Icon(Icons.play_arrow, size: 18),
                          tooltip: 'Restore',
                          onPressed: () => _restoreSelectionSet(name),
                        ),
                        IconButton(
                          icon: Icon(Icons.delete_outline,
                              size: 18, color: colorScheme.error),
                          onPressed: () => _deleteSelectionSet(name),
                        ),
                      ],
                    ),
                  )),

            const Divider(height: 24),

            // Smart groups
            _sectionHeader('Smart Groups', Icons.auto_awesome),
            const SizedBox(height: 8),
            if (_smartGroups.isEmpty)
              Padding(
                padding: const EdgeInsets.all(8),
                child: Text(
                  'No smart groups defined.\nSmart groups filter elements by criteria (type, material, property values).',
                  style: theme.textTheme.bodySmall
                      ?.copyWith(color: colorScheme.onSurfaceVariant),
                ),
              )
            else
              ..._smartGroups.map((name) {
                int count = 0;
                try {
                  count = selection.getSmartGroupElementCount(name: name).toInt();
                } catch (_) {}
                return ListTile(
                  dense: true,
                  leading: const Icon(Icons.auto_awesome, size: 20),
                  title: Text(name),
                  subtitle: Text('$count elements'),
                  trailing: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      IconButton(
                        icon: const Icon(Icons.play_arrow, size: 18),
                        tooltip: 'Apply',
                        onPressed: () => _applySmartGroup(name),
                      ),
                      IconButton(
                        icon: Icon(Icons.delete_outline,
                            size: 18, color: colorScheme.error),
                        onPressed: () => _deleteSmartGroup(name),
                      ),
                    ],
                  ),
                );
              }),

            const SizedBox(height: 8),
          ],
        ),
      ),
    );
  }

  Widget _sectionHeader(String title, IconData icon) {
    final theme = Theme.of(context);
    return Row(
      children: [
        Icon(icon, size: 18, color: theme.colorScheme.primary),
        const SizedBox(width: 8),
        Text(title,
            style: theme.textTheme.titleSmall
                ?.copyWith(fontWeight: FontWeight.bold)),
      ],
    );
  }

  Widget _statChip(String label, int count, Color color) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('$count',
            style: TextStyle(
                fontSize: 20, fontWeight: FontWeight.bold, color: color)),
        Text(label, style: Theme.of(context).textTheme.labelSmall),
      ],
    );
  }

  Widget _actionChip(String label, IconData icon, VoidCallback? onPressed) {
    return ActionChip(
      avatar: Icon(icon, size: 16),
      label: Text(label, style: const TextStyle(fontSize: 12)),
      onPressed: onPressed,
    );
  }
}

/// Show selection actions panel as a bottom sheet.
void showSelectionActionsPanel(BuildContext context) {
  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    backgroundColor: Colors.transparent,
    builder: (_) => const SelectionActionsPanel(),
  );
}
