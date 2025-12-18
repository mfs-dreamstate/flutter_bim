# Rust API Design for Flutter BIM Viewer

## Overview

This document defines the API surface exposed from Rust to Flutter via Flutter Rust Bridge. The API is designed to be:
- **Type-safe**: Strong typing across FFI boundary
- **Async-friendly**: Non-blocking operations for heavy tasks
- **Efficient**: Minimal data transfer across FFI
- **Error-handled**: Comprehensive error propagation

## Core API Structure

### Main API Entry Point

```rust
// rust/src/api.rs

use flutter_rust_bridge::frb;

/// Main BIM viewer API - singleton instance
pub struct BimViewerApi {
    inner: Arc<Mutex<BimViewerState>>,
}

impl BimViewerApi {
    #[frb(sync)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(BimViewerState::new())),
        }
    }
}
```

## Data Types

### Core Data Models

```rust
// Shared between Rust and Dart via FRB

/// 3D Vector
#[derive(Clone, Debug)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// 3D Bounding box
#[derive(Clone, Debug)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

/// Camera configuration
#[derive(Clone, Debug)]
pub struct CameraConfig {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

/// Viewport dimensions
#[derive(Clone, Debug)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
    pub pixel_ratio: f32,
}

/// Color representation
#[derive(Clone, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// Material properties
#[derive(Clone, Debug)]
pub struct Material {
    pub name: String,
    pub base_color: Color,
    pub metallic: f32,
    pub roughness: f32,
    pub opacity: f32,
}

/// Element type enumeration
#[derive(Clone, Debug)]
pub enum ElementType {
    Wall,
    Slab,
    Beam,
    Column,
    Window,
    Door,
    Roof,
    Stair,
    Railing,
    Furniture,
    Unknown(String),
}

/// Property value
#[derive(Clone, Debug)]
pub enum PropertyValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Vector(Vec3),
}

/// Element property
#[derive(Clone, Debug)]
pub struct Property {
    pub name: String,
    pub value: PropertyValue,
    pub property_set: String,
}

/// BIM element info
#[derive(Clone, Debug)]
pub struct ElementInfo {
    pub id: String,
    pub name: String,
    pub element_type: ElementType,
    pub bounds: BoundingBox,
    pub material: Option<Material>,
    pub layer: String,
}

/// Model metadata
#[derive(Clone, Debug)]
pub struct ModelMetadata {
    pub name: String,
    pub file_path: String,
    pub file_size: u64,
    pub element_count: usize,
    pub bounds: BoundingBox,
    pub units: String,
    pub schema: String,
}

/// Loading progress
#[derive(Clone, Debug)]
pub struct LoadProgress {
    pub stage: LoadingStage,
    pub progress: f32,
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum LoadingStage {
    Reading,
    Parsing,
    BuildingGeometry,
    OptimizingScene,
    Complete,
}

/// Selection result
#[derive(Clone, Debug)]
pub struct SelectionResult {
    pub element_id: String,
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
}

/// Render statistics
#[derive(Clone, Debug)]
pub struct RenderStats {
    pub frame_time_ms: f32,
    pub triangle_count: usize,
    pub draw_calls: usize,
    pub visible_elements: usize,
}
```

## API Methods

### 1. Initialization & Configuration

```rust
impl BimViewerApi {
    /// Initialize the viewer with configuration
    #[frb(sync)]
    pub fn initialize(&self, config: ViewerConfig) -> Result<(), BimError> {
        // Initialize graphics backend, set up rendering pipeline
    }

    /// Get viewer version
    #[frb(sync)]
    pub fn get_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Check if viewer is initialized
    #[frb(sync)]
    pub fn is_initialized(&self) -> bool {
        // Check initialization state
    }

    /// Shutdown and cleanup resources
    pub async fn shutdown(&self) -> Result<(), BimError> {
        // Clean up GPU resources, close files
    }
}

#[derive(Clone, Debug)]
pub struct ViewerConfig {
    pub enable_shadows: bool,
    pub enable_ambient_occlusion: bool,
    pub antialiasing: AntialiasingMode,
    pub max_texture_size: u32,
}

#[derive(Clone, Debug)]
pub enum AntialiasingMode {
    None,
    FXAA,
    MSAA2x,
    MSAA4x,
    MSAA8x,
}
```

### 2. Model Loading

```rust
impl BimViewerApi {
    /// Load a BIM model from file path
    pub async fn load_model(&self, path: String) -> Result<ModelMetadata, BimError> {
        // Parse IFC file, build scene graph, return metadata
    }

    /// Get loading progress (call periodically during load)
    #[frb(sync)]
    pub fn get_load_progress(&self) -> Option<LoadProgress> {
        // Return current loading progress
    }

    /// Unload current model
    pub async fn unload_model(&self) -> Result<(), BimError> {
        // Clear scene, free memory
    }

    /// Get current model metadata
    #[frb(sync)]
    pub fn get_model_metadata(&self) -> Option<ModelMetadata> {
        // Return loaded model metadata
    }
}
```

