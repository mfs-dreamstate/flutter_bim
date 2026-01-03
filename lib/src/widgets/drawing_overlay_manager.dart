import 'dart:io';
import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:file_picker/file_picker.dart';
import 'package:image/image.dart' as img;
import 'core/bridge/api.dart' as rust;

/// Drawing Overlay Manager Widget
/// Allows users to upload floor plans/drawings and overlay them on the 3D model
class DrawingOverlayManager extends StatefulWidget {
  const DrawingOverlayManager({super.key});

  @override
  State<DrawingOverlayManager> createState() => _DrawingOverlayManagerState();
}

class _DrawingOverlayManagerState extends State<DrawingOverlayManager> {
  String _viewMode = '3d';
  String? _overlayId;
  double _opacity = 0.7;
  double _positionX = 0.0;
  double _positionY = 0.0;
  double _positionZ = 0.1; // Slightly above ground
  double _scaleX = 10.0;
  double _scaleY = 10.0;
  double _rotation = 0.0;
  bool _overlayVisible = true;
  bool _uploading = false;

  @override
  void initState() {
    super.initState();
    _loadViewMode();
  }

  Future<void> _loadViewMode() async {
    try {
      final mode = rust.getViewMode();
      setState(() {
        _viewMode = mode;
      });
    } catch (e) {
      debugPrint('[OVERLAY] Failed to get view mode: $e');
    }
  }

