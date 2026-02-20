import '../bridge/bim/model.dart';

/// Abstract interface for element selection operations.
abstract interface class ISelectionService {
  /// Pick an element at normalized screen coordinates.
  ElementInfo? pickElement({required double screenX, required double screenY});

  /// Set the selected element by ID (null to clear).
  void setSelectedElement({int? elementId});

  /// Get all elements from all visible models.
  List<ElementInfo> getAllElementsFromAllModels();

  /// Get all elements from the primary model.
  List<ElementInfo> getAllElements();

  /// Reload all model meshes (applies visibility/highlight changes).
  String reloadAllModelsMesh();
}
