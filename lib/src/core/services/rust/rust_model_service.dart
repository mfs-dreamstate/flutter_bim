import 'package:flutter/foundation.dart';
import '../../bridge/api.dart' as rust;
import '../../bridge/bim/model.dart';
import '../../bridge/bim/model_registry.dart';
import '../../errors/bim_error.dart';
import '../model_service.dart';

/// Rust implementation of [IModelService].
class RustModelService implements IModelService {
  @override
  Future<ModelInfo> loadIfcFile({required String filePath}) async {
    try {
      return await rust.loadIfcFile(filePath: filePath);
    } catch (e) {
      throw BimModelError(
        userMessage: 'Failed to load IFC file',
        technicalMessage: 'loadIfcFile($filePath) failed: $e',
        cause: e,
      );
    }
  }

  @override
  Future<ModelInfo> parseIfcContent({required String content}) async {
    try {
      return await rust.parseIfcContent(content: content);
    } catch (e) {
      throw BimModelError(
        userMessage: 'Failed to parse IFC content',
        technicalMessage: 'parseIfcContent failed: $e',
        cause: e,
      );
    }
  }

  @override
  void unloadModelById({required String modelId}) {
    try {
      rust.unloadModelById(modelId: modelId);
    } catch (e) {
      throw BimModelError(
        userMessage: 'Failed to unload model',
        technicalMessage: 'unloadModelById($modelId) failed: $e',
        cause: e,
      );
    }
  }

  @override
  List<RegisteredModelInfo> listLoadedModels() => rust.listLoadedModels();

  @override
  int getModelCount() {
    try {
      return rust.getModelCount().toInt();
    } catch (e) {
      debugPrint('[RustModelService] getModelCount failed: $e');
      return 0;
    }
  }

  @override
  void setModelVisible({required String modelId, required bool visible}) {
    try {
      rust.setModelVisible(modelId: modelId, visible: visible);
    } catch (e) {
      throw BimModelError(
        userMessage: 'Failed to toggle model visibility',
        technicalMessage: 'setModelVisible($modelId, $visible) failed: $e',
        cause: e,
      );
    }
  }

  @override
  void setPrimaryModel({required String modelId}) {
    try {
      rust.setPrimaryModel(modelId: modelId);
    } catch (e) {
      throw BimModelError(
        userMessage: 'Failed to set primary model',
        technicalMessage: 'setPrimaryModel($modelId) failed: $e',
        cause: e,
      );
    }
  }

  @override
  void clearAllModels() => rust.clearAllModels();

  @override
  bool isModelLoaded() => rust.isModelLoaded();

  @override
  String loadAllModelsIntoRenderer() {
    try {
      return rust.loadAllModelsIntoRenderer();
    } catch (e) {
      throw BimModelError(
        userMessage: 'Failed to load models into renderer',
        technicalMessage: 'loadAllModelsIntoRenderer failed: $e',
        cause: e,
      );
    }
  }

  @override
  ModelInfo getModelInfo() => rust.getModelInfo();
}
