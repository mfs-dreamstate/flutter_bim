import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:file_picker/file_picker.dart';
import '../core/bridge/api.dart' as rust;
import '../core/bridge/bim/model_registry.dart';

/// Widget for managing multiple loaded models
class ModelManagerDrawer extends StatefulWidget {
  final VoidCallback? onModelsChanged;

  const ModelManagerDrawer({super.key, this.onModelsChanged});

  @override
  State<ModelManagerDrawer> createState() => _ModelManagerDrawerState();
}

class _ModelManagerDrawerState extends State<ModelManagerDrawer> {
  List<RegisteredModelInfo> _models = [];
  bool _isLoading = false;

  @override
  void initState() {
    super.initState();
    _refreshModels();
  }

  void _refreshModels() {
    setState(() {
      _models = rust.listLoadedModels();
    });
  }

  Future<void> _loadSampleModel() async {
    // Show sample selection dialog
    final selected = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Load Sample Model'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text('Select a discipline model to load:'),
            const SizedBox(height: 16),
            _SampleOption(
              icon: Icons.home,
              color: Colors.brown,
              title: 'Architectural',
              subtitle: 'Walls, doors, windows, roof',
              onTap: () => Navigator.pop(context, 'test/sample_architectural.ifc'),
            ),
            _SampleOption(
              icon: Icons.foundation,
              color: Colors.blueGrey,
              title: 'Structural',
              subtitle: 'Columns, beams, footings',
              onTap: () => Navigator.pop(context, 'test/sample_structural.ifc'),
            ),
            _SampleOption(
              icon: Icons.plumbing,
              color: Colors.teal,
              title: 'MEP (Mechanical)',
              subtitle: 'Pipes, ducts, vents',
              onTap: () => Navigator.pop(context, 'test/sample_mep.ifc'),
            ),
            _SampleOption(
              icon: Icons.electrical_services,
              color: Colors.orange,
              title: 'Electrical',
              subtitle: 'Cable trays, outlets, lights',
              onTap: () => Navigator.pop(context, 'test/sample_electrical.ifc'),
            ),
            const Divider(),
            _SampleOption(
              icon: Icons.apartment,
              color: Colors.grey,
              title: 'Original Sample',
              subtitle: 'Basic building elements',
              onTap: () => Navigator.pop(context, 'test/sample_building.ifc'),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
        ],
      ),
    );

    if (selected == null) return;

    setState(() => _isLoading = true);
    debugPrint('[MODEL_MANAGER] Loading sample: $selected');

    try {
      debugPrint('[MODEL_MANAGER] Reading asset file...');
      final content = await rootBundle.loadString(selected);
      debugPrint('[MODEL_MANAGER] Asset loaded, ${content.length} bytes');

      debugPrint('[MODEL_MANAGER] Parsing IFC content...');
      await rust.parseIfcContent(content: content);
      debugPrint('[MODEL_MANAGER] IFC parsed successfully');

      _refreshModels();
      widget.onModelsChanged?.call();

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Loaded: ${selected.split('/').last}'),
            backgroundColor: Colors.green,
          ),
        );
      }
    } catch (e, stackTrace) {
      debugPrint('[MODEL_MANAGER ERROR] Failed to load sample: $e');
      debugPrint('[MODEL_MANAGER ERROR] Stack trace: $stackTrace');
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Failed to load sample: $e'),
            backgroundColor: Colors.red,
            duration: const Duration(seconds: 5),
          ),
        );
      }
    } finally {
      if (mounted) setState(() => _isLoading = false);
    }
  }

  Future<void> _pickAndLoadModel() async {
    setState(() => _isLoading = true);

    try {
      final result = await FilePicker.platform.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['ifc'],
      );

      if (result == null || result.files.isEmpty) {
        setState(() => _isLoading = false);
        return;
      }

      final filePath = result.files.first.path;
      if (filePath == null) {
        setState(() => _isLoading = false);
        return;
      }

      await rust.loadIfcFile(filePath: filePath);
      _refreshModels();
      widget.onModelsChanged?.call();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Failed to load model: $e')),
        );
      }
    } finally {
      if (mounted) setState(() => _isLoading = false);
    }
  }

  Future<void> _unloadModel(String modelId) async {
    try {
      rust.unloadModelById(modelId: modelId);
      _refreshModels();
      widget.onModelsChanged?.call();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Failed to unload model: $e')),
        );
      }
    }
  }

  void _toggleModelVisibility(String modelId, bool visible) {
    try {
      rust.setModelVisible(modelId: modelId, visible: visible);
      _refreshModels();
      widget.onModelsChanged?.call();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Failed to toggle visibility: $e')),
        );
      }
    }
  }

  void _setPrimaryModel(String modelId) {
    try {
      rust.setPrimaryModel(modelId: modelId);
      _refreshModels();
      widget.onModelsChanged?.call();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Failed to set primary model: $e')),
        );
      }
    }
  }

  void _clearAllModels() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Clear All Models'),
        content: const Text('Are you sure you want to unload all models?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () {
              Navigator.pop(context);
              rust.clearAllModels();
              _refreshModels();
              widget.onModelsChanged?.call();
            },
            child: const Text('Clear All'),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

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
                    Icon(Icons.layers, color: colorScheme.onPrimaryContainer),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        'Model Manager',
                        style: Theme.of(context).textTheme.titleLarge?.copyWith(
                              color: colorScheme.onPrimaryContainer,
                              fontWeight: FontWeight.bold,
                            ),
                      ),
                    ),
                    if (_isLoading)
                      SizedBox(
                        width: 20,
                        height: 20,
                        child: CircularProgressIndicator(
                          strokeWidth: 2,
                          color: colorScheme.onPrimaryContainer,
                        ),
                      ),
                  ],
                ),
                const SizedBox(height: 8),
                Text(
                  '${_models.length} model${_models.length == 1 ? '' : 's'} loaded',
                  style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        color: colorScheme.onPrimaryContainer.withValues(alpha: 0.8),
                      ),
                ),
              ],
            ),
          ),

          // Action buttons
          Padding(
            padding: const EdgeInsets.all(12),
            child: Row(
              children: [
                Expanded(
                  child: FilledButton.icon(
                    onPressed: _isLoading ? null : _loadSampleModel,
                    icon: const Icon(Icons.file_open, size: 18),
                    label: const Text('Sample'),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: OutlinedButton.icon(
                    onPressed: _isLoading ? null : _pickAndLoadModel,
                    icon: const Icon(Icons.folder_open, size: 18),
                    label: const Text('Browse'),
                  ),
                ),
              ],
            ),
          ),

          const Divider(height: 1),

          // Model list
          Expanded(
            child: _models.isEmpty
                ? Center(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          Icons.architecture_outlined,
                          size: 64,
                          color: colorScheme.outline,
                        ),
                        const SizedBox(height: 16),
                        Text(
                          'No models loaded',
                          style: Theme.of(context).textTheme.titleMedium?.copyWith(
                                color: colorScheme.outline,
                              ),
                        ),
                        const SizedBox(height: 8),
                        Text(
                          'Load a sample or browse for an IFC file',
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                                color: colorScheme.outline,
                              ),
                        ),
                      ],
                    ),
                  )
                : ListView.builder(
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    itemCount: _models.length,
                    itemBuilder: (context, index) {
                      final model = _models[index];
                      return _ModelListTile(
                        model: model,
                        onToggleVisibility: (visible) =>
                            _toggleModelVisibility(model.id, visible),
                        onSetPrimary: () => _setPrimaryModel(model.id),
                        onUnload: () => _unloadModel(model.id),
                      );
                    },
                  ),
          ),

          // Footer actions
          if (_models.isNotEmpty) ...[
            const Divider(height: 1),
            Padding(
              padding: const EdgeInsets.all(12),
              child: Row(
                children: [
                  Expanded(
                    child: OutlinedButton.icon(
                      onPressed: _clearAllModels,
                      icon: Icon(Icons.delete_outline, color: colorScheme.error),
                      label: Text(
                        'Clear All',
                        style: TextStyle(color: colorScheme.error),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _ModelListTile extends StatelessWidget {
  final RegisteredModelInfo model;
  final Function(bool) onToggleVisibility;
  final VoidCallback onSetPrimary;
  final VoidCallback onUnload;

  const _ModelListTile({
    required this.model,
    required this.onToggleVisibility,
    required this.onSetPrimary,
    required this.onUnload,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Card(
      margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      child: ExpansionTile(
        leading: Stack(
          children: [
            Icon(
              Icons.architecture,
              color: model.visible ? colorScheme.primary : colorScheme.outline,
            ),
            if (model.isPrimary)
              Positioned(
                right: -2,
                bottom: -2,
                child: Container(
                  padding: const EdgeInsets.all(2),
                  decoration: BoxDecoration(
                    color: colorScheme.primary,
                    shape: BoxShape.circle,
                  ),
                  child: Icon(
                    Icons.star,
                    size: 10,
                    color: colorScheme.onPrimary,
                  ),
                ),
              ),
          ],
        ),
        title: Text(
          model.name,
          style: TextStyle(
            fontWeight: model.isPrimary ? FontWeight.bold : FontWeight.normal,
            color: model.visible ? null : colorScheme.outline,
          ),
        ),
        subtitle: Text(
          model.isPrimary ? 'Primary Model' : model.id,
          style: TextStyle(
            fontSize: 12,
            color: model.isPrimary ? colorScheme.primary : colorScheme.outline,
          ),
        ),
        trailing: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            IconButton(
              icon: Icon(
                model.visible ? Icons.visibility : Icons.visibility_off,
                color: model.visible ? colorScheme.primary : colorScheme.outline,
              ),
              onPressed: () => onToggleVisibility(!model.visible),
              tooltip: model.visible ? 'Hide model' : 'Show model',
            ),
          ],
        ),
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // Model info
                _InfoRow('Project', model.modelInfo.projectName),
                _InfoRow('Building', model.modelInfo.buildingName),
                _InfoRow('Elements', model.modelInfo.stats.totalEntities.toString()),

                const SizedBox(height: 12),

                // Actions
                Row(
                  children: [
                    if (!model.isPrimary)
                      Expanded(
                        child: OutlinedButton.icon(
                          onPressed: onSetPrimary,
                          icon: const Icon(Icons.star_outline, size: 18),
                          label: const Text('Set Primary'),
                        ),
                      ),
                    if (!model.isPrimary) const SizedBox(width: 8),
                    Expanded(
                      child: OutlinedButton.icon(
                        onPressed: onUnload,
                        icon: Icon(Icons.close, size: 18, color: colorScheme.error),
                        label: Text(
                          'Unload',
                          style: TextStyle(color: colorScheme.error),
                        ),
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  final String label;
  final String value;

  const _InfoRow(this.label, this.value);

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(
            label,
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.outline,
                ),
          ),
          Text(
            value,
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  fontWeight: FontWeight.w500,
                ),
          ),
        ],
      ),
    );
  }
}

class _SampleOption extends StatelessWidget {
  final IconData icon;
  final Color color;
  final String title;
  final String subtitle;
  final VoidCallback onTap;

  const _SampleOption({
    required this.icon,
    required this.color,
    required this.title,
    required this.subtitle,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: CircleAvatar(
        backgroundColor: color.withValues(alpha: 0.2),
        child: Icon(icon, color: color, size: 20),
      ),
      title: Text(title),
      subtitle: Text(subtitle, style: const TextStyle(fontSize: 12)),
      onTap: onTap,
      dense: true,
    );
  }
}
