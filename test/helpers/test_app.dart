import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_bim/src/core/providers/service_providers.dart';
import '../mocks/mock_services.dart';

/// Create a test app wrapped in ProviderScope with mock overrides.
Widget createTestApp({
  required Widget child,
  MockRendererService? renderer,
  MockModelService? model,
  MockSelectionService? selection,
  MockMeasurementService? measurement,
  MockSectionPlaneService? sectionPlane,
  MockOverlayService? overlay,
  MockLightingService? lighting,
  MockVisibilityService? visibility,
}) {
  return ProviderScope(
    overrides: [
      if (renderer != null)
        rendererServiceProvider.overrideWithValue(renderer),
      if (model != null)
        modelServiceProvider.overrideWithValue(model),
      if (selection != null)
        selectionServiceProvider.overrideWithValue(selection),
      if (measurement != null)
        measurementServiceProvider.overrideWithValue(measurement),
      if (sectionPlane != null)
        sectionPlaneServiceProvider.overrideWithValue(sectionPlane),
      if (overlay != null)
        overlayServiceProvider.overrideWithValue(overlay),
      if (lighting != null)
        lightingServiceProvider.overrideWithValue(lighting),
      if (visibility != null)
        visibilityServiceProvider.overrideWithValue(visibility),
    ],
    child: MaterialApp(
      home: child,
    ),
  );
}

/// Create a fully-mocked test app with all services mocked.
Widget createFullyMockedTestApp({
  required Widget child,
}) {
  return createTestApp(
    child: child,
    renderer: MockRendererService(),
    model: MockModelService(),
    selection: MockSelectionService(),
    measurement: MockMeasurementService(),
    sectionPlane: MockSectionPlaneService(),
    overlay: MockOverlayService(),
    lighting: MockLightingService(),
    visibility: MockVisibilityService(),
  );
}
