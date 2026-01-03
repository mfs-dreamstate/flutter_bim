/// Flutter BIM - A high-performance BIM viewer for Flutter
///
/// This package provides widgets and tools for loading, visualizing,
/// and interacting with IFC (Industry Foundation Classes) files.
///
/// ## Features
/// - Load and parse IFC 2x3 and IFC 4 files
/// - High-performance 3D rendering with wgpu (Vulkan/Metal)
/// - Element selection and property inspection
/// - Multi-model support
/// - GIS integration with OpenStreetMap
/// - Measurement and analysis tools
///
/// ## Usage
/// ```dart
/// import 'package:flutter_bim/flutter_bim.dart';
///
/// BimViewer(
///   onModelLoaded: (modelInfo) {
///     print('Loaded: ${modelInfo.projectName}');
///   },
/// )
/// ```
library flutter_bim;

// Core widgets - Main functionality
export 'src/widgets/bim_viewer.dart' show ViewerScreen;
export 'src/widgets/element_tree.dart' show ElementTreeDrawer;
export 'src/widgets/properties_panel.dart' show showPropertiesPanel;
export 'src/widgets/model_manager.dart' show ModelManagerDrawer;

// Optional widgets - Additional features
export 'src/widgets/map_view.dart' show MapViewScreen, GeoReference;
export 'src/widgets/measurement_tools.dart' show showMeasurementTools;
export 'src/widgets/section_plane_tools.dart' show showSectionPlaneTools;
export 'src/widgets/grid_overlay.dart' show GridOverlay;
export 'src/widgets/drawing_overlay_manager.dart' show showDrawingOverlayManager;

// Data models - User-facing data structures
export 'src/core/bridge/bim/model.dart' show ModelInfo, ModelStats, RegisteredModelInfo;
export 'src/core/bridge/bim/entities.dart' show ElementInfo;
export 'src/core/bridge/bim/geometry.dart' show Mesh, GeometryData;

// Core API - For advanced users who want direct access
export 'src/core/bridge/api.dart';
export 'src/core/bridge/frb_generated.dart' show RustLib;
