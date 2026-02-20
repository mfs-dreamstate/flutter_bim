/// Abstract interface for drawing overlay operations.
abstract interface class IOverlayService {
  /// Upload a drawing overlay image.
  Future<void> uploadDrawingOverlay({
    required String id,
    required int width,
    required int height,
    required List<int> rgbaPixels,
  });

  /// Set overlay transform (position, scale, rotation).
  void setOverlayTransform({
    required String id,
    required double positionX,
    required double positionY,
    required double positionZ,
    required double scaleX,
    required double scaleY,
    required double rotation,
  });

  /// Set overlay opacity.
  void setOverlayOpacity({required String id, required double opacity});

  /// Set overlay visibility.
  void setOverlayVisible({required String id, required bool visible});

  /// Remove an overlay.
  void removeOverlay({required String id});

  /// Set view mode ('3d', '2d', 'overlay').
  void setViewMode({required String mode});

  /// Get current view mode.
  String getViewMode();
}
