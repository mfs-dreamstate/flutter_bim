import 'dart:async';
import 'dart:typed_data';
import 'dart:ui' as ui;
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/constants/render_config.dart';
import '../../core/providers/model_state.dart';
import '../../core/providers/selection_state.dart';
import '../../core/providers/service_providers.dart';

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

  // Touch tracking
  double _lastScale = 1.0;

  @override
  void initState() {
    super.initState();
    _startRenderLoop();
  }

  @override
  void dispose() {
    _renderTimer?.cancel();
    super.dispose();
  }

  void _startRenderLoop() {
    _renderTimer = Timer.periodic(
      const Duration(milliseconds: RenderConfig.renderIntervalMs),
      (_) => _renderFrame(),
    );
  }

  Future<void> _renderFrame() async {
    if (_isRendering) return;

    final renderer = ref.read(rendererServiceProvider);
    if (!renderer.isInitialized()) return;

    _isRendering = true;
    try {
      final Uint8List pixels = renderer.renderFrame();
      final image = await _createImageFromPixels(
        pixels,
        RenderConfig.width,
        RenderConfig.height,
      );

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
  }

  void _onScaleUpdate(ScaleUpdateDetails details) {
    final renderer = ref.read(rendererServiceProvider);

    if (details.pointerCount >= 2) {
      final scaleDelta = details.scale - _lastScale;
      _lastScale = details.scale;
      renderer.zoomCamera(delta: scaleDelta * RenderConfig.pinchZoomSensitivity);
    } else {
      final delta = details.focalPointDelta;
      renderer.orbitCamera(
        deltaX: -delta.dx * RenderConfig.orbitSensitivity,
        deltaY: delta.dy * RenderConfig.orbitSensitivity,
      );
    }
  }

  void _onPointerSignal(PointerSignalEvent event) {
    if (event is PointerScrollEvent) {
      final renderer = ref.read(rendererServiceProvider);
      renderer.zoomCamera(
        delta: event.scrollDelta.dy * RenderConfig.scrollZoomSensitivity,
      );
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
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        return Listener(
          onPointerSignal: _onPointerSignal,
          child: GestureDetector(
            onScaleStart: _onScaleStart,
            onScaleUpdate: _onScaleUpdate,
            onTapUp: (details) => _onTapUp(details, constraints),
            child: Container(
              color: const Color(RenderConfig.viewportBackgroundColor),
              child: Center(
                child: _frameImage != null
                    ? RawImage(
                        image: _frameImage,
                        fit: BoxFit.contain,
                      )
                    : const Text(
                        'Waiting for frame...',
                        style: TextStyle(color: Colors.white54),
                      ),
              ),
            ),
          ),
        );
      },
    );
  }
}
