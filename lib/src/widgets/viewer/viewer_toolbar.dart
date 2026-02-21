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
        // Fit to model
        if (modelState.modelLoaded)
          IconButton(
            icon: const Icon(Icons.center_focus_strong),
            onPressed: () =>
                ref.read(rendererServiceProvider).fitCameraToAllModels(),
            tooltip: 'Fit to Model',
          ),
        // Overflow menu with all other tools
        PopupMenuButton<String>(
          icon: const Icon(Icons.more_vert),
          tooltip: 'More Tools',
          onSelected: (action) => _handleOverflowAction(context, ref, action),
          itemBuilder: (context) => [
            if (modelState.modelLoaded) ...[
              PopupMenuItem<String>(
                value: 'visibility',
                child: _menuRow(Icons.visibility, 'Element Visibility'),
              ),
              PopupMenuItem<String>(
                value: 'grid',
                child: _menuRow(
                  Icons.grid_4x4,
                  visibilityState.gridVisible ? 'Hide Grid' : 'Show Grid',
                ),
              ),
              PopupMenuItem<String>(
                value: 'render_mode',
                child: _menuRow(Icons.view_in_ar, 'Render Mode'),
              ),
              PopupMenuItem<String>(
                value: 'color_mode',
                child: _menuRow(Icons.palette, 'Color Mode'),
              ),
              const PopupMenuDivider(),
              PopupMenuItem<String>(
                value: 'measure',
                child: _menuRow(Icons.straighten, 'Measurement'),
              ),
              PopupMenuItem<String>(
                value: 'section',
                child: _menuRow(Icons.cut, 'Section Plane'),
              ),
              PopupMenuItem<String>(
                value: 'drawing',
                child: _menuRow(Icons.draw, 'Drawing Overlay'),
              ),
              const PopupMenuDivider(),
            ],
            PopupMenuItem<String>(
              value: 'lighting',
              child: _menuRow(Icons.light_mode, 'Lighting'),
            ),
            PopupMenuItem<String>(
              value: 'restart',
              child: _menuRow(Icons.refresh, 'Restart Renderer'),
            ),
            PopupMenuItem<String>(
              value: 'settings',
              child: _menuRow(Icons.settings, 'Settings'),
            ),
          ],
        ),
      ],
    );
  }

  static Widget _menuRow(IconData icon, String label) {
    return Row(
      children: [
        Icon(icon, size: 20),
        const SizedBox(width: 12),
        Text(label),
      ],
    );
  }

  void _handleOverflowAction(BuildContext context, WidgetRef ref, String action) {
    final rendererState = ref.read(rendererStateProvider);
    final visibilityState = ref.read(visibilityStateProvider);

    switch (action) {
      case 'visibility':
        _showVisibilityMenu(context, ref, visibilityState);
      case 'grid':
        ref.read(visibilityStateProvider.notifier).toggleGridVisibility();
      case 'render_mode':
        _showRenderModeMenu(context, ref, rendererState);
      case 'color_mode':
        _showColorModeMenu(context, ref);
      case 'measure':
        showMeasurementTools(
          context,
          onMeasurementStarted: (type) => debugPrint('[MEASUREMENT] Started $type'),
          onMeasurementCleared: () => debugPrint('[MEASUREMENT] Cleared'),
        );
      case 'section':
        showSectionPlaneTools(
          context,
          onPlaneChanged: () => debugPrint('[SECTION] Changed'),
        );
      case 'drawing':
        showDrawingOverlayManager(context);
      case 'lighting':
        showLightingSettings(context);
      case 'restart':
        _restartRenderer(ref);
      case 'settings':
        Navigator.push(context, MaterialPageRoute(builder: (_) => const SettingsScreen()));
    }
  }

  void _showVisibilityMenu(BuildContext context, WidgetRef ref, dynamic visibilityState) {
    final RenderBox button = context.findRenderObject()! as RenderBox;
    final overlay = Overlay.of(context).context.findRenderObject()! as RenderBox;
    showMenu<String>(
      context: context,
      position: RelativeRect.fromRect(
        button.localToGlobal(Offset.zero) & button.size,
        Offset.zero & overlay.size,
      ),
      items: visibilityState.typeVisibility.entries.map<PopupMenuEntry<String>>((e) {
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
    ).then((type) {
      if (type != null) {
        ref.read(visibilityStateProvider.notifier).toggleTypeVisibility(type);
      }
    });
  }

  void _showRenderModeMenu(BuildContext context, WidgetRef ref, dynamic rendererState) {
    final RenderBox button = context.findRenderObject()! as RenderBox;
    final overlay = Overlay.of(context).context.findRenderObject()! as RenderBox;
    showMenu<int>(
      context: context,
      position: RelativeRect.fromRect(
        button.localToGlobal(Offset.zero) & button.size,
        Offset.zero & overlay.size,
      ),
      items: [
        PopupMenuItem<int>(
          value: 0,
          child: _menuRow(Icons.view_in_ar, 'Shaded'),
        ),
        if (rendererState.wireframeSupported)
          PopupMenuItem<int>(
            value: 1,
            child: _menuRow(Icons.grid_3x3, 'Wireframe'),
          ),
        PopupMenuItem<int>(
          value: 2,
          child: _menuRow(Icons.blur_on, 'X-Ray'),
        ),
      ],
    ).then((mode) {
      if (mode != null) {
        ref.read(rendererStateProvider.notifier).setRenderMode(mode);
        if (mode == 2 || rendererState.renderMode == 2) {
          try { rust.reloadAllModelsMesh(); } catch (_) {}
        }
      }
    });
  }

  void _showColorModeMenu(BuildContext context, WidgetRef ref) {
    final RenderBox button = context.findRenderObject()! as RenderBox;
    final overlay = Overlay.of(context).context.findRenderObject()! as RenderBox;
    final currentMode = rust_selection.getColorMode();
    showMenu<int>(
      context: context,
      position: RelativeRect.fromRect(
        button.localToGlobal(Offset.zero) & button.size,
        Offset.zero & overlay.size,
      ),
      items: [
        _colorModeItem(context, 0, 'Normal', Icons.format_paint, currentMode),
        _colorModeItem(context, 1, 'By Type', Icons.category, currentMode),
        _colorModeItem(context, 2, 'By Storey', Icons.layers, currentMode),
        _colorModeItem(context, 3, 'By Material', Icons.texture, currentMode),
      ],
    ).then((mode) {
      if (mode != null) {
        try {
          rust_selection.setColorMode(mode: mode);
          rust.reloadAllModelsMesh();
        } catch (_) {}
      }
    });
  }

  Future<void> _restartRenderer(WidgetRef ref) async {
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
