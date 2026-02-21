import 'dart:async';
import 'dart:typed_data';
import 'dart:ui' as ui;
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/bridge/api.dart' as rust;
import '../../core/constants/render_config.dart';
import '../../core/providers/model_state.dart';
import '../../core/providers/renderer_state.dart';
import '../../core/providers/selection_state.dart';
import '../../core/providers/service_providers.dart';

/// Interaction mode for single-finger gestures.
enum InteractionMode { orbit, pan, fly }

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

  // FastNav: track interaction state
  bool _interacting = false;
  Timer? _interactionEndTimer;

  // Level cut slider
  bool _levelCutEnabled = false;
  double _levelCutValue = 1.0; // 0.0 = bottom, 1.0 = top (no cut)
  double _boundsMinY = -50.0;
  double _boundsMaxY = 50.0;

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
    _interactionEndTimer?.cancel();
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

  /// Signal interaction start (FastNav: skip expensive post-processing)
  void _beginInteraction() {
    _interactionEndTimer?.cancel();
    if (!_interacting) {
      _interacting = true;
      try {
        rust.setInteractionActive(active: true);
      } catch (_) {}
    }
  }

  /// Signal interaction end after a short delay (let GPU pipeline settle)
  void _endInteraction() {
    _interactionEndTimer?.cancel();
    _interactionEndTimer = Timer(const Duration(milliseconds: 150), () {
      if (_interacting) {
        _interacting = false;
        try {
          rust.setInteractionActive(active: false);
        } catch (_) {}
        _markDirty(); // Re-render at full quality
      }
    });
  }

  /// Fetch scene bounds and update level cut range
  void _updateSceneBounds() {
    try {
      final bounds = rust.getSceneBounds();
      if (bounds.length >= 6) {
        setState(() {
          _boundsMinY = bounds[1].toDouble();
          _boundsMaxY = bounds[4].toDouble();
        });
      }
    } catch (_) {
      // No geometry loaded yet
    }
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
    _beginInteraction();
  }

  void _onScaleUpdate(ScaleUpdateDetails details) {
    final renderer = ref.read(rendererServiceProvider);
    final delta = details.focalPointDelta;
    final mode = ref.read(interactionModeProvider);

    if (mode == InteractionMode.fly) {
      // Fly mode: single finger = look around, pinch = fly forward/back,
      // two-finger pan = strafe
      if (_pointerCount == 2) {
        final isScaling = (details.scale - 1.0).abs() >= 0.02;
        if (isScaling) {
          final scaleDelta = details.scale - _lastScale;
          _lastScale = details.scale;
          renderer.flyCamera(
            forward: scaleDelta * RenderConfig.pinchZoomSensitivity,
            right: 0,
            up: 0,
          );
        } else {
          renderer.flyCamera(
            forward: 0,
            right: -delta.dx * RenderConfig.panSensitivity,
            up: delta.dy * RenderConfig.panSensitivity,
          );
        }
      } else {
        // Single finger: look around (yaw/pitch)
        renderer.lookCamera(
          deltaX: -delta.dx * RenderConfig.orbitSensitivity,
          deltaY: delta.dy * RenderConfig.orbitSensitivity,
        );
      }
    } else if (_pointerCount == 2) {
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

  void _onScaleEnd(ScaleEndDetails details) {
    _endInteraction();
  }

  void _onPointerSignal(PointerSignalEvent event) {
    if (event is PointerScrollEvent) {
      _beginInteraction();
      final renderer = ref.read(rendererServiceProvider);
      renderer.zoomCamera(
        delta: event.scrollDelta.dy * RenderConfig.scrollZoomSensitivity,
      );
      _markDirty();
      _endInteraction();
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

  Offset? _doubleTapPosition;

  void _onDoubleTapDown(TapDownDetails details) {
    _doubleTapPosition = details.localPosition;
  }

  void _onDoubleTap(BoxConstraints constraints) {
    final modelState = ref.read(modelStateProvider);
    if (!modelState.modelLoaded || _doubleTapPosition == null) return;

    final screenX = _doubleTapPosition!.dx / constraints.maxWidth;
    final screenY = _doubleTapPosition!.dy / constraints.maxHeight;

    final renderer = ref.read(rendererServiceProvider);
    final hit = renderer.setOrbitCenterFromScreen(
      screenX: screenX,
      screenY: screenY,
    );

    if (hit) {
      debugPrint('[Viewport] Orbit center set to tapped point');
    }
    _markDirty();
  }

  /// Sync walkthrough mode on the Rust side when interaction mode changes.
  void _onModeChanged(InteractionMode? prev, InteractionMode next) {
    final renderer = ref.read(rendererServiceProvider);
    if (next == InteractionMode.fly) {
      renderer.setWalkthroughMode(enabled: true);
    } else if (prev == InteractionMode.fly) {
      renderer.setWalkthroughMode(enabled: false);
    }
  }

  void _toggleLevelCut() {
    setState(() {
      _levelCutEnabled = !_levelCutEnabled;
      if (_levelCutEnabled) {
        _updateSceneBounds();
        _levelCutValue = 1.0; // Start fully open (no cut)
        // Disable section fill (stencil caps) during level cut — it produces
        // visual artifacts with placeholder box geometry.
        try {
          rust.setSectionFillEnabled(enabled: false);
        } catch (_) {}
      } else {
        // Clear section plane and re-enable section fill when disabling
        try {
          rust.clearSectionPlane();
          rust.setSectionFillEnabled(enabled: true);
        } catch (_) {}
        _markDirty();
      }
    });
  }

  void _onLevelCutChanged(double value) {
    setState(() {
      _levelCutValue = value;
    });
    // Map slider value to Y position
    final yPos = _boundsMinY + (_boundsMaxY - _boundsMinY) * value;
    try {
      if (value >= 0.99) {
        // At the top: clear section plane
        rust.clearSectionPlane();
      } else {
        // Set horizontal section plane cutting from above
        // Normal pointing DOWN (-Y) so everything above the plane is clipped
        rust.setSectionPlane(
          originX: 0,
          originY: yPos,
          originZ: 0,
          normalX: 0,
          normalY: -1,
          normalZ: 0,
        );
      }
    } catch (_) {}
    _markDirty();
  }

  @override
  Widget build(BuildContext context) {
    // Watch interaction mode so FAB rebuilds
    final mode = ref.watch(interactionModeProvider);
    final modelState = ref.watch(modelStateProvider);

    // Sync walkthrough mode on the Rust side when interaction mode changes
    ref.listen<InteractionMode>(interactionModeProvider, _onModeChanged);

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
                  onScaleEnd: _onScaleEnd,
                  onTapUp: (details) => _onTapUp(details, constraints),
                  onDoubleTapDown: _onDoubleTapDown,
                  onDoubleTap: () => _onDoubleTap(constraints),
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
            // Level cut slider (vertical, right side)
            if (_levelCutEnabled && modelState.modelLoaded)
              Positioned(
                right: 64,
                top: 16,
                bottom: 72,
                child: _LevelCutSlider(
                  value: _levelCutValue,
                  onChanged: _onLevelCutChanged,
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
            // Mode toggle FABs + level cut toggle
            Positioned(
              right: 12,
              bottom: 12,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (modelState.modelLoaded)
                    _ModeButton(
                      icon: Icons.content_cut,
                      label: 'Level Cut',
                      active: _levelCutEnabled,
                      onPressed: _toggleLevelCut,
                    ),
                  if (modelState.modelLoaded) const SizedBox(height: 8),
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
                  const SizedBox(height: 8),
                  _ModeButton(
                    icon: Icons.flight,
                    label: 'Fly',
                    active: mode == InteractionMode.fly,
                    onPressed: () => ref.read(interactionModeProvider.notifier).state =
                        InteractionMode.fly,
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

/// Vertical slider for level cuts - clips the model from top to bottom.
class _LevelCutSlider extends StatelessWidget {
  final double value;
  final ValueChanged<double> onChanged;

  const _LevelCutSlider({
    required this.value,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 40,
      child: Column(
        children: [
          // Top label
          const Icon(Icons.arrow_drop_up, color: Colors.white54, size: 20),
          // Vertical slider
          Expanded(
            child: RotatedBox(
              quarterTurns: -1,
              child: SliderTheme(
                data: SliderThemeData(
                  trackHeight: 4,
                  thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 8),
                  activeTrackColor: Colors.blue.shade300,
                  inactiveTrackColor: Colors.white24,
                  thumbColor: Colors.blue.shade400,
                  overlayColor: Colors.blue.withAlpha(40),
                ),
                child: Slider(
                  value: value,
                  min: 0.0,
                  max: 1.0,
                  onChanged: onChanged,
                ),
              ),
            ),
          ),
          // Bottom label
          const Icon(Icons.arrow_drop_down, color: Colors.white54, size: 20),
          // Percentage label
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
            decoration: BoxDecoration(
              color: Colors.black54,
              borderRadius: BorderRadius.circular(4),
            ),
            child: Text(
              '${(value * 100).round()}%',
              style: const TextStyle(
                color: Colors.white70,
                fontSize: 10,
                fontFamily: 'monospace',
              ),
            ),
          ),
        ],
      ),
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