### 3. Camera Control

```rust
impl BimViewerApi {
    /// Set camera configuration
    #[frb(sync)]
    pub fn set_camera(&self, camera: CameraConfig) -> Result<(), BimError> {
        // Update camera parameters
    }

    /// Get current camera configuration
    #[frb(sync)]
    pub fn get_camera(&self) -> Option<CameraConfig> {
        // Return current camera state
    }

    /// Fit camera to show entire model
    #[frb(sync)]
    pub fn fit_to_view(&self) -> Result<CameraConfig, BimError> {
        // Calculate camera position to show all geometry
    }

    /// Orbit camera around target
    #[frb(sync)]
    pub fn orbit_camera(&self, delta_x: f32, delta_y: f32) -> Result<CameraConfig, BimError> {
        // Rotate camera around target point
    }

    /// Pan camera
    #[frb(sync)]
    pub fn pan_camera(&self, delta_x: f32, delta_y: f32) -> Result<CameraConfig, BimError> {
        // Move camera and target in screen space
    }

    /// Zoom camera
    #[frb(sync)]
    pub fn zoom_camera(&self, delta: f32) -> Result<CameraConfig, BimError> {
        // Move camera closer/farther from target
    }
}
```

### 4. Rendering

```rust
impl BimViewerApi {
    /// Render a frame and return pixel data
    pub async fn render_frame(&self, viewport: Viewport) -> Result<Vec<u8>, BimError> {
        // Render scene to texture, return RGBA pixel data
    }

    /// Get render statistics for last frame
    #[frb(sync)]
    pub fn get_render_stats(&self) -> RenderStats {
        // Return performance metrics
    }

    /// Update viewport size
    #[frb(sync)]
    pub fn set_viewport(&self, viewport: Viewport) -> Result<(), BimError> {
        // Resize rendering surface
    }
}
```

### 5. Element Selection & Query

```rust
impl BimViewerApi {
    /// Select element at screen coordinates
    #[frb(sync)]
    pub fn select_at_point(&self, x: f32, y: f32) -> Option<SelectionResult> {
        // Raycast and return nearest element
    }

    /// Get element information by ID
    #[frb(sync)]
    pub fn get_element_info(&self, id: String) -> Option<ElementInfo> {
        // Return element metadata
    }

    /// Get element properties
    #[frb(sync)]
    pub fn get_element_properties(&self, id: String) -> Vec<Property> {
        // Return all properties for element
    }

    /// Get all elements of specific type
    #[frb(sync)]
    pub fn get_elements_by_type(&self, element_type: ElementType) -> Vec<String> {
        // Return element IDs matching type
    }

    /// Search elements by name
    #[frb(sync)]
    pub fn search_elements(&self, query: String) -> Vec<ElementInfo> {
        // Full-text search on element names
    }
}
```

### 6. Visibility & Filtering

```rust
impl BimViewerApi {
    /// Set element visibility
    #[frb(sync)]
    pub fn set_element_visible(&self, id: String, visible: bool) -> Result<(), BimError> {
        // Show/hide specific element
    }

    /// Set multiple elements visibility
    #[frb(sync)]
    pub fn set_elements_visible(&self, ids: Vec<String>, visible: bool) -> Result<(), BimError> {
        // Batch visibility update
    }

    /// Hide all elements except specified
    #[frb(sync)]
    pub fn isolate_elements(&self, ids: Vec<String>) -> Result<(), BimError> {
        // Show only specified elements
    }

    /// Show all elements
    #[frb(sync)]
    pub fn show_all(&self) -> Result<(), BimError> {
        // Reset all visibility
    }

    /// Get list of layers
    #[frb(sync)]
    pub fn get_layers(&self) -> Vec<String> {
        // Return all unique layer names
    }

    /// Set layer visibility
    #[frb(sync)]
    pub fn set_layer_visible(&self, layer: String, visible: bool) -> Result<(), BimError> {
        // Show/hide all elements on layer
    }
}
```

### 7. Visual Style & Rendering Options

```rust
impl BimViewerApi {
    /// Set rendering mode
    #[frb(sync)]
    pub fn set_render_mode(&self, mode: RenderMode) -> Result<(), BimError> {
        // Change visual style
    }

    /// Set element color override
    #[frb(sync)]
    pub fn set_element_color(&self, id: String, color: Color) -> Result<(), BimError> {
        // Override element color
    }

    /// Clear element color override
    #[frb(sync)]
    pub fn clear_element_color(&self, id: String) -> Result<(), BimError> {
        // Restore original color
    }

    /// Set background color
    #[frb(sync)]
    pub fn set_background_color(&self, color: Color) -> Result<(), BimError> {
        // Change background
    }
}

#[derive(Clone, Debug)]
pub enum RenderMode {
    Shaded,          // Full materials and lighting
    Wireframe,       // Edges only
    HiddenLine,      // Wireframe with hidden line removal
    Flat,            // Flat shading
    Realistic,       // PBR materials
    Conceptual,      // Simple colors
}
```

