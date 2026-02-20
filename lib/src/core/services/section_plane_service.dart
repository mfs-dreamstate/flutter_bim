/// Abstract interface for section plane operations.
abstract interface class ISectionPlaneService {
  /// Set a section plane with origin and normal.
  void setSectionPlane({
    required double originX,
    required double originY,
    required double originZ,
    required double normalX,
    required double normalY,
    required double normalZ,
  });

  /// Enable or disable the section plane.
  void setSectionPlaneEnabled({required bool enabled});

  /// Clear the section plane.
  void clearSectionPlane();

  /// Check if section plane is active.
  bool isSectionPlaneActive();

  /// Set section plane from axis (0=X, 1=Y, 2=Z) and position.
  void setSectionPlaneFromAxis({required int axis, required double position});
}
