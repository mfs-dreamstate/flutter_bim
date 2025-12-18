# Flutter BIM Viewer with Rust Backend - Project Plan

## Project Overview

A cross-platform BIM (Building Information Modeling) viewer application built with Flutter for the UI and Rust for high-performance 3D rendering and BIM data processing, connected via Flutter Rust Bridge (FRB).

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────┐
│         Flutter UI Layer            │
│  - Material/Cupertino Widgets       │
│  - State Management (Riverpod/Bloc) │
│  - User Interactions                │
└─────────────┬───────────────────────┘
              │
              │ Flutter Rust Bridge (FFI)
              │
┌─────────────▼───────────────────────┐
│         Rust Core Layer             │
│  - BIM File Parsing (IFC)           │
│  - 3D Rendering Engine              │
│  - Geometry Processing              │
│  - Performance-Critical Operations  │
└─────────────────────────────────────┘
```

### Technology Stack

**Frontend (Flutter)**
- Flutter SDK (latest stable)
- Dart 3.x
- State Management: Riverpod or Bloc
- UI: Material Design 3

**Backend (Rust)**
- Rust 1.75+
- Graphics: wgpu or three-d
- BIM Parsing: Custom IFC parser or existing crates
- Linear Algebra: nalgebra or glam

**Bridge**
- flutter_rust_bridge: Latest version
- FFI bindings for seamless Rust ↔ Dart communication

## Core Features

### Phase 1: Foundation
1. Load and parse IFC files (Industry Foundation Classes)
2. Basic 3D model visualization
3. Camera controls (pan, zoom, rotate)
4. Basic model navigation

### Phase 2: Visualization
1. Material and texture rendering
2. Lighting system
3. Wireframe/solid toggle
4. Section views
5. Measurement tools

### Phase 3: BIM Features
1. Element selection and inspection
2. Property display
3. Layer/category filtering
4. Search functionality
5. Element highlighting

### Phase 4: Advanced Features
1. Clash detection
2. Annotations
3. Export capabilities
4. Model comparison
5. Performance optimization for large models

### Phase 5: 2D GIS Integration
1. Map view integration (OpenStreetMap/Google Maps)
2. IFC georeferencing extraction
3. Building footprint overlay on map
4. Site context visualization
5. Coordinate system conversion
6. Dual view mode (3D BIM ↔ 2D Map)

## Project Structure

```
bim_viewer/
├── android/                    # Android platform code
├── ios/                        # iOS platform code
├── lib/                        # Flutter/Dart code
│   ├── main.dart
│   ├── features/
│   │   ├── viewer/            # BIM viewer feature
│   │   ├── file_manager/      # File handling
│   │   └── settings/          # App settings
│   ├── core/
│   │   ├── bridge/            # FRB generated bindings
│   │   └── models/            # Data models
│   └── shared/
│       ├── widgets/           # Reusable widgets
│       └── utils/             # Utilities
├── rust/                       # Rust code
│   ├── src/
│   │   ├── lib.rs            # Main library entry
│   │   ├── api.rs            # Flutter-exposed API
│   │   ├── bim/              # BIM processing
│   │   │   ├── ifcopenshell_wrapper.rs  # IfcOpenShell FFI wrapper
│   │   │   ├── geometry.rs              # Geometry post-processing
│   │   │   └── model.rs                 # BIM model structure
│   │   ├── renderer/         # 3D rendering
│   │   │   ├── engine.rs
│   │   │   ├── camera.rs
│   │   │   └── scene.rs
│   │   └── utils/            # Rust utilities
│   ├── cpp/                  # C++ bridge for IfcOpenShell
│   │   └── ifcopenshell_bridge.cpp
│   ├── Cargo.toml
│   └── build.rs
├── test/                      # Flutter tests
├── integration_test/          # Integration tests
├── pubspec.yaml
└── README.md
```

## Implementation Steps

### Step 1: Environment Setup

**Prerequisites**
- Install Flutter SDK
- Install Rust toolchain (rustup)
- Install Android Studio / Xcode (for mobile targets)
- Install VS Code with Flutter and Rust extensions

**Platform-Specific Requirements**
- Windows: Visual Studio C++ Build Tools
- macOS: Xcode Command Line Tools
- Linux: GCC, CMake, GTK development libraries

### Step 2: Project Initialization

1. Create Flutter project:
   ```bash
   flutter create bim_viewer
   cd bim_viewer
   ```

2. Initialize Rust library:
   ```bash
   cargo new --lib rust
   ```

3. Add flutter_rust_bridge dependencies to `pubspec.yaml`:
   ```yaml
   dependencies:
     flutter_rust_bridge: ^2.0.0
     ffi: ^2.1.0

   dev_dependencies:
     ffigen: ^9.0.0
     build_runner: ^2.4.0
   ```

4. Configure `rust/Cargo.toml`:
   ```toml
   [lib]
   crate-type = ["cdylib", "staticlib"]

   [dependencies]
   flutter_rust_bridge = "2.0"
   anyhow = "1.0"

   # C++ interop for IfcOpenShell
   cxx = "1.0"

   [build-dependencies]
   cxx-build = "1.0"
   ```

5. Set up IfcOpenShell (see IFCOPENSHELL_INTEGRATION.md for details)

### Step 3: Flutter Rust Bridge Setup

1. Create FRB configuration file
2. Define Rust API surface in `rust/src/api.rs`
3. Generate bindings with `flutter_rust_bridge_codegen`
4. Configure build scripts for each platform

### Step 4: BIM Core Development (Rust + IfcOpenShell)

**IMPORTANT**: We use **IfcOpenShell** instead of a custom parser for superior performance and reliability.

**4.1 IfcOpenShell Integration**
- Set up IfcOpenShell compilation (C++)
- Create Rust FFI bindings using `cxx` or `bindgen`
- Build for all target platforms (Windows, Android, iOS)
- Wrap IfcOpenShell API in safe Rust interface

**4.2 IFC Model Loading** *(using IfcOpenShell)*
- Load IFC files via IfcOpenShell API
- Extract model metadata (project info, site data)
- Retrieve all building elements
- Parse element properties and relationships

**4.3 Geometry Extraction** *(using OpenCASCADE via IfcOpenShell)*
- Use IfcOpenShell's geometry iterator
- Extract triangulated meshes (vertices, indices, normals)
- Handle complex geometry (extrusions, B-reps, CSG)
- Process materials and colors
- Calculate bounding boxes

**4.4 Data Processing** *(Rust-native)*
- Build spatial index (R-tree) for fast queries
- Create Rust-native BimModel structure
- Implement element caching
- Optimize memory layout

**4.5 Rendering Engine**
- Initialize graphics context (wgpu)
- Create render pipeline
- Implement camera system
- Develop material system
- Build lighting system

### Step 5: Flutter UI Development

**5.1 File Management**
- File picker integration
- Recent files list
- File format validation

**5.2 Viewer Widget**
- Native texture widget for rendering
- Touch gesture handling
- Camera control UI
- Toolbar implementation

**5.3 Properties Panel**
- Element property display
- Search and filter
- Tree view of model hierarchy

**5.4 Settings**
- Rendering options
- Performance settings
- Theme selection

### Step 6: Integration

1. Connect Flutter UI to Rust backend via FRB
2. Implement state synchronization
3. Handle async operations
4. Error handling and logging

### Step 7: Testing

1. Unit tests for Rust components
2. Widget tests for Flutter UI
3. Integration tests for full workflow
4. Performance benchmarking
5. Platform-specific testing

### Step 8: 2D GIS Integration

**8.1 Georeferencing Extraction (Rust)**
- Parse IfcSite entity from IFC
- Extract IfcMapConversion data
- Read ProjectedCRS information
- Calculate building bounds in geographic coordinates
- Return lat/lon, rotation, elevation to Flutter

**8.2 Map View Integration (Flutter)**
- Add flutter_map dependency
- Create MapView widget
- Configure OpenStreetMap tile layer
- Display initial map centered on building location
- Handle map gestures (pan, zoom)

**8.3 Building Footprint Overlay**
- Calculate building footprint from IFC geometry
- Convert local coordinates to lat/lon
- Draw polygon on map
- Add building marker
- Style footprint (color, opacity, outline)

**8.4 Dual View Mode**
- Create tab navigation (3D View / Map View)
- Or split-screen layout option
- Synchronize selection between views
- Share camera/location state
- Smooth transitions

**8.5 Site Context Features**
- Show nearby addresses
- Display property boundaries (if available)
- Add compass/north indicator
- Distance/area measurements on map
- Export map screenshot with building location

### Step 9: Optimization

1. Profile rendering performance
2. Optimize large model loading
3. Implement level-of-detail (LOD)
4. Add caching mechanisms
5. Memory management improvements
6. Optimize map tile loading and caching

### Step 10: Deployment

1. Configure platform-specific builds
2. Create app icons and splash screens
3. Set up CI/CD pipeline
4. Prepare store listings
5. Documentation and user guides

## Key Technical Challenges

### Challenge 1: FFI Performance
- **Issue**: Frequent FFI calls can impact performance
- **Solution**: Batch operations, use shared memory for large data

### Challenge 2: Graphics Context Management
- **Issue**: Different rendering approaches per platform
- **Solution**: Abstract rendering backend, use wgpu for cross-platform

### Challenge 3: Large File Handling
- **Issue**: BIM files can be hundreds of MB
- **Solution**: Streaming parser, progressive loading, spatial indexing

### Challenge 4: Memory Management
- **Issue**: Rust and Dart have different memory models
- **Solution**: Clear ownership rules, use Arc/Mutex where needed

### Challenge 5: Platform Differences
- **Issue**: iOS/Android/Desktop have different capabilities
- **Solution**: Feature detection, graceful degradation

## Dependencies

### Flutter Dependencies
```yaml
dependencies:
  flutter:
    sdk: flutter
  flutter_rust_bridge: ^2.0.0
  ffi: ^2.1.0
  riverpod: ^2.4.0
  file_picker: ^6.0.0
  path_provider: ^2.1.0
  shared_preferences: ^2.2.0

  # 2D GIS / Map integration
  flutter_map: ^6.1.0
  latlong2: ^0.9.0
  geolocator: ^10.1.0

