import 'dart:typed_data';
import 'package:flutter_bim/src/core/bridge/api.dart' show MeasurementResult, MeasurementPoint;
import 'package:flutter_bim/src/core/bridge/bim/model.dart';
import 'package:flutter_bim/src/core/bridge/bim/model_registry.dart';
import 'package:flutter_bim/src/core/services/services.dart';

/// Mock renderer that returns a blank frame.
class MockRendererService implements IRendererService {
  bool initialized = false;
  int renderModeSetting = 0;

  @override
  Future<String> initialize({required int width, required int height}) async {
    initialized = true;
    return 'Mock renderer initialized ${width}x$height';
  }

  @override
  bool isInitialized() => initialized;

  @override
  Uint8List renderFrame() => Uint8List(480 * 360 * 4);

  @override
  void orbitCamera({required double deltaX, required double deltaY}) {}

  @override
  void zoomCamera({required double delta}) {}

  @override
  void panCamera({required double deltaX, required double deltaY}) {}

  @override
  void fitCameraToAllModels() {}

  @override
  void fitCameraToModel() {}

  @override
  void setRenderMode({required int mode}) => renderModeSetting = mode;

  @override
  int getRenderMode() => renderModeSetting;

  @override
  bool isWireframeSupported() => true;

  @override
  void setInteractionActive({required bool active}) {}

  @override
  List<double> getSceneBounds() => [-50, -50, -50, 50, 50, 50];

  @override
  void flyCamera({required double forward, required double right, required double up}) {}

  @override
  void lookCamera({required double deltaX, required double deltaY}) {}

  @override
  void setWalkthroughMode({required bool enabled}) {}

  @override
  bool setOrbitCenterFromScreen({required double screenX, required double screenY}) => false;
}

/// Mock model service with in-memory model tracking.
class MockModelService implements IModelService {
  final List<String> _loadedModelIds = [];
  bool _hasModel = false;
  int _modelCount = 0;

  @override
  Future<ModelInfo> loadIfcFile({required String filePath}) async {
    _hasModel = true;
    _modelCount++;
    return _dummyModelInfo();
  }

  @override
  Future<ModelInfo> parseIfcContent({required String content}) async {
    _hasModel = true;
    _modelCount++;
    _loadedModelIds.add('model_$_modelCount');
    return _dummyModelInfo();
  }

  @override
  void unloadModelById({required String modelId}) {
    _loadedModelIds.remove(modelId);
    _modelCount = _loadedModelIds.length;
    _hasModel = _modelCount > 0;
  }

  @override
  List<RegisteredModelInfo> listLoadedModels() => [];

  @override
  int getModelCount() => _modelCount;

  @override
  void setModelVisible({required String modelId, required bool visible}) {}

  @override
  void setPrimaryModel({required String modelId}) {}

  @override
  void clearAllModels() {
    _loadedModelIds.clear();
    _modelCount = 0;
    _hasModel = false;
  }

  @override
  bool isModelLoaded() => _hasModel;

  @override
  String loadAllModelsIntoRenderer() => 'Mock: $_modelCount models loaded';

  @override
  ModelInfo getModelInfo() => _dummyModelInfo();

  ModelInfo _dummyModelInfo() => ModelInfo(
        projectName: 'Test Project',
        buildingName: 'Test Building',
        siteName: 'Test Site',
        stats: ModelStats(
          totalEntities: BigInt.from(10),
          walls: BigInt.from(2),
          slabs: BigInt.from(1),
          columns: BigInt.from(1),
          beams: BigInt.from(1),
          doors: BigInt.from(1),
          windows: BigInt.from(1),
          storeys: BigInt.from(1),
        ),
      );
}

/// Mock selection service.
class MockSelectionService implements ISelectionService {
  int? selectedId;
  final List<ElementInfo> _elements;

  MockSelectionService({List<ElementInfo>? elements})
      : _elements = elements ?? [];

  @override
  ElementInfo? pickElement({required double screenX, required double screenY}) {
    return _elements.isNotEmpty ? _elements.first : null;
  }

