import 'package:flutter/foundation.dart';
import '../../bridge/api.dart' as rust;
import '../../bridge/bim/model.dart';
import '../../errors/bim_error.dart';
import '../selection_service.dart';

/// Rust implementation of [ISelectionService].
class RustSelectionService implements ISelectionService {
  @override
  ElementInfo? pickElement({required double screenX, required double screenY}) {
    try {
      return rust.pickElement(screenX: screenX, screenY: screenY);
    } catch (e) {
      throw BimSelectionError(
        userMessage: 'Failed to pick element',
        technicalMessage: 'pickElement($screenX, $screenY) failed: $e',
        cause: e,
      );
    }
  }

  @override
  void setSelectedElement({int? elementId}) {
    try {
      rust.setSelectedElement(elementId: elementId);
    } catch (e) {
      debugPrint('[RustSelectionService] setSelectedElement failed: $e');
    }
  }

  @override
  List<ElementInfo> getAllElementsFromAllModels() {
    try {
      return rust.getAllElementsFromAllModels();
    } catch (e) {
      throw BimSelectionError(
        userMessage: 'Failed to load elements',
        technicalMessage: 'getAllElementsFromAllModels failed: $e',
        cause: e,
      );
    }
  }

  @override
  List<ElementInfo> getAllElements() => rust.getAllElements();

  @override
  String reloadAllModelsMesh() {
    try {
      return rust.reloadAllModelsMesh();
    } catch (e) {
      throw BimSelectionError(
        userMessage: 'Failed to reload mesh',
        technicalMessage: 'reloadAllModelsMesh failed: $e',
        cause: e,
      );
    }
  }
}