dev_dependencies:
  flutter_test:
    sdk: flutter
  build_runner: ^2.4.0
  integration_test:
    sdk: flutter
```

### Rust Dependencies
```toml
[dependencies]
flutter_rust_bridge = "2.0"
wgpu = "0.18"
nalgebra = "0.32"
anyhow = "1.0"
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.35", features = ["sync"] }

# For IFC parsing
nom = "7.1"  # Parser combinator library

[dev-dependencies]
criterion = "0.5"  # Benchmarking
```

## Development Workflow

### Daily Development
1. Start with Rust implementation of core features
2. Define clean API surface in `api.rs`
3. Run FRB codegen to update bindings
4. Implement Flutter UI consuming Rust APIs
5. Test on target platforms
6. Iterate based on performance/UX

### Build Process
```bash
# Generate FRB bindings
flutter_rust_bridge_codegen generate

# Build Rust library
cd rust && cargo build --release

# Run Flutter app
flutter run

# Run tests
cargo test          # Rust tests
flutter test        # Dart tests
```

## Performance Targets

- Load 10MB IFC file: < 2 seconds
- Render 60 FPS for models with 100k+ triangles
- Smooth navigation (< 16ms frame time)
- Memory usage: < 500MB for typical models
- Cold start: < 3 seconds

## Security Considerations

1. Validate all IFC input to prevent malformed files
2. Sandbox file system access
3. Implement proper error handling (no panics exposed to Flutter)
4. Secure storage for recent files/settings

## Future Enhancements

1. Cloud storage integration (Dropbox, Google Drive, OneDrive)
2. Collaborative viewing (real-time multi-user)
3. AR support (view model in real-world context via camera)
4. VR support (immersive viewing with headsets)
5. PDF export (2D drawings from sections)
6. Point cloud support (laser scan data integration)
7. Multi-model loading (federated models)
8. Animation and sequencing (4D construction simulation)
9. Cost estimation integration (5D BIM)
10. Offline tile caching for maps

## Resources

### Documentation
- Flutter Rust Bridge: https://cjycode.com/flutter_rust_bridge/
- IFC Format: https://www.buildingsmart.org/
- wgpu: https://wgpu.rs/

### Example Projects
- Study existing Rust 3D engines
- Review Flutter platform view implementations
- Examine open-source BIM viewers

### Community
- Flutter Discord
- Rust Users Forum
- BuildingSMART forums

## Success Metrics

1. Successfully load and display IFC models
2. Smooth 60 FPS rendering on target devices
3. Cross-platform compatibility (iOS, Android, Windows, macOS, Linux)
4. Positive user feedback on performance
5. Code maintainability score > 8/10

---

**Next Steps**: Review this plan, adjust based on requirements, and proceed with Step 1 (Environment Setup) when ready to implement.
