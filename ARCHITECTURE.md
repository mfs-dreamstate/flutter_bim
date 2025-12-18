# Technical Architecture - Flutter Rust BIM Viewer

## System Architecture

### Component Overview

```
┌────────────────────────────────────────────────────────────┐
│                     Flutter Application                     │
├────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Presentation│  │    Domain    │  │     Data     │     │
│  │    Layer    │  │    Layer     │  │    Layer     │     │
│  └──────┬──────┘  └──────┬───────┘  └──────┬───────┘     │
│         │                 │                  │              │
│         └────────┬────────┴────────┬─────────┘             │
│                  │                 │                        │
│         ┌────────▼─────────────────▼────────┐             │
│         │   Flutter Rust Bridge API Layer   │             │
│         └────────────────┬──────────────────┘             │
└──────────────────────────┼──────────────────────────────┘
                           │ FFI Boundary
┌──────────────────────────▼──────────────────────────────┐
│                     Rust Core Library                     │
├───────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │     BIM     │  │   Renderer   │  │   Graphics   │    │
│  │   Parser    │  │    Engine    │  │    Backend   │    │
│  └──────┬──────┘  └──────┬───────┘  └──────┬───────┘    │
│         │                 │                  │             │
│         └────────┬────────┴────────┬─────────┘            │
│                  │                 │                       │
│         ┌────────▼─────────────────▼────────┐            │
│         │      Core Utilities & Types       │            │
│         └───────────────────────────────────┘            │
└───────────────────────────────────────────────────────────┘
```

## Flutter Layer Architecture

### 1. Presentation Layer

**Responsibilities**
- UI rendering and user interaction
- State management
- Navigation
- Animation

**Key Components**
```
lib/presentation/
├── viewer/
│   ├── viewer_screen.dart          # Main viewer page
│   ├── viewer_controller.dart      # State management
│   └── widgets/
│       ├── canvas_widget.dart      # Native rendering surface
│       ├── toolbar_widget.dart     # Action toolbar
│       ├── properties_panel.dart   # Element properties
│       └── navigation_cube.dart    # 3D navigation helper
├── file_browser/
│   ├── file_browser_screen.dart
│   └── file_list_widget.dart
└── settings/
    └── settings_screen.dart
```

### 2. Domain Layer

**Responsibilities**
- Business logic
- Use cases
- Domain models
- Repository interfaces

**Key Components**
```
lib/domain/
├── models/
│   ├── bim_model.dart             # BIM model representation
│   ├── element.dart               # BIM element
│   ├── property.dart              # Element property
│   ├── camera_state.dart          # Camera position/orientation
│   └── render_settings.dart       # Rendering configuration
├── repositories/
│   ├── bim_repository.dart        # Abstract BIM operations
│   └── settings_repository.dart   # Abstract settings operations
└── usecases/
    ├── load_model.dart            # Load BIM model use case
    ├── select_element.dart        # Element selection use case
    └── update_camera.dart         # Camera manipulation use case
```

### 3. Data Layer

**Responsibilities**
- Rust bridge implementation
- Data persistence
- Caching

**Key Components**
```
lib/data/
├── repositories/
│   ├── bim_repository_impl.dart   # Concrete implementation
│   └── settings_repository_impl.dart
├── datasources/
│   ├── rust_bridge_datasource.dart # FRB wrapper
│   └── local_storage_datasource.dart
└── mappers/
    └── model_mapper.dart          # Rust ↔ Dart mapping
```

## Rust Layer Architecture

### 1. API Layer (Flutter Interface)

**File**: `rust/src/api.rs`

```rust
// Public API exposed to Flutter via FRB

pub struct BimViewerApi {
    viewer: Arc<Mutex<BimViewer>>,
}

impl BimViewerApi {
    pub fn new() -> Self { }

    pub async fn load_model(&self, path: String) -> Result<ModelInfo> { }

    pub fn render_frame(&self, viewport: Viewport) -> Result<FrameData> { }

    pub fn update_camera(&self, transform: CameraTransform) -> Result<()> { }

    pub fn select_element(&self, ray: Ray) -> Result<Option<ElementInfo>> { }

    pub fn get_element_properties(&self, id: String) -> Result<Vec<Property>> { }
}
```

### 2. BIM Processing Layer

**File**: `rust/src/bim/`

**Components**
- `ifc_parser.rs`: IFC file parsing
- `model.rs`: In-memory model representation
- `geometry.rs`: Geometry extraction and processing
- `spatial_index.rs`: Spatial indexing for selection