  Future<void> _setViewMode(String mode) async {
    try {
      rust.setViewMode(mode: mode);
      setState(() {
        _viewMode = mode;
      });
    } catch (e) {
      debugPrint('[OVERLAY] Failed to set view mode: $e');
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Failed to set view mode: $e')),
        );
      }
    }
  }

  Future<void> _pickAndUploadImage() async {
    try {
      // Pick image file
      final result = await FilePicker.platform.pickFiles(
        type: FileType.image,
        withData: true,
      );

      if (result == null || result.files.isEmpty) return;

      setState(() {
        _uploading = true;
      });

      final file = result.files.first;
      final bytes = file.bytes ?? await File(file.path!).readAsBytes();

      // Decode image
      final image = img.decodeImage(bytes);
      if (image == null) {
        throw Exception('Failed to decode image');
      }

      // Convert to RGBA
      final rgba = image.convert(numChannels: 4);
      final rgbaBytes = rgba.getBytes();

      // Upload to Rust
      final overlayId = 'overlay_${DateTime.now().millisecondsSinceEpoch}';
      await rust.uploadDrawingOverlay(
        id: overlayId,
        width: rgba.width,
        height: rgba.height,
        rgbaPixels: rgbaBytes,
      );

      setState(() {
        _overlayId = overlayId;
        _uploading = false;
      });

      // Apply initial transform
      _updateTransform();

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Overlay uploaded: ${file.name} (${rgba.width}x${rgba.height})'),
          ),
        );
      }
    } catch (e) {
      setState(() {
        _uploading = false;
      });
      debugPrint('[OVERLAY] Failed to upload image: $e');
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Failed to upload overlay: $e'),
            backgroundColor: Theme.of(context).colorScheme.error,
          ),
        );
      }
    }
  }

  void _updateTransform() {
    if (_overlayId == null) return;

    try {
      rust.setOverlayTransform(
        id: _overlayId!,
        positionX: _positionX,
        positionY: _positionY,
        positionZ: _positionZ,
        scaleX: _scaleX,
        scaleY: _scaleY,
        rotation: _rotation,
      );
    } catch (e) {
      debugPrint('[OVERLAY] Failed to update transform: $e');
    }
  }

  void _updateOpacity() {
    if (_overlayId == null) return;

    try {
      rust.setOverlayOpacity(id: _overlayId!, opacity: _opacity);
    } catch (e) {
      debugPrint('[OVERLAY] Failed to update opacity: $e');
    }
  }

  void _toggleVisibility() {
    if (_overlayId == null) return;

    setState(() {
      _overlayVisible = !_overlayVisible;
    });

    try {
      rust.setOverlayVisible(id: _overlayId!, visible: _overlayVisible);
    } catch (e) {
      debugPrint('[OVERLAY] Failed to toggle visibility: $e');
    }
  }

  void _removeOverlay() {
    if (_overlayId == null) return;

    try {
      rust.removeOverlay(id: _overlayId!);
      setState(() {
        _overlayId = null;
        _overlayVisible = true;
      });

      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Overlay removed')),
      );
    } catch (e) {
      debugPrint('[OVERLAY] Failed to remove overlay: $e');
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
                'Drawing Overlay',
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

          // View Mode Selection
          Text(
            'View Mode:',
            style: theme.textTheme.titleSmall?.copyWith(fontWeight: FontWeight.bold),
          ),
          const SizedBox(height: 8),
          SegmentedButton<String>(
            segments: const [
              ButtonSegment(value: '3d', label: Text('3D'), icon: Icon(Icons.view_in_ar)),
              ButtonSegment(value: '2d', label: Text('2D'), icon: Icon(Icons.map)),
              ButtonSegment(value: 'overlay', label: Text('Overlay'), icon: Icon(Icons.layers)),
            ],
            selected: {_viewMode},
            onSelectionChanged: (Set<String> selection) {
              _setViewMode(selection.first);
            },
          ),

          const SizedBox(height: 24),

          // Upload Button
          if (_overlayId == null)
            FilledButton.icon(
              onPressed: _uploading ? null : _pickAndUploadImage,
              icon: _uploading
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.upload_file),
              label: Text(_uploading ? 'Uploading...' : 'Upload Floor Plan'),
            )
          else ...[
            // Overlay Controls
            Row(
              children: [
                Expanded(
                  child: Text(
                    'Overlay Active',
                    style: theme.textTheme.titleMedium?.copyWith(
                      color: colorScheme.primary,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                ),
                Switch(
                  value: _overlayVisible,
                  onChanged: (_) => _toggleVisibility(),
                ),
              ],
            ),

            const SizedBox(height: 16),

            // Opacity Slider
            Text('Opacity: ${(_opacity * 100).toInt()}%'),
            Slider(
              value: _opacity,
              min: 0.0,
              max: 1.0,
              divisions: 20,
              label: '${(_opacity * 100).toInt()}%',
              onChanged: (value) {
                setState(() {
                  _opacity = value;
                });
                _updateOpacity();
              },
            ),

            const SizedBox(height: 16),

            // Scale Controls
            Text('Scale: ${_scaleX.toStringAsFixed(1)}m × ${_scaleY.toStringAsFixed(1)}m'),
            Row(
              children: [
                Expanded(
                  child: Slider(
                    value: _scaleX,
                    min: 1.0,
                    max: 100.0,
                    label: '${_scaleX.toStringAsFixed(1)}m',
                    onChanged: (value) {
                      setState(() {
                        _scaleX = value;
                        _scaleY = value; // Keep aspect ratio
                      });
                      _updateTransform();
                    },
                  ),
                ),
              ],
            ),

            const SizedBox(height: 16),

            // Rotation Slider
            Text('Rotation: ${(_rotation * 180 / 3.14159).toStringAsFixed(0)}°'),
            Slider(
              value: _rotation,
              min: -3.14159,
              max: 3.14159,
              divisions: 72,
              label: '${(_rotation * 180 / 3.14159).toStringAsFixed(0)}°',
              onChanged: (value) {
                setState(() {
                  _rotation = value;
                });
                _updateTransform();
              },
            ),

            const SizedBox(height: 16),

            // Remove Button
            OutlinedButton.icon(
              onPressed: _removeOverlay,
              icon: const Icon(Icons.delete),
              label: const Text('Remove Overlay'),
              style: OutlinedButton.styleFrom(
                foregroundColor: colorScheme.error,
              ),
            ),
          ],

          const SizedBox(height: 16),

          // Info
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: colorScheme.primaryContainer.withValues(alpha: 0.5),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Row(
              children: [
                Icon(Icons.info_outline, size: 16, color: colorScheme.onPrimaryContainer),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Upload floor plans or 2D drawings to compare with the 3D model',
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

/// Show drawing overlay manager bottom sheet
void showDrawingOverlayManager(BuildContext context) {
  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    backgroundColor: Colors.transparent,
    builder: (context) => const DrawingOverlayManager(),
  );
}
