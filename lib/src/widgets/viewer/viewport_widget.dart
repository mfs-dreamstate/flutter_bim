import 'dart:async';
import 'dart:typed_data';
import 'dart:ui' as ui;
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/constants/render_config.dart';
import '../../core/providers/model_state.dart';
import '../../core/providers/renderer_state.dart';
import '../../core/providers/selection_state.dart';
import '../../core/providers/service_providers.dart';

/// Interaction mode for single-finger gestures.
enum InteractionMode { orbit, pan }

/// Provider for the current interaction mode.
final interactionModeProvider = StateProvider<InteractionMode>(
  (ref) => InteractionMode.orbit,
);

/// The 3D render surface with gesture handling and render loop.
class ViewportWidget extends ConsumerStatefulWidget {
  const ViewportWidget({super.key});

  @override
  ConsumerState<ViewportWidget> createState() => _ViewportWidgetState();
}

class _ViewportWidgetState extends ConsumerState<ViewportWidget> {
  ui.Image? _frameImage;
  Timer? _renderTimer;
  bool _isRendering = false;

  // Dirty flag: only re-render when something changed
  bool _needsRender = true;
  int _idleFrames = 0;

  // FPS counter
  int _frameCount = 0;
  int _displayFps = 0;
  int _displayFrameTimeMs = 0;
  Timer? _fpsTimer;

  // Touch tracking
  double _lastScale = 1.0;
  int _pointerCount = 0;

  @override
  void initState() {
    super.initState();
    _startRenderLoop();
    _fpsTimer = Timer.periodic(const Duration(seconds: 1), (_) {
      if (mounted) {
        setState(() {
          _displayFps = _frameCount;
          _frameCount = 0;
        });
      }
    });
  }

  @override
  void dispose() {
    _renderTimer?.cancel();
    _fpsTimer?.cancel();
    super.dispose();
  }

  void _markDirty() {
    _needsRender = true;
    _idleFrames = 0;
  }

  void _startRenderLoop() {
    _renderTimer = Timer.periodic(
      const Duration(milliseconds: RenderConfig.renderIntervalMs),
      (_) => _renderFrame(),
    );
  }

  Future<void> _renderFrame() async {
    if (_isRendering) return;

    // Skip rendering if nothing changed (render a few extra frames after
    // last interaction to catch GPU pipeline latency)
    if (!_needsRender) {
      _idleFrames++;
      if (_idleFrames > 3) return; // 3 extra frames after last change
    }

    final renderer = ref.read(rendererServiceProvider);
    if (!renderer.isInitialized()) return;

    final rState = ref.read(rendererStateProvider);

    _isRendering = true;
    _needsRender = false;
    try {
      final sw = Stopwatch()..start();
      final Uint8List pixels = renderer.renderFrame();
      final image = await _createImageFromPixels(
        pixels,
        rState.renderWidth,
        rState.renderHeight,
      );
      sw.stop();
      _displayFrameTimeMs = sw.elapsedMilliseconds;
      _frameCount++;

      if (mounted) {
        setState(() {
          _frameImage = image;
        });
      }
    } catch (e) {
      debugPrint('Render error: $e');
    } finally {
      _isRendering = false;
    }
  }

  Future<ui.Image> _createImageFromPixels(
    Uint8List pixels,
    int width,
    int height,
  ) async {
    final completer = Completer<ui.Image>();
    ui.decodeImageFromPixels(
      pixels,
      width,
      height,
      ui.PixelFormat.rgba8888,
      (image) => completer.complete(image),
    );
    return completer.future;
  }

  void _onScaleStart(ScaleStartDetails details) {
    _lastScale = 1.0;
    _pointerCount = details.pointerCount;
  }