**Architecture**
```rust
// Model representation
pub struct BimModel {
    elements: HashMap<String, Element>,
    spatial_index: RTree<ElementBounds>,
    metadata: ModelMetadata,
}

// IfcOpenShell Wrapper
// Using IfcOpenShell (C++) via FFI for robust IFC parsing
pub struct IfcModelLoader {
    ifc_file: Option<UniquePtr<ffi::IfcFile>>,  // C++ object via cxx
}

impl IfcModelLoader {
    pub fn load_file(&mut self, path: &Path) -> Result<BimModel> {
        // Load IFC via IfcOpenShell C++ library
        // Extract geometry using OpenCASCADE
        // Build Rust-native BimModel
    }

    pub fn extract_geometry(&self) -> Vec<MeshData> {
        // Use IfcOpenShell's optimized geometry extraction
    }
}

// Note: Uses IfcOpenShell for 10x better performance and reliability
// compared to custom parser. Handles all IFC edge cases.
```

### 3. Rendering Layer

**File**: `rust/src/renderer/`

**Components**
- `engine.rs`: Main rendering engine
- `camera.rs`: Camera implementation
- `scene.rs`: Scene graph
- `mesh.rs`: Mesh data structures
- `material.rs`: Material system
- `pipeline.rs`: Render pipeline

**Architecture**
```rust
pub struct RenderEngine {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: RenderPipeline,
    scene: Scene,
}

pub struct Scene {
    meshes: Vec<Mesh>,
    camera: Camera,
    lights: Vec<Light>,
}

impl RenderEngine {
    pub fn new(config: RenderConfig) -> Result<Self> { }
    pub fn render(&mut self, viewport: Viewport) -> Result<Vec<u8>> { }
    pub fn update_scene(&mut self, scene: Scene) { }
}
```

## Data Flow

### Loading a BIM Model

```
User selects file
       │
       ▼
[Flutter] File picker
       │
       ▼
[Flutter] Load model use case
       │
       ▼
[FRB] load_model(path)
       │
       ▼
[Rust] IfcModelLoader::load_file()
       │
       ▼
[C++ FFI] IfcOpenShell::load()
       │
       ├─► Parse IFC file (battle-tested parser)
       ├─► Extract geometry via OpenCASCADE
       └─► Return geometry data
       │
       ▼
[Rust] Process geometry
       │
       ├─► Convert to Rust mesh structures
       ├─► Build spatial index (R-tree)
       └─► Create BimModel
       │
       ▼
[Rust] Store in BimViewer
       │
       ▼
[FRB] Return ModelInfo
       │
       ▼
[Flutter] Update UI state
       │
       ▼
Display model info
```

### Rendering a Frame

```
Flutter rebuild triggered
       │
       ▼
[Flutter] Canvas widget build
       │
       ▼
[FRB] render_frame(viewport)
       │
       ▼
[Rust] RenderEngine::render()
       │
       ├─► Update camera matrices
       ├─► Cull invisible objects
       ├─► Sort by material
       ├─► Execute draw calls
       └─► Read pixel data
       │
       ▼
[FRB] Return frame bytes
       │
       ▼
[Flutter] Update texture
       │
       ▼
Display on screen
```

### Element Selection

```
User taps screen
       │
       ▼
[Flutter] GestureDetector
       │
       ▼
[Flutter] Convert to world ray
       │
       ▼
[FRB] select_element(ray)
       │
       ▼
[Rust] SpatialIndex::query(ray)
       │
       ├─► Find intersecting bounds
       ├─► Detailed ray-mesh test
       └─► Return nearest element
       │
       ▼
[FRB] Return ElementInfo
       │
       ▼
[Flutter] Show properties panel
       │
       ▼
Display element details
```

## Threading Model

### Flutter Side (Dart)

- **UI Thread**: Handles all UI rendering and user interaction
- **Isolates**: For CPU-intensive Dart operations (if needed)
- **FRB calls**: Automatically handled via FFI

### Rust Side

- **Main Thread**: Handles synchronous FFI calls
- **Tokio Runtime**: For async operations
  - File I/O
  - Model parsing
  - Heavy computations
- **Rendering Thread**: Dedicated thread for GPU operations

**Example Threading**
```rust
pub struct BimViewer {
    model: Arc<Mutex<Option<BimModel>>>,
    renderer: Arc<Mutex<RenderEngine>>,
    runtime: tokio::Runtime,
}

impl BimViewer {
    pub async fn load_model(&self, path: String) -> Result<ModelInfo> {
        // This runs on tokio runtime
        let model = tokio::task::spawn_blocking(move || {
            IfcParser::parse_file(&path)
        }).await??;

        *self.model.lock().unwrap() = Some(model);
        Ok(model_info)
    }
}
```

