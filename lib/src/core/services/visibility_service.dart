/// Abstract interface for element type visibility operations.
abstract interface class IVisibilityService {
  /// Set visibility for an element type.
  void setElementTypeVisible({required String elementType, required bool visible});

  /// Check if an element type is visible.
  bool isElementTypeVisible({required String elementType});

  /// Get all hidden element types.
  List<String> getHiddenElementTypes();

  /// Set grid visibility.
  void setGridVisible({required bool visible});

  /// Check if grid is visible.
  bool isGridVisible();
}
