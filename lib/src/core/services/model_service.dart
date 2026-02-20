import '../bridge/bim/model.dart';
import '../bridge/bim/model_registry.dart';

/// Abstract interface for model management operations.
abstract interface class IModelService {
  /// Load an IFC file from a file path.
  Future<ModelInfo> loadIfcFile({required String filePath});

  /// Parse IFC content from a string.
  Future<ModelInfo> parseIfcContent({required String content});

  /// Unload a specific model by ID.
  void unloadModelById({required String modelId});

  /// List all loaded models.
  List<RegisteredModelInfo> listLoadedModels();

  /// Get the number of loaded models.
  int getModelCount();

  /// Set model visibility.
  void setModelVisible({required String modelId, required bool visible});

  /// Set the primary model.
  void setPrimaryModel({required String modelId});

  /// Clear all models.
  void clearAllModels();

  /// Check if any model is loaded.
  bool isModelLoaded();

  /// Load all visible models into the renderer.
  String loadAllModelsIntoRenderer();

  /// Get model info for the primary model.
  ModelInfo getModelInfo();
}