  void _onScaleUpdate(ScaleUpdateDetails details) {
    final renderer = ref.read(rendererServiceProvider);
    final delta = details.focalPointDelta;
    final mode = ref.read(interactionModeProvider);

    if (_pointerCount == 2) {
      final isScaling = (details.scale - 1.0).abs() >= 0.02;
      if (isScaling) {
        // Pinch zoom
        final scaleDelta = details.scale - _lastScale;
        _lastScale = details.scale;
        renderer.zoomCamera(
            delta: scaleDelta * RenderConfig.pinchZoomSensitivity);
      } else {
        // Two-finger pan (always pan regardless of mode)
        renderer.panCamera(
          deltaX: -delta.dx * RenderConfig.panSensitivity,
          deltaY: delta.dy * RenderConfig.panSensitivity,
        );
      }
    } else {
      // Single finger: depends on mode
      if (mode == InteractionMode.pan) {
        renderer.panCamera(
          deltaX: -delta.dx * RenderConfig.panSensitivity,
          deltaY: delta.dy * RenderConfig.panSensitivity,
        );
      } else {
        renderer.orbitCamera(
          deltaX: -delta.dx * RenderConfig.orbitSensitivity,
          deltaY: delta.dy * RenderConfig.orbitSensitivity,
        );
      }
    }
    _markDirty();
  }

  void _onPointerSignal(PointerSignalEvent event) {
    if (event is PointerScrollEvent) {
      final renderer = ref.read(rendererServiceProvider);
      renderer.zoomCamera(
        delta: event.scrollDelta.dy * RenderConfig.scrollZoomSensitivity,
      );
      _markDirty();
    }
  }

  void _onTapUp(TapUpDetails details, BoxConstraints constraints) {
    final modelState = ref.read(modelStateProvider);
    if (!modelState.modelLoaded) return;

    final screenX = details.localPosition.dx / constraints.maxWidth;
    final screenY = details.localPosition.dy / constraints.maxHeight;

    ref.read(selectionStateProvider.notifier).pickElement(
          screenX: screenX,
          screenY: screenY,
        );
    _markDirty();
  }

  @override
  Widget build(BuildContext context) {
    // Watch interaction mode so FAB rebuilds
    final mode = ref.watch(interactionModeProvider);

    return LayoutBuilder(
      builder: (context, constraints) {
        return Stack(
          children: [
            // Viewport
            Positioned.fill(
              child: Listener(
                onPointerSignal: _onPointerSignal,
                child: GestureDetector(
                  onScaleStart: _onScaleStart,
                  onScaleUpdate: _onScaleUpdate,
                  onTapUp: (details) => _onTapUp(details, constraints),
                  child: Container(
                    color: const Color(RenderConfig.viewportBackgroundColor),
                    child: _frameImage != null
                        ? SizedBox.expand(
                            child: RawImage(
                              image: _frameImage,
                              fit: BoxFit.fill,
                            ),
                          )
                        : const Center(
                            child: Text(
                              'Waiting for frame...',
                              style: TextStyle(color: Colors.white54),
                            ),
                          ),
                  ),
                ),
              ),
            ),
            // FPS counter
            Positioned(
              left: 8,
              bottom: 8,
              child: IgnorePointer(
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
                  decoration: BoxDecoration(
                    color: Colors.black54,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    '$_displayFps fps  ${_displayFrameTimeMs}ms',
                    style: const TextStyle(
                      color: Colors.white70,
                      fontSize: 11,
                      fontFamily: 'monospace',
                    ),
                  ),
                ),
              ),
            ),
            // Mode toggle FAB
            Positioned(
              right: 12,
              bottom: 12,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  _ModeButton(
                    icon: Icons.open_with,
                    label: 'Pan',
                    active: mode == InteractionMode.pan,
                    onPressed: () => ref.read(interactionModeProvider.notifier).state =
                        InteractionMode.pan,
                  ),
                  const SizedBox(height: 8),
                  _ModeButton(
                    icon: Icons.threed_rotation,
                    label: 'Orbit',
                    active: mode == InteractionMode.orbit,
                    onPressed: () => ref.read(interactionModeProvider.notifier).state =
                        InteractionMode.orbit,
                  ),
                ],
              ),
            ),
          ],
        );
      },
    );
  }
}

class _ModeButton extends StatelessWidget {
  final IconData icon;
  final String label;
  final bool active;
  final VoidCallback onPressed;

  const _ModeButton({
    required this.icon,
    required this.label,
    required this.active,
    required this.onPressed,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return SizedBox(
      width: 48,
      height: 48,
      child: Material(
        elevation: active ? 4 : 1,
        shape: const CircleBorder(),
        color: active ? colorScheme.primaryContainer : colorScheme.surface,
        child: InkWell(
          customBorder: const CircleBorder(),
          onTap: onPressed,
          child: Tooltip(
            message: label,
            child: Icon(
              icon,
              size: 22,
              color: active ? colorScheme.onPrimaryContainer : colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      ),
    );
  }
}
