/// Render configuration constants.
/// Single source of truth for all rendering magic numbers.
class RenderConfig {
  RenderConfig._();

  /// Render surface width in pixels (reduced for mobile performance, scaled up).
  static const int width = 480;

  /// Render surface height in pixels.
  static const int height = 360;

  /// Gesture throttle interval in milliseconds (~60Hz max gesture rate).
  static const int gestureThrottleMs = 16;

  /// Render loop interval in milliseconds (~60 FPS).
  static const int renderIntervalMs = 16;

  /// Orbit sensitivity multiplier for single-finger drag.
  static const double orbitSensitivity = 1.2;

  /// Orbit sensitivity multiplier for mouse drag.
  static const double mouseOrbitSensitivity = 0.01;

  /// Zoom sensitivity for pinch gestures.
  static const double pinchZoomSensitivity = 80.0;

  /// Zoom sensitivity for mouse wheel.
  static const double scrollZoomSensitivity = 0.01;

  /// Background color for the 3D viewport.
  static const int viewportBackgroundColor = 0xFF1A1A24;

  /// Grid overlay color.
  static const int gridOverlayColor = 0x22FFFFFF;

  /// Grid overlay spacing in logical pixels.
  static const double gridSpacing = 60.0;
}
