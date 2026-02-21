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

  /// Set interaction active state for FastNav quality adaptation.
  void setInteractionActive({required bool active});

  /// Get scene bounding box in world coordinates [minX, minY, minZ, maxX, maxY, maxZ].
  List<double> getSceneBounds();

  /// Fly-mode movement: constant speed derived from scene scale.
  void flyCamera({required double forward, required double right, required double up});

  /// Rotate the camera view direction (yaw/pitch) for fly/walkthrough mode.
  void lookCamera({required double deltaX, required double deltaY});

  /// Enable or disable walkthrough (fly) mode.
  void setWalkthroughMode({required bool enabled});

  /// Set orbit center by picking a surface point from screen coordinates.
  /// Returns true if a surface was hit.
  bool setOrbitCenterFromScreen({required double screenX, required double screenY});
}