### 8. Measurements & Analysis

```rust
impl BimViewerApi {
    /// Measure distance between two points
    #[frb(sync)]
    pub fn measure_distance(&self, start: Vec3, end: Vec3) -> f32 {
        // Calculate Euclidean distance
    }

    /// Get element volume
    #[frb(sync)]
    pub fn get_element_volume(&self, id: String) -> Option<f32> {
        // Calculate volume if available
    }

    /// Get element surface area
    #[frb(sync)]
    pub fn get_element_area(&self, id: String) -> Option<f32> {
        // Calculate surface area
    }

    /// Create section plane
    #[frb(sync)]
    pub fn create_section(&self, origin: Vec3, normal: Vec3) -> Result<String, BimError> {
        // Create cutting plane, return section ID
    }

    /// Remove section plane
    #[frb(sync)]
    pub fn remove_section(&self, id: String) -> Result<(), BimError> {
        // Remove section by ID
    }
}
```

### 9. GIS & Georeferencing

```rust
impl BimViewerApi {
    /// Get geographic location of the model
    #[frb(sync)]
    pub fn get_geo_location(&self) -> Option<GeoLocation> {
        // Extract from IfcSite, IfcMapConversion
    }

    /// Get building footprint in geographic coordinates
    #[frb(sync)]
    pub fn get_building_footprint(&self) -> Option<GeoFootprint> {
        // Calculate footprint polygon in lat/lon
    }

    /// Convert local IFC coordinates to geographic coordinates
    #[frb(sync)]
    pub fn local_to_geo(&self, point: Vec3) -> Option<GeoPoint> {
        // Apply coordinate transformation
    }

    /// Convert geographic coordinates to local IFC coordinates
    #[frb(sync)]
    pub fn geo_to_local(&self, geo_point: GeoPoint) -> Option<Vec3> {
        // Inverse coordinate transformation
    }
}

#[derive(Clone, Debug)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f32,
    pub true_north: f32,  // Rotation from project north to true north (degrees)
    pub coordinate_system: String,  // e.g., "EPSG:4326"
}

#[derive(Clone, Debug)]
pub struct GeoPoint {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Clone, Debug)]
pub struct GeoFootprint {
    pub points: Vec<GeoPoint>,  // Polygon vertices
    pub bounds: GeoBounds,
}

#[derive(Clone, Debug)]
pub struct GeoBounds {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lon: f64,
    pub max_lon: f64,
}
```

### 10. Export & Utilities

```rust
impl BimViewerApi {
    /// Export current view as image
    pub async fn export_image(&self, path: String, width: u32, height: u32) -> Result<(), BimError> {
        // Render at specified size and save to file
    }

    /// Get system info
    #[frb(sync)]
    pub fn get_system_info(&self) -> SystemInfo {
        // Return GPU/system information
    }
}

#[derive(Clone, Debug)]
pub struct SystemInfo {
    pub gpu_name: String,
    pub gpu_vendor: String,
    pub max_texture_size: u32,
    pub backend: String,  // "Vulkan", "Metal", "DX12", etc.
}
```

## Error Handling

```rust
// rust/src/error.rs

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum BimError {
    #[error("Model not loaded")]
    NoModelLoaded,

    #[error("Failed to parse IFC file: {0}")]
    ParseError(String),

    #[error("Invalid file format")]
    InvalidFormat,

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Rendering error: {0}")]
    RenderError(String),

    #[error("Invalid element ID: {0}")]
    InvalidElementId(String),

    #[error("Not initialized")]
    NotInitialized,

    #[error("Graphics device error: {0}")]
    GraphicsError(String),

    #[error("Invalid camera configuration: {0}")]
    InvalidCamera(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, BimError>;
```

## Usage Examples from Flutter

### Initialize and Load Model

```dart
// Initialize viewer
final api = BimViewerApi();
await api.initialize(ViewerConfig(
  enableShadows: true,
  enableAmbientOcclusion: true,
  antialiasing: AntialiasingMode.msaa4x,
  maxTextureSize: 4096,
));

// Load model
try {
  final metadata = await api.loadModel('/path/to/model.ifc');
  print('Loaded: ${metadata.name}');
  print('Elements: ${metadata.elementCount}');

  // Fit camera to view
  final camera = api.fitToView();
  api.setCamera(camera);
} catch (e) {
  print('Error loading model: $e');
}
```

