import 'package:flutter/material.dart';
import '../core/bridge/api/camera.dart' as camera;

/// Camera modes panel — orthographic, turntable, walkthrough, fit-to targets.
class CameraModesPanel extends StatefulWidget {
  const CameraModesPanel({super.key});

  @override
  State<CameraModesPanel> createState() => _CameraModesPanelState();
}

class _CameraModesPanelState extends State<CameraModesPanel> {
  bool _orthographic = false;
  bool _turntable = false;
  bool _walkthrough = false;

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  void _refresh() {
    try {
      setState(() {
        _orthographic = camera.isOrthographic();
        _turntable = camera.isTurntableMode();
        _walkthrough = camera.isWalkthroughMode();
      });
    } catch (e) {
      debugPrint('[CAMERA] refresh failed: $e');
    }
  }

  void _toggleOrthographic(bool v) {
    try {
      camera.setOrthographic(enabled: v);
      setState(() => _orthographic = v);
    } catch (e) {
      debugPrint('[CAMERA] orthographic failed: $e');
    }
  }

  void _toggleTurntable(bool v) {
    try {
      camera.setTurntableMode(enabled: v);
      setState(() => _turntable = v);
    } catch (e) {
      debugPrint('[CAMERA] turntable failed: $e');
    }
  }

  void _toggleWalkthrough(bool v) {
    try {
      camera.setWalkthroughMode(enabled: v);
      setState(() => _walkthrough = v);
    } catch (e) {
      debugPrint('[CAMERA] walkthrough failed: $e');
    }
  }

  void _fitToModel() {
    try {
      camera.fitCameraToAllModels();
      if (mounted) Navigator.pop(context);
    } catch (_) {}
  }

  void _fitToSelected() {
    try {
      camera.fitCameraToSelected();
      if (mounted) Navigator.pop(context);
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context)
            .showSnackBar(SnackBar(content: Text('No selection: $e')));
      }
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
              Text('Camera Modes',
                  style: theme.textTheme.titleLarge
                      ?.copyWith(fontWeight: FontWeight.bold)),
              IconButton(
                icon: const Icon(Icons.close),
                onPressed: () => Navigator.pop(context),
              ),
            ],
          ),
          const SizedBox(height: 8),

          // Projection
          _sectionHeader('Projection', Icons.crop_square),
          SwitchListTile(
            secondary: const Icon(Icons.crop_square),
            title: const Text('Orthographic'),
            subtitle: const Text('No perspective distortion'),
            value: _orthographic,
            onChanged: _toggleOrthographic,
            dense: true,
          ),

          const Divider(),

          // Navigation modes
          _sectionHeader('Navigation Mode', Icons.navigation),
          SwitchListTile(
            secondary: const Icon(Icons.sync),
            title: const Text('Turntable'),
            subtitle: const Text('Constrain orbit to Y-up rotation'),
            value: _turntable,
            onChanged: _toggleTurntable,
            dense: true,
          ),
          SwitchListTile(
            secondary: const Icon(Icons.directions_walk),
            title: const Text('Walkthrough'),
            subtitle: const Text('First-person WASD navigation'),
            value: _walkthrough,
            onChanged: _toggleWalkthrough,
            dense: true,
          ),

          if (_walkthrough) ...[
            const SizedBox(height: 8),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.primaryContainer.withAlpha(128),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Text(
                'Use on-screen joystick or connect a gamepad.\n'
                'Drag viewport to look around.',
                style: theme.textTheme.bodySmall
                    ?.copyWith(color: colorScheme.onPrimaryContainer),
              ),
            ),
          ],

          const Divider(),

          // Fit-to shortcuts
          _sectionHeader('Fit Camera To', Icons.center_focus_strong),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              _fitButton('All Models', Icons.select_all, _fitToModel),
              _fitButton('Selection', Icons.select_all, _fitToSelected),
            ],
          ),

          const SizedBox(height: 8),
        ],
      ),
    );
  }

  Widget _sectionHeader(String title, IconData icon) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(top: 8, bottom: 4),
      child: Row(
        children: [
          Icon(icon, size: 18, color: theme.colorScheme.primary),
          const SizedBox(width: 8),
          Text(title,
              style: theme.textTheme.titleSmall
                  ?.copyWith(fontWeight: FontWeight.bold)),
        ],
      ),
    );
  }

  Widget _fitButton(String label, IconData icon, VoidCallback onTap) {
    return ActionChip(
      avatar: Icon(icon, size: 16),
      label: Text(label),
      onPressed: onTap,
    );
  }
}

/// Show camera modes panel as a bottom sheet.
void showCameraModesPanel(BuildContext context) {
  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    backgroundColor: Colors.transparent,
    builder: (_) => const CameraModesPanel(),
  );
}
