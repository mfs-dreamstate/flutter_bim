import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../bridge/bim/model.dart';
import '../services/selection_service.dart';
import 'service_providers.dart';

/// Notifier managing selected element state.
class SelectionNotifier extends Notifier<ElementInfo?> {
  @override
  ElementInfo? build() => null;

  ISelectionService get _service => ref.read(selectionServiceProvider);

  void selectElement(ElementInfo element) {
    state = element;
    _service.setSelectedElement(elementId: element.id);
    _service.reloadAllModelsMesh();
  }

  void clearSelection() {
    state = null;
    try {
      _service.setSelectedElement(elementId: null);
      _service.reloadAllModelsMesh();
    } catch (e) {
      debugPrint('[SelectionNotifier] clearSelection failed: $e');
    }
  }

  ElementInfo? pickElement({required double screenX, required double screenY}) {
    try {
      final element = _service.pickElement(screenX: screenX, screenY: screenY);
      if (element != null) {
        selectElement(element);
      } else {
        clearSelection();
      }
      return element;
    } catch (e) {
      debugPrint('[SelectionNotifier] pickElement failed: $e');
      return null;
    }
  }
}

/// Provider for the selection state.
final selectionStateProvider =
    NotifierProvider<SelectionNotifier, ElementInfo?>(SelectionNotifier.new);
