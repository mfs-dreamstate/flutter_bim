import 'package:flutter/material.dart';
import '../core/bridge/api/camera.dart' as camera;

/// Viewpoint manager — save, restore, and delete named camera viewpoints.
class ViewpointManager extends StatefulWidget {
  const ViewpointManager({super.key});

  @override
  State<ViewpointManager> createState() => _ViewpointManagerState();
}

class _ViewpointManagerState extends State<ViewpointManager> {
  List<camera.ViewpointData> _viewpoints = [];
  final _nameController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  void _refresh() {
    try {
      setState(() => _viewpoints = camera.getViewpoints());
    } catch (e) {
      debugPrint('[VIEWPOINTS] refresh failed: $e');
    }
  }

  void _save() {
    final name = _nameController.text.trim();
    if (name.isEmpty) return;
    try {
      camera.saveViewpoint(name: name);
      _nameController.clear();
      _refresh();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Viewpoint "$name" saved')),
        );
      }
    } catch (e) {
      debugPrint('[VIEWPOINTS] save failed: $e');
    }
  }

  void _restore(String name) {
    try {
      camera.restoreViewpoint(name: name);
      if (mounted) Navigator.pop(context);
    } catch (e) {
      debugPrint('[VIEWPOINTS] restore failed: $e');
    }
  }

  void _delete(String name) {
    try {
      camera.deleteViewpoint(name: name);
      _refresh();
    } catch (e) {
      debugPrint('[VIEWPOINTS] delete failed: $e');
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
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('Saved Viewpoints',
                  style: theme.textTheme.titleLarge
                      ?.copyWith(fontWeight: FontWeight.bold)),
              IconButton(
                icon: const Icon(Icons.close),
                onPressed: () => Navigator.pop(context),
              ),
            ],
          ),
          const SizedBox(height: 12),
          // Save new viewpoint
          Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _nameController,
                  decoration: InputDecoration(
                    hintText: 'Viewpoint name',
                    border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(8)),
                    isDense: true,
                    contentPadding:
                        const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                  ),
                  onSubmitted: (_) => _save(),
                ),
              ),
              const SizedBox(width: 8),
              FilledButton.icon(
                onPressed: _save,
                icon: const Icon(Icons.save, size: 18),
                label: const Text('Save'),
              ),
            ],
          ),
          const SizedBox(height: 16),
          if (_viewpoints.isEmpty)
            Padding(
              padding: const EdgeInsets.all(24),
              child: Text(
                'No saved viewpoints yet.\nSave the current camera view to quickly return to it later.',
                textAlign: TextAlign.center,
                style: theme.textTheme.bodyMedium
                    ?.copyWith(color: colorScheme.onSurfaceVariant),
              ),
            )
          else
            ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 300),
              child: ListView.separated(
                shrinkWrap: true,
                itemCount: _viewpoints.length,
                separatorBuilder: (_, __) => const Divider(height: 1),
                itemBuilder: (context, index) {
                  final vp = _viewpoints[index];
                  return ListTile(
                    leading: Icon(
                      vp.orthographic ? Icons.crop_square : Icons.camera_alt,
                      color: colorScheme.primary,
                    ),
                    title: Text(vp.name),
                    subtitle: Text(
                      vp.orthographic ? 'Orthographic' : 'Perspective',
                      style: theme.textTheme.bodySmall,
                    ),
                    trailing: IconButton(
                      icon: Icon(Icons.delete_outline,
                          color: colorScheme.error, size: 20),
                      onPressed: () => _delete(vp.name),
                    ),
                    onTap: () => _restore(vp.name),
                  );
                },
              ),
            ),
          const SizedBox(height: 8),
        ],
      ),
    );
  }
}

/// Show viewpoint manager as a bottom sheet.
void showViewpointManager(BuildContext context) {
  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    backgroundColor: Colors.transparent,
    builder: (_) => const ViewpointManager(),
  );
}
