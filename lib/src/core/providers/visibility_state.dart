import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../constants/bim_element_types.dart';
import '../services/selection_service.dart';
import '../services/visibility_service.dart';
import 'service_providers.dart';

/// State for element type visibility and grid.
@immutable
class VisibilityState {
  final Map<String, bool> typeVisibility;
  final bool gridVisible;

  const VisibilityState({
    this.typeVisibility = const {},
    this.gridVisible = true,
  });

  VisibilityState copyWith({
    Map<String, bool>? typeVisibility,
    bool? gridVisible,
  }) {
    return VisibilityState(
      typeVisibility: typeVisibility ?? this.typeVisibility,
      gridVisible: gridVisible ?? this.gridVisible,
    );
  }
}

/// Notifier managing element type visibility.
class VisibilityNotifier extends Notifier<VisibilityState> {
  @override
  VisibilityState build() => VisibilityState(
        typeVisibility: Map.of(BimElementVisuals.defaultVisibility),
      );

  IVisibilityService get _visService => ref.read(visibilityServiceProvider);
  ISelectionService get _selService => ref.read(selectionServiceProvider);

  void toggleTypeVisibility(String elementType) {
    final current = state.typeVisibility[elementType] ?? true;
    final newVisibility = !current;

    final updated = Map<String, bool>.from(state.typeVisibility);
    updated[elementType] = newVisibility;
    state = state.copyWith(typeVisibility: updated);

    try {
      _visService.setElementTypeVisible(
        elementType: elementType,
        visible: newVisibility,
      );
      _selService.reloadAllModelsMesh();
    } catch (e) {
      debugPrint('[VisibilityNotifier] toggleTypeVisibility failed: $e');
    }
  }

  void toggleGridVisibility() {
    state = state.copyWith(gridVisible: !state.gridVisible);
  }
}

/// Provider for the visibility state.
final visibilityStateProvider =
    NotifierProvider<VisibilityNotifier, VisibilityState>(VisibilityNotifier.new);
