import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/constants/render_config.dart';
import '../../core/providers/model_state.dart';
import '../../core/providers/renderer_state.dart';
import '../../core/providers/visibility_state.dart';
import '../grid_overlay.dart';
import 'selection_info_bar.dart';
import 'viewport_widget.dart';

/// Main body of the viewer: status bar, viewport stack, control hints.
class ViewerBody extends ConsumerWidget {
  const ViewerBody({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final rendererState = ref.watch(rendererStateProvider);
    final modelState = ref.watch(modelStateProvider);
    final visibilityState = ref.watch(visibilityStateProvider);

    if (rendererState.error != null) {
      return _buildErrorView(context, ref, rendererState.error!);
    }

    if (rendererState.isInitializing) {
      return _buildLoadingView(context, rendererState.status);
    }

    return Column(
      children: [
        // Status bar
        Container(
          padding: const EdgeInsets.all(8),
          color: Theme.of(context).colorScheme.surfaceContainerHighest,
          child: Row(
            children: [
              Icon(
                Icons.check_circle,
                size: 16,
                color: Theme.of(context).colorScheme.primary,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  rendererState.status,
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
              Text(
                '${rendererState.renderWidth}x${rendererState.renderHeight}',
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ],
          ),
        ),

        // 3D Viewport
        Expanded(
          child: Stack(
            children: [
              const ViewportWidget(),
              // Grid overlay
              if (modelState.modelLoaded)
                GridOverlay(
                  visible: visibilityState.gridVisible,
                  color: const Color(RenderConfig.gridOverlayColor),
                  spacing: RenderConfig.gridSpacing,
                ),
              // Selection info bar
              const SelectionInfoBar(),
            ],
          ),
        ),

        // Controls hint
        Container(
          padding: const EdgeInsets.all(12),
          color: Theme.of(context).colorScheme.surfaceContainerHighest,
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              _buildControlHint(context, Icons.touch_app, 'Tap to select'),
              const SizedBox(width: 16),
              _buildControlHint(context, Icons.swipe, 'Drag to orbit'),
              const SizedBox(width: 16),
              _buildControlHint(context, Icons.pinch, 'Pinch to zoom'),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildErrorView(BuildContext context, WidgetRef ref, String error) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.error_outline,
            size: 64,
            color: Theme.of(context).colorScheme.error,
          ),
          const SizedBox(height: 16),
          Text(
            'Renderer Error',
            style: Theme.of(context).textTheme.headlineSmall,
          ),
          const SizedBox(height: 8),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 32),
            child: SelectableText(
              error,
              textAlign: TextAlign.center,
              style: Theme.of(context).textTheme.bodyMedium,
            ),
          ),
          const SizedBox(height: 24),
          ElevatedButton.icon(
            onPressed: () =>
                ref.read(rendererStateProvider.notifier).initialize(),
            icon: const Icon(Icons.refresh),
            label: const Text('Retry'),
          ),
        ],
      ),
    );
  }

  Widget _buildLoadingView(BuildContext context, String status) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          const CircularProgressIndicator(),
          const SizedBox(height: 16),
          Text(status),
        ],
      ),
    );
  }

  Widget _buildControlHint(
    BuildContext context,
    IconData icon,
    String label,
  ) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon,
            size: 16,
            color: Theme.of(context).colorScheme.onSurfaceVariant),
        const SizedBox(width: 4),
        Text(
          label,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
        ),
      ],
    );
  }
}
