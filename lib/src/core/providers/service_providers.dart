import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../services/services.dart';

/// Provider for the renderer service.
final rendererServiceProvider = Provider<IRendererService>((ref) {
  return RustRendererService();
});

/// Provider for the model service.
final modelServiceProvider = Provider<IModelService>((ref) {
  return RustModelService();
});

/// Provider for the selection service.
final selectionServiceProvider = Provider<ISelectionService>((ref) {
  return RustSelectionService();
});

/// Provider for the measurement service.
final measurementServiceProvider = Provider<IMeasurementService>((ref) {
  return RustMeasurementService();
});

/// Provider for the section plane service.
final sectionPlaneServiceProvider = Provider<ISectionPlaneService>((ref) {
  return RustSectionPlaneService();
});

/// Provider for the overlay service.
final overlayServiceProvider = Provider<IOverlayService>((ref) {
  return RustOverlayService();
});

/// Provider for the lighting service.
final lightingServiceProvider = Provider<ILightingService>((ref) {
  return RustLightingService();
});

/// Provider for the visibility service.
final visibilityServiceProvider = Provider<IVisibilityService>((ref) {
  return RustVisibilityService();
});
