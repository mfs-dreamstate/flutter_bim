import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../services/model_service.dart';
import 'service_providers.dart';

/// State for loaded models.
@immutable
class ModelState {
  final bool modelLoaded;
  final int modelCount;
  final bool isLoading;

  const ModelState({
    this.modelLoaded = false,
    this.modelCount = 0,
    this.isLoading = false,
  });

  ModelState copyWith({
    bool? modelLoaded,
    int? modelCount,
    bool? isLoading,
  }) {
    return ModelState(
      modelLoaded: modelLoaded ?? this.modelLoaded,
      modelCount: modelCount ?? this.modelCount,
      isLoading: isLoading ?? this.isLoading,
    );
  }
}

/// Notifier managing model loading state.
class ModelNotifier extends Notifier<ModelState> {
  @override
  ModelState build() => const ModelState();

  IModelService get _service => ref.read(modelServiceProvider);

  void refreshModelCount() {
    final count = _service.getModelCount();
    state = state.copyWith(modelCount: count);
  }

  Future<String> loadAllModelsIntoRenderer() async {
    state = state.copyWith(isLoading: true);
    try {
      final result = _service.loadAllModelsIntoRenderer();
      refreshModelCount();
      state = state.copyWith(modelLoaded: true, isLoading: false);
      return result;
    } catch (e) {
      state = state.copyWith(isLoading: false);
      rethrow;
    }
  }

  void setModelLoaded(bool loaded) {
    state = state.copyWith(modelLoaded: loaded);
  }

  void onModelsChanged() {
    refreshModelCount();
    if (state.modelLoaded) {
      // Defer heavy renderer reload to next microtask so the caller's
      // frame isn't blocked by the sync FFI call
      Future.microtask(() {
        try {
          _service.loadAllModelsIntoRenderer();
        } catch (e) {
          debugPrint('[ModelNotifier] Error reloading models: $e');
        }
      });
    }
  }
}

/// Provider for the model state.
final modelStateProvider =
    NotifierProvider<ModelNotifier, ModelState>(ModelNotifier.new);
