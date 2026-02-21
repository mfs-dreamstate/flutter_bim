import 'package:flutter/material.dart';
import '../core/bridge/api/comparison.dart' as comparison;
import '../core/bridge/api/models.dart' as models;
import '../core/bridge/api.dart' as rust;

/// Model comparison panel — compare two loaded models and view differences.
class ModelComparisonPanel extends StatefulWidget {
  const ModelComparisonPanel({super.key});

  @override
  State<ModelComparisonPanel> createState() => _ModelComparisonPanelState();
}

class _ModelComparisonPanelState extends State<ModelComparisonPanel> {
  List<_ModelEntry> _models = [];
  String? _modelAId;
  String? _modelBId;
  comparison.ComparisonResult? _result;
  bool _overlayActive = false;
  bool _running = false;

  @override
  void initState() {
    super.initState();
    _loadModels();
    _overlayActive = comparison.isComparisonOverlayActive();
  }

  void _loadModels() {
    try {
      final loaded = models.listLoadedModels();
      setState(() {
        _models = loaded
            .map((m) => _ModelEntry(id: m.id, name: m.name))
            .toList();
      });
    } catch (e) {
      debugPrint('[COMPARISON] loadModels failed: $e');
    }
  }

  void _runComparison() {
    if (_modelAId == null || _modelBId == null) return;
    setState(() => _running = true);
    try {
      final result = comparison.compareModels(
        modelAId: _modelAId!,
        modelBId: _modelBId!,
      );
      setState(() {
        _result = result;
        _running = false;
      });
    } catch (e) {
      setState(() => _running = false);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Comparison failed: $e')),
        );
      }
    }
  }

  void _toggleOverlay() {
    try {
      if (_overlayActive) {
        comparison.clearComparisonOverlay();
        rust.reloadAllModelsMesh();
      } else if (_modelAId != null && _modelBId != null) {
        comparison.setComparisonOverlay(
          modelIdA: _modelAId!,
          modelIdB: _modelBId!,
        );
        rust.reloadAllModelsMesh();
      }
      setState(() => _overlayActive = !_overlayActive);
    } catch (e) {
      debugPrint('[COMPARISON] overlay toggle failed: $e');
    }
  }

  void _navigateToGroup(String status) {
    try {
      comparison.navigateToComparisonGroup(status: status);
      rust.reloadAllModelsMesh();
    } catch (e) {
      debugPrint('[COMPARISON] navigate failed: $e');
    }
  }

  void _exportCsv() {
    if (_modelAId == null || _modelBId == null) return;
    try {
      final csv = comparison.exportComparisonCsv(
        modelAId: _modelAId!,
        modelBId: _modelBId!,
      );
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('CSV exported (${csv.length} chars)')),
        );
      }
    } catch (e) {
      debugPrint('[COMPARISON] export failed: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Container(
      constraints: BoxConstraints(
        maxHeight: MediaQuery.of(context).size.height * 0.8,
      ),
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(16)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('Model Comparison',
                  style: theme.textTheme.titleLarge
                      ?.copyWith(fontWeight: FontWeight.bold)),
              IconButton(
                icon: const Icon(Icons.close),
                onPressed: () => Navigator.pop(context),
              ),
            ],
          ),
          const SizedBox(height: 12),

          if (_models.length < 2)
            Padding(
              padding: const EdgeInsets.all(24),
              child: Text(
                'Load at least 2 models to compare revisions.',
                textAlign: TextAlign.center,
                style: theme.textTheme.bodyMedium
                    ?.copyWith(color: colorScheme.onSurfaceVariant),
              ),
            )
          else ...[
            // Model selectors
            _modelDropdown('Baseline (old)', _modelAId,
                (v) => setState(() => _modelAId = v)),
            const SizedBox(height: 8),
            _modelDropdown('Updated (new)', _modelBId,
                (v) => setState(() => _modelBId = v)),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: FilledButton.icon(
                    onPressed: (_modelAId != null &&
                            _modelBId != null &&
                            _modelAId != _modelBId &&
                            !_running)
                        ? _runComparison
                        : null,
                    icon: _running
                        ? const SizedBox(
                            width: 18,
                            height: 18,
                            child:
                                CircularProgressIndicator(strokeWidth: 2))
                        : const Icon(Icons.compare_arrows),
                    label: const Text('Compare'),
                  ),
                ),
                if (_result != null) ...[
                  const SizedBox(width: 8),
                  IconButton(
                    icon: Icon(
                      _overlayActive
                          ? Icons.visibility
                          : Icons.visibility_off,
                      color: _overlayActive ? colorScheme.primary : null,
                    ),
                    tooltip: 'Toggle color overlay',
                    onPressed: _toggleOverlay,
                  ),
                  IconButton(
                    icon: const Icon(Icons.download, size: 20),
                    tooltip: 'Export CSV',
                    onPressed: _exportCsv,
                  ),
                ],
              ],
            ),
          ],

          if (_result != null) ...[
            const SizedBox(height: 16),
            // Summary cards
            Row(
              children: [
                _statusCard('Added', _result!.addedElements.length,
                    Colors.green, 'added'),
                const SizedBox(width: 8),
                _statusCard('Removed', _result!.removedElements.length,
                    Colors.red, 'removed'),
                const SizedBox(width: 8),
                _statusCard('Modified', _result!.modifiedElements.length,
                    Colors.orange, 'modified'),
                const SizedBox(width: 8),
                _statusCard('Same', _result!.unchangedCount,
                    Colors.grey, null),
              ],
            ),
            const Divider(height: 24),
            // Detail lists
            Flexible(
              child: DefaultTabController(
                length: 3,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    TabBar(
                      isScrollable: true,
                      tabs: [
                        Tab(
                            text:
                                'Added (${_result!.addedElements.length})'),
                        Tab(
                            text:
                                'Removed (${_result!.removedElements.length})'),
                        Tab(
                            text:
                                'Modified (${_result!.modifiedElements.length})'),
                      ],
                    ),
                    Flexible(
                      child: TabBarView(
                        children: [
                          _elementList(_result!.addedElements
                              .map((e) => _DiffItem(
                                  e.name, e.elementType, null))
                              .toList()),
                          _elementList(_result!.removedElements
                              .map((e) => _DiffItem(
                                  e.name, e.elementType, null))
                              .toList()),
                          _elementList(_result!.modifiedElements
                              .map((e) => _DiffItem(e.name, e.elementType,
                                  e.changes.join(', ')))
                              .toList()),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }

  Widget _modelDropdown(
      String label, String? value, ValueChanged<String?> onChanged) {
    return DropdownButtonFormField<String>(
      value: value,
      decoration: InputDecoration(
        labelText: label,
        border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
        isDense: true,
        contentPadding:
            const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      ),
      items: _models
          .map((m) => DropdownMenuItem(
                value: m.id,
                child: Text(m.name, overflow: TextOverflow.ellipsis),
              ))
          .toList(),
      onChanged: onChanged,
    );
  }

  Widget _statusCard(String label, int count, Color color, String? status) {
    return Expanded(
      child: GestureDetector(
        onTap: status != null ? () => _navigateToGroup(status) : null,
        child: Container(
          padding: const EdgeInsets.symmetric(vertical: 10),
          decoration: BoxDecoration(
            color: color.withAlpha(25),
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: color.withAlpha(80)),
          ),
          child: Column(
            children: [
              Text('$count',
                  style: TextStyle(
                      fontSize: 18,
                      fontWeight: FontWeight.bold,
                      color: color)),
              Text(label,
                  style: TextStyle(fontSize: 10, color: color)),
            ],
          ),
        ),
      ),
    );
  }

  Widget _elementList(List<_DiffItem> items) {
    if (items.isEmpty) {
      return const Center(child: Text('None'));
    }
    return ListView.builder(
      shrinkWrap: true,
      itemCount: items.length,
      itemBuilder: (context, index) {
        final item = items[index];
        return ListTile(
          dense: true,
          title: Text(item.name, style: Theme.of(context).textTheme.bodySmall),
          subtitle: Text(
            item.detail ?? item.type,
            style: Theme.of(context).textTheme.labelSmall,
          ),
        );
      },
    );
  }
}

class _ModelEntry {
  final String id;
  final String name;
  _ModelEntry({required this.id, required this.name});
}

class _DiffItem {
  final String name;
  final String type;
  final String? detail;
  _DiffItem(this.name, this.type, this.detail);
}

/// Show model comparison panel as a bottom sheet.
void showModelComparisonPanel(BuildContext context) {
  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    backgroundColor: Colors.transparent,
    builder: (_) => const ModelComparisonPanel(),
  );
}
