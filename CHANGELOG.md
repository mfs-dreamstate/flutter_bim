# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-01-03

### Added - Initial Release 🎉

#### Core Features
- **IFC File Support**: Load and parse IFC 2x3 and IFC 4 files
- **3D Rendering**: High-performance wgpu-based rendering (Vulkan/Metal)
- **Element Selection**: Tap to select and highlight elements in 3D view
- **Properties Panel**: View detailed element information and metadata
- **Multi-Model Support**: Load and manage multiple IFC models simultaneously
- **Element Tree View**: Hierarchical navigation of model elements
- **Model Manager**: UI for loading, unloading, and switching between models

#### Widgets
- `ViewerScreen`: Main 3D viewer widget with touch controls
- `ElementTreeDrawer`: Hierarchical element tree navigation
- `ModelManagerDrawer`: Multi-model management interface
- `MapViewScreen`: GIS integration with OpenStreetMap
- `PropertiesPanel`: Element property display
- `GridOverlay`: Reference grid for 3D viewport
- `MeasurementTools`: UI for measurements (calculations in progress)
- `SectionPlaneTools`: UI for section planes (rendering in progress)

#### Rendering Features
- Interactive camera controls (orbit, pan, zoom)
- Touch gesture support (pinch-to-zoom, swipe-to-orbit)
- Element visibility toggles by type
- Lighting controls (direction, intensity, color)
- Wireframe and shaded render modes
- Element highlighting on selection

#### Platform Support
- ✅ Android (SDK 26+, Vulkan required)
- ✅ iOS (iOS 13+, Metal required)
- ❌ Web (planned for v0.3.0)
- ❌ Desktop (planned for future)

### Known Limitations
- Measurement calculations not yet implemented (UI only)
- Section plane rendering not yet implemented (UI only)
- Basic materials only (PBR materials planned for v0.3.0)
- Limited IFC entity type coverage (core types supported)
- No animation support yet

### Development
- Rust backend with Flutter Rust Bridge FFI
- Custom IFC parser using nom combinators
- wgpu for cross-platform graphics (Vulkan/Metal)
- MIT License

---

## [Unreleased]

### Planned for v0.2.0
- Complete measurement tool calculations
- Section plane rendering implementation
- Performance optimizations
- Bug fixes based on community feedback

### Planned for v0.3.0
- Web platform support (wgpu WebGL backend)
- Expanded IFC entity type support
- Advanced materials (PBR)
- Improved documentation

### Planned for v1.0.0
- Full test coverage (unit + integration)
- Production-ready performance
- Comprehensive API documentation
- Desktop platform support (Windows, macOS, Linux)

---

[0.1.0]: https://github.com/mfs-dreamstate/flutter_bim/releases/tag/v0.1.0
