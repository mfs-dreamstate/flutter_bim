import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/bridge/api.dart' as rust;
import '../../core/bridge/api/selection.dart' as rust_selection;
import '../../core/providers/model_state.dart';
import '../../core/providers/renderer_state.dart';
import '../../core/providers/service_providers.dart';
import '../../core/providers/visibility_state.dart';
import '../measurement_tools.dart';
import '../section_plane_tools.dart';
import '../drawing_overlay_manager.dart';
import '../settings_screen.dart';
import 'lighting_settings_sheet.dart';

/// AppBar actions for the viewer: visibility, fit, grid, wireframe, tools.
class ViewerToolbar extends ConsumerWidget {
  final GlobalKey<ScaffoldState> scaffoldKey;

  const ViewerToolbar({super.key, required this.scaffoldKey});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final rendererState = ref.watch(rendererStateProvider);
    final modelState = ref.watch(modelStateProvider);
    final visibilityState = ref.watch(visibilityStateProvider);
    final modelService = ref.read(modelServiceProvider);

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Element tree button
        if (modelState.modelLoaded)
          IconButton(
            icon: const Icon(Icons.account_tree),
            onPressed: () => scaffoldKey.currentState?.openDrawer(),
            tooltip: 'Element Tree',
          ),
        // Model manager button
        IconButton(
          icon: Badge(
            isLabelVisible: modelState.modelCount > 0,
            label: Text('${modelState.modelCount}'),
            child: const Icon(Icons.layers),
          ),
          onPressed: () => scaffoldKey.currentState?.openEndDrawer(),
          tooltip: 'Model Manager',
        ),
        // Load models button
        if (!modelState.modelLoaded && modelService.isModelLoaded())
          IconButton(
            icon: const Icon(Icons.upload),
            onPressed: () async {
              try {
                final result = await ref
                    .read(modelStateProvider.notifier)
                    .loadAllModelsIntoRenderer();
                ref.read(rendererStateProvider.notifier).setStatus(result);
              } catch (e) {
                ref
                    .read(rendererStateProvider.notifier)
                    .setError('Failed to load models: $e');
              }
            },
            tooltip: 'Load Models',
          ),
        // Visibility popup
        if (modelState.modelLoaded)
          PopupMenuButton<String>(
            icon: const Icon(Icons.visibility),
            tooltip: 'Element Visibility',
            onSelected: (type) => ref
                .read(visibilityStateProvider.notifier)
                .toggleTypeVisibility(type),
            itemBuilder: (context) =>
                visibilityState.typeVisibility.entries.map((e) {
              return PopupMenuItem<String>(
                value: e.key,
                child: Row(
                  children: [
                    Icon(
                      e.value ? Icons.visibility : Icons.visibility_off,
                      color: e.value
                          ? Theme.of(context).colorScheme.primary
                          : Theme.of(context).colorScheme.outline,
                      size: 20,
                    ),
                    const SizedBox(width: 12),
                    Text(e.key),
                  ],
                ),
              );
            }).toList(),
          ),
        // Fit to model
        if (modelState.modelLoaded)
          IconButton(
            icon: const Icon(Icons.center_focus_strong),
            onPressed: () =>
                ref.read(rendererServiceProvider).fitCameraToAllModels(),
            tooltip: 'Fit to Model',
          ),
        // Grid toggle
        if (modelState.modelLoaded)
          IconButton(
            icon: Icon(
              Icons.grid_4x4,
              color: visibilityState.gridVisible
                  ? null
                  : Theme.of(context).colorScheme.outline,
            ),
            onPressed: () => ref
                .read(visibilityStateProvider.notifier)
                .toggleGridVisibility(),
            tooltip: visibilityState.gridVisible ? 'Hide Grid' : 'Show Grid',
          ),
        // Render mode selector
        if (modelState.modelLoaded)
          PopupMenuButton<int>(
            icon: Icon(
              rendererState.renderMode == 2
                  ? Icons.blur_on
                  : rendererState.renderMode == 1
                      ? Icons.grid_3x3
                      : Icons.view_in_ar,
            ),
            tooltip: 'Render Mode',
            onSelected: (mode) {
              ref.read(rendererStateProvider.notifier).setRenderMode(mode);
              // Reload mesh to apply/remove X-ray alpha
              if (mode == 2 || rendererState.renderMode == 2) {
                try {
                  rust.reloadAllModelsMesh();
                } catch (_) {}
              }
            },
            itemBuilder: (context) => [
              PopupMenuItem<int>(
                value: 0,
                child: Row(
                  children: [
                    Icon(Icons.view_in_ar,
                        color: rendererState.renderMode == 0
                            ? Theme.of(context).colorScheme.primary
                            : null,
                        size: 20),
                    const SizedBox(width: 12),
                    const Text('Shaded'),
                  ],
                ),
              ),
              if (rendererState.wireframeSupported)
                PopupMenuItem<int>(
                  value: 1,
                  child: Row(
                    children: [
                      Icon(Icons.grid_3x3,
                          color: rendererState.renderMode == 1
                              ? Theme.of(context).colorScheme.primary
                              : null,
                          size: 20),
                      const SizedBox(width: 12),
                      const Text('Wireframe'),
                    ],
                  ),
                ),
              PopupMenuItem<int>(
                value: 2,
                child: Row(
                  children: [
                    Icon(Icons.blur_on,
                        color: rendererState.renderMode == 2
                            ? Theme.of(context).colorScheme.primary
                            : null,
                        size: 20),
                    const SizedBox(width: 12),
                    const Text('X-Ray'),
                  ],
                ),
              ),
            ],
          ),
        // Color mode selector
        if (modelState.modelLoaded)
          PopupMenuButton<int>(
            icon: const Icon(Icons.palette),
            tooltip: 'Color Mode',
            onSelected: (mode) {
              try {
                rust_selection.setColorMode(mode: mode);
                rust.reloadAllModelsMesh();
              } catch (_) {}
            },
            itemBuilder: (context) {
              final currentMode = rust_selection.getColorMode();
              return [
                _colorModeItem(context, 0, 'Normal', Icons.format_paint, currentMode),
                _colorModeItem(context, 1, 'By Type', Icons.category, currentMode),
                _colorModeItem(context, 2, 'By Storey', Icons.layers, currentMode),
                _colorModeItem(context, 3, 'By Material', Icons.texture, currentMode),
              ];
            },
          ),
        // Measurement tools
        if (modelState.modelLoaded)
          IconButton(
            icon: const Icon(Icons.straighten),
            onPressed: () => showMeasurementTools(
              context,
              onMeasurementStarted: (type) {
                debugPrint('[MEASUREMENT] Started $type measurement');
              },
              onMeasurementCleared: () {
                debugPrint('[MEASUREMENT] Cleared measurement');
              },
            ),
            tooltip: 'Measurement Tools',
          ),
        // Section plane
        if (modelState.modelLoaded)
          IconButton(
            icon: const Icon(Icons.cut),
            onPressed: () => showSectionPlaneTools(
              context,
              onPlaneChanged: () {
                debugPrint('[SECTION] Section plane changed');
              },
            ),
            tooltip: 'Section Plane',
          ),
        // Drawing overlay
        if (modelState.modelLoaded)
          IconButton(
            icon: const Icon(Icons.layers),
            onPressed: () => showDrawingOverlayManager(context),
            tooltip: 'Drawing Overlay',
          ),
        // Lighting settings
        IconButton(
          icon: const Icon(Icons.light_mode),
          onPressed: () => showLightingSettings(context),
          tooltip: 'Lighting Settings',
        ),
        // Restart renderer
        IconButton(
          icon: const Icon(Icons.refresh),
          onPressed: () async {
            await ref.read(rendererStateProvider.notifier).initialize();
            ref.read(modelStateProvider.notifier).refreshModelCount();
            final modelService = ref.read(modelServiceProvider);
            if (modelService.isModelLoaded()) {
              try {
                final result = await ref
                    .read(modelStateProvider.notifier)
                    .loadAllModelsIntoRenderer();
                ref.read(rendererStateProvider.notifier).setStatus(result);
              } catch (e) {
                debugPrint('[ViewerToolbar] Error loading models: $e');
              }
            }
          },
          tooltip: 'Restart Renderer',
        ),
        // Settings
        IconButton(
          icon: const Icon(Icons.settings),
          onPressed: () => Navigator.push(
            context,
            MaterialPageRoute(builder: (context) => const SettingsScreen()),
          ),
          tooltip: 'Settings',
        ),
      ],
    );
  }

  PopupMenuItem<int> _colorModeItem(
    BuildContext context,
    int value,
    String label,
    IconData icon,
    int currentMode,
  ) {
    return PopupMenuItem<int>(
      value: value,
      child: Row(
        children: [
          Icon(icon,
              color: currentMode == value
                  ? Theme.of(context).colorScheme.primary
                  : null,
              size: 20),
          const SizedBox(width: 12),
          Text(label),
        ],
      ),
    );
  }
}
