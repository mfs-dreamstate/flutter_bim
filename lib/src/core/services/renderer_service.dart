import 'dart:typed_data';

/// Abstract interface for renderer operations.
abstract interface class IRendererService {
  /// Initialize the renderer with given dimensions.
  Future<String> initialize({required int width, required int height});

  /// Check if the renderer is initialized.
  bool isInitialized();

  /// Render a frame and return RGBA pixel data.
  Uint8List renderFrame();

  /// Orbit the camera by delta amounts.
  void orbitCamera({required double deltaX, required double deltaY});

  /// Zoom the camera by delta amount.
  void zoomCamera({required double delta});

  /// Pan the camera (translate position and target).
  void panCamera({required double deltaX, required double deltaY});

  /// Fit camera to all visible model bounds.
  void fitCameraToAllModels();

  /// Fit camera to primary model bounds.
  void fitCameraToModel();

  /// Set render mode (0 = Shaded, 1 = Wireframe).
  void setRenderMode({required int mode});

  /// Get current render mode.
  int getRenderMode();

  /// Check if wireframe rendering is supported.
  bool isWireframeSupported();
}