## Memory Management

### Shared Data

**Problem**: Large geometry data needs to be accessed by both Rust and Flutter

**Solution**: Use shared memory approach
```rust
// Rust side: Pin memory and share pointer
pub fn get_vertex_buffer(&self) -> *const u8 {
    self.vertices.as_ptr()
}

// Flutter side: Use Pointer and asTypedList
final ptr = api.getVertexBuffer();
final vertices = ptr.asTypedList(vertexCount);
```

### Ownership Rules

1. **Rust owns model data**: Flutter holds only IDs/references
2. **Flutter owns UI state**: Rust doesn't track UI concerns
3. **Shared camera state**: Synchronized via explicit calls
4. **Texture data**: Rust renders, Flutter displays

## Error Handling

### Rust Layer

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BimError {
    #[error("Failed to parse IFC file: {0}")]
    ParseError(String),

    #[error("Rendering error: {0}")]
    RenderError(String),

    #[error("Invalid element ID: {0}")]
    InvalidElementId(String),
}

pub type Result<T> = std::result::Result<T, BimError>;
```

### Flutter Layer

```dart
class BimException implements Exception {
  final String message;
  final BimErrorType type;

  BimException(this.message, this.type);
}

enum BimErrorType {
  parsing,
  rendering,
  fileAccess,
  unknown,
}
```

### Error Propagation

```
[Rust] Error occurs
    │
    ▼
[Rust] Convert to BimError
    │
    ▼
[FRB] Serialize error
    │
    ▼
[Flutter] Deserialize to exception
    │
    ▼
[Flutter] Handle in UI
    │
    ▼
Show error dialog
```

## Performance Optimization Strategies

### 1. Minimize FFI Calls
- Batch operations where possible
- Use shared memory for large data
- Cache results on Flutter side

### 2. Spatial Indexing
- R-tree for element lookup
- Frustum culling
- Level-of-detail (LOD) system

### 3. Rendering Optimization
- Instanced rendering for repeated geometry
- Material batching
- Texture atlasing
- Occlusion culling

### 4. Memory Optimization
- Stream large files
- Unload invisible geometry
- Compress texture data
- Use vertex buffer sharing

### 5. Async Loading
- Progressive model loading
- Background parsing
- Lazy property loading

## Platform-Specific Considerations

### Android
- Use GLES 3.0+ or Vulkan via wgpu
- Handle activity lifecycle
- Manage memory constraints on lower-end devices

### iOS
- Use Metal via wgpu
- Handle app state transitions
- Optimize for various screen sizes

### Desktop (Windows/macOS/Linux)
- Use Vulkan/Metal/DX12 via wgpu
- Larger viewport sizes
- Higher performance expectations
- File system access patterns

## Security Architecture

### File Access
- Validate file paths
- Sandbox file operations
- Limit file size (configurable)

### Input Validation
- Validate IFC file structure
- Limit recursion depth
- Prevent buffer overflows
- Sanitize user input

### Error Information
- Don't leak path information in errors
- Sanitize error messages for users
- Log detailed errors securely

## Testability

### Unit Testing (Rust)
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_ifc_parser() {
        let result = IfcParser::parse_stream(test_data);
        assert!(result.is_ok());
    }
}
```

### Widget Testing (Flutter)
```dart
testWidgets('Viewer widget displays model', (tester) async {
  await tester.pumpWidget(ViewerWidget());
  expect(find.byType(Canvas), findsOneWidget);
});
```

### Integration Testing
- Mock Rust API for Flutter tests
- Test FFI boundary explicitly
- End-to-end workflow tests

## Monitoring and Logging

### Performance Metrics
- Frame time (target: < 16ms)
- Model load time
- Memory usage
- FFI call overhead

### Logging Strategy
```rust
// Rust: Use tracing
use tracing::{info, warn, error};

info!("Loading model: {}", path);
warn!("Large model detected: {} MB", size);
error!("Parse error: {}", err);
```

```dart
// Flutter: Use logger
final logger = Logger('BimViewer');

logger.info('Model loaded successfully');
logger.warning('Performance degradation detected');
logger.severe('Failed to render frame', error, stackTrace);
```

## Configuration Management

### Build-time Configuration
- Debug vs Release builds
- Platform-specific features
- Graphics backend selection

### Runtime Configuration
- Rendering quality settings
- Cache sizes
- Performance profiles
- User preferences

---

This architecture provides a solid foundation for building a high-performance, cross-platform BIM viewer with clear separation of concerns and optimal performance characteristics.