### Render Loop

```dart
class ViewerWidget extends StatefulWidget {
  @override
  State<ViewerWidget> createState() => _ViewerWidgetState();
}

class _ViewerWidgetState extends State<ViewerWidget> {
  late BimViewerApi api;
  Uint8List? frameData;

  @override
  void initState() {
    super.initState();
    api = BimViewerApi();
    _renderFrame();
  }

  Future<void> _renderFrame() async {
    final viewport = Viewport(
      width: MediaQuery.of(context).size.width.toInt(),
      height: MediaQuery.of(context).size.height.toInt(),
      pixelRatio: MediaQuery.of(context).devicePixelRatio,
    );

    final pixels = await api.renderFrame(viewport);
    setState(() {
      frameData = Uint8List.fromList(pixels);
    });
  }

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onPanUpdate: (details) {
        api.panCamera(details.delta.dx, details.delta.dy);
        _renderFrame();
      },
      child: frameData != null
          ? Image.memory(frameData!)
          : CircularProgressIndicator(),
    );
  }
}
```

### Element Selection

```dart
void _handleTap(TapDownDetails details) {
  final result = api.selectAtPoint(
    details.localPosition.dx,
    details.localPosition.dy,
  );

  if (result != null) {
    final info = api.getElementInfo(result.elementId);
    final properties = api.getElementProperties(result.elementId);

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(info!.name),
        content: Column(
          children: properties.map((p) =>
            Text('${p.name}: ${p.value}')
          ).toList(),
        ),
      ),
    );
  }
}
```

### 2D Map View Integration

```dart
import 'package:flutter_map/flutter_map.dart';
import 'package:latlong2/latlong.dart';

class MapViewWidget extends StatefulWidget {
  final BimViewerApi api;

  const MapViewWidget({required this.api});

  @override
  State<MapViewWidget> createState() => _MapViewWidgetState();
}

class _MapViewWidgetState extends State<MapViewWidget> {
  GeoLocation? geoLocation;
  GeoFootprint? footprint;

  @override
  void initState() {
    super.initState();
    _loadGeoData();
  }

  void _loadGeoData() {
    geoLocation = widget.api.getGeoLocation();
    footprint = widget.api.getBuildingFootprint();
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    if (geoLocation == null) {
      return Center(child: Text('No geographic data available'));
    }

    return FlutterMap(
      options: MapOptions(
        initialCenter: LatLng(geoLocation!.latitude, geoLocation!.longitude),
        initialZoom: 16.0,
      ),
      children: [
        // OpenStreetMap tiles
        TileLayer(
          urlTemplate: 'https://tile.openstreetmap.org/{z}/{x}/{y}.png',
          userAgentPackageName: 'com.yourcompany.bim_viewer',
        ),

        // Building footprint polygon
        if (footprint != null)
          PolygonLayer(
            polygons: [
              Polygon(
                points: footprint!.points.map((p) =>
                  LatLng(p.latitude, p.longitude)
                ).toList(),
                color: Colors.blue.withOpacity(0.3),
                borderColor: Colors.blue,
                borderStrokeWidth: 2,
              ),
            ],
          ),

        // Building center marker
        MarkerLayer(
          markers: [
            Marker(
              point: LatLng(geoLocation!.latitude, geoLocation!.longitude),
              width: 80,
              height: 80,
              child: Icon(
                Icons.location_pin,
                color: Colors.red,
                size: 40,
              ),
            ),
          ],
        ),
      ],
    );
  }
}
```

## Performance Considerations

### Batching

When performing multiple operations, batch them:

```rust
// Instead of multiple FFI calls
for id in element_ids {
    api.set_element_visible(id, false); // Bad: N FFI calls
}

// Use batch operation
api.set_elements_visible(element_ids, false); // Good: 1 FFI call
```

### Async Operations

Long-running operations are async to prevent blocking:

```rust
// Heavy operations don't block UI
pub async fn load_model(&self, path: String) -> Result<ModelMetadata, BimError>
pub async fn render_frame(&self, viewport: Viewport) -> Result<Vec<u8>, BimError>
```

### Shared Memory (Future Enhancement)

For high-frequency rendering, consider shared memory:

```rust
pub fn get_frame_buffer_ptr(&self) -> *const u8 {
    // Return pointer to shared memory region
    // Flutter reads directly without copying
}
```

## Testing API

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_creation() {
        let api = BimViewerApi::new();
        assert!(!api.is_initialized());
    }

    #[tokio::test]
    async fn test_load_model() {
        let api = BimViewerApi::new();
        let result = api.load_model("test.ifc".to_string()).await;
        assert!(result.is_ok());
    }
}
```

---

This API provides a clean, type-safe interface between Flutter and Rust, enabling efficient BIM viewing with high performance.