  @override
  void setSelectedElement({int? elementId}) => selectedId = elementId;

  @override
  List<ElementInfo> getAllElementsFromAllModels() => _elements;

  @override
  List<ElementInfo> getAllElements() => _elements;

  @override
  String reloadAllModelsMesh() => 'Mock mesh reloaded';
}

/// Mock measurement service.
class MockMeasurementService implements IMeasurementService {
  String? activeType;
  int points = 0;

  @override
  void startMeasurement({required String measurementType}) {
    activeType = measurementType;
    points = 0;
  }

  @override
  int addMeasurementPoint({required double x, required double y, required double z}) {
    points++;
    return points;
  }

  @override
  MeasurementResult getMeasurementResult() => MeasurementResult(
        measurementType: activeType ?? 'distance',
        value: 5.0,
        unit: 'm',
        points: [
          const MeasurementPoint(x: 0, y: 0, z: 0),
          const MeasurementPoint(x: 5, y: 0, z: 0),
        ],
      );

  @override
  int getMeasurementPointCount() => points;

  @override
  void clearMeasurement() {
    activeType = null;
    points = 0;
  }
}

/// Mock section plane service.
class MockSectionPlaneService implements ISectionPlaneService {
  bool active = false;

  @override
  void setSectionPlane({
    required double originX,
    required double originY,
    required double originZ,
    required double normalX,
    required double normalY,
    required double normalZ,
  }) {
    active = true;
  }

  @override
  void setSectionPlaneEnabled({required bool enabled}) => active = enabled;

  @override
  void clearSectionPlane() => active = false;

  @override
  bool isSectionPlaneActive() => active;

  @override
  void setSectionPlaneFromAxis({required int axis, required double position}) {
    active = true;
  }
}

/// Mock overlay service.
class MockOverlayService implements IOverlayService {
  String viewMode = '3d';
  final Map<String, bool> overlays = {};

  @override
  Future<void> uploadDrawingOverlay({
    required String id,
    required int width,
    required int height,
    required List<int> rgbaPixels,
  }) async {
    overlays[id] = true;
  }

  @override
  void setOverlayTransform({
    required String id,
    required double positionX,
    required double positionY,
    required double positionZ,
    required double scaleX,
    required double scaleY,
    required double rotation,
  }) {}

  @override
  void setOverlayOpacity({required String id, required double opacity}) {}

  @override
  void setOverlayVisible({required String id, required bool visible}) {
    overlays[id] = visible;
  }

  @override
  void removeOverlay({required String id}) => overlays.remove(id);

  @override
  void setViewMode({required String mode}) => viewMode = mode;

  @override
  String getViewMode() => viewMode;
}

/// Mock lighting service.
class MockLightingService implements ILightingService {
  double lastX = 0, lastY = 0, lastZ = 0;
  double lastR = 0, lastG = 0, lastB = 0;
  double lastIntensity = 0;

  @override
  void setLightDirection({required double x, required double y, required double z}) {
    lastX = x;
    lastY = y;
    lastZ = z;
  }

  @override
  void setLightColor({required double r, required double g, required double b}) {
    lastR = r;
    lastG = g;
    lastB = b;
  }

  @override
  void setLightIntensity({required double intensity}) {
    lastIntensity = intensity;
  }

  @override
  void setAmbientColor({required double r, required double g, required double b}) {}
}

/// Mock visibility service.
class MockVisibilityService implements IVisibilityService {
  final Map<String, bool> _typeVisibility = {};
  bool _gridVisible = true;

  @override
  void setElementTypeVisible({required String elementType, required bool visible}) {
    _typeVisibility[elementType] = visible;
  }

  @override
  bool isElementTypeVisible({required String elementType}) {
    return _typeVisibility[elementType] ?? true;
  }

  @override
  List<String> getHiddenElementTypes() {
    return _typeVisibility.entries
        .where((e) => !e.value)
        .map((e) => e.key)
        .toList();
  }

  @override
  void setGridVisible({required bool visible}) => _gridVisible = visible;

  @override
  bool isGridVisible() => _gridVisible;
}
