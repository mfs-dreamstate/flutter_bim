# Flutter BIM - Package Migration Plan

**Goal**: Convert the BIM viewer app into a reusable Flutter package publishable on pub.dev

**Package Name**: `flutter_bim`
**Status**: ✅ Name available on pub.dev
**Target Version**: 0.1.0 (initial preview)

---

## 🎯 Overview

This document outlines the steps to transform the current BIM viewer app into a reusable Flutter package that other developers can integrate into their projects.

---

## 📋 Migration Checklist

### Phase 1: Project Restructuring ⏳
- [ ] Create package directory structure
- [ ] Move current lib/ code to lib/src/
- [ ] Create lib/flutter_bim.dart public API export file
- [ ] Create example/ directory with demo app
- [ ] Update import paths to use package syntax

### Phase 2: Package Configuration ⏳
- [ ] Update pubspec.yaml for package publishing
- [ ] Add LICENSE file (MIT or BSD recommended)
- [ ] Update CHANGELOG.md with initial release notes
- [ ] Create/update README.md for package usage
- [ ] Add API documentation comments
- [ ] Configure analysis_options.yaml for package standards

### Phase 3: Rust Library Strategy ⏳
- [ ] Decide on Rust distribution approach:
  - Option A: Pre-built binaries per platform (recommended for MVP)
  - Option B: Build from source (requires users to have Rust toolchain)
  - Option C: Hybrid (pre-built with source fallback)
- [ ] Create build scripts for library compilation
- [ ] Document platform-specific build requirements
- [ ] Handle library loading in package

### Phase 4: Example App ⏳
- [ ] Create minimal example app
- [ ] Demonstrate key features (loading IFC, 3D rendering, properties)
- [ ] Include sample IFC files
- [ ] Add example README with usage instructions

### Phase 5: Documentation ⏳
- [ ] Write comprehensive README.md
- [ ] Add dartdoc comments to all public APIs
- [ ] Create getting started guide
- [ ] Document platform setup requirements
- [ ] Add architecture overview
- [ ] Include troubleshooting section

### Phase 6: Testing & Quality ⏳
- [ ] Add unit tests for Dart code
- [ ] Add integration tests
- [ ] Verify example app works on Android/iOS
- [ ] Run flutter analyze with no issues
- [ ] Format all code with dart format
- [ ] Verify package score requirements

### Phase 7: Pre-Publication ⏳
- [ ] Run `flutter pub publish --dry-run`
- [ ] Fix any publication warnings
- [ ] Verify package size is reasonable
- [ ] Check all dependencies are properly listed
- [ ] Ensure no sensitive data in package

### Phase 8: Publication 🚀
- [ ] Create GitHub repository (recommended)
- [ ] Tag initial release (v0.1.0)
- [ ] Run `flutter pub publish`
- [ ] Announce on Flutter community channels

---

## 📁 Detailed File Structure

```
flutter_bim/
│
├── lib/
│   ├── flutter_bim.dart              # Main export - public API surface
│   │
│   └── src/                          # Private implementation (not exported)
│       ├── widgets/
│       │   ├── bim_viewer.dart       # Main 3D viewer widget
│       │   ├── viewer_controls.dart  # Camera/render controls
│       │   ├── element_tree.dart     # Hierarchical element tree
│       │   ├── properties_panel.dart # Element properties display
│       │   ├── model_manager.dart    # Multi-model management
│       │   ├── map_view.dart         # GIS integration (optional)
│       │   ├── measurement_tools.dart
│       │   ├── section_plane_tools.dart
│       │   └── drawing_overlay.dart
│       │
│       ├── core/
│       │   ├── bridge/               # FFI bindings (generated)
│       │   │   ├── api.dart
│       │   │   ├── frb_generated.dart
│       │   │   └── ...
│       │   ├── models/               # Data models
│       │   │   ├── model_info.dart
│       │   │   ├── element_info.dart
│       │   │   └── geometry.dart
│       │   └── services/
│       │       ├── ifc_service.dart
│       │       └── renderer_service.dart
│       │
│       └── utils/
│           ├── color_utils.dart
│           └── math_utils.dart
│
├── example/                          # Example app demonstrating usage
│   ├── lib/
│   │   └── main.dart                 # Simple demo app
│   ├── assets/
│   │   └── sample.ifc                # Sample IFC file
│   ├── android/
│   ├── ios/
│   ├── pubspec.yaml
│   └── README.md
│
├── rust/                             # Rust native library
│   ├── src/
│   ├── Cargo.toml
│   └── build.rs
│
├── scripts/                          # Build automation scripts
│   ├── build_rust.sh                 # Build Rust for all platforms
│   ├── generate_bindings.sh          # Regenerate FFI bindings
│   └── prepare_release.sh            # Pre-publication checklist
│
├── test/                             # Package tests
│   ├── flutter_bim_test.dart
│   └── widget_test.dart
│
├── .gitignore
├── analysis_options.yaml
├── CHANGELOG.md
├── LICENSE
├── pubspec.yaml                      # Package configuration
└── README.md                         # Package documentation
```

---

## 🔌 Public API Design

The main `lib/flutter_bim.dart` should export only what users need:

```dart
library flutter_bim;

// Core widgets
export 'src/widgets/bim_viewer.dart';
export 'src/widgets/element_tree.dart';
export 'src/widgets/properties_panel.dart';
export 'src/widgets/model_manager.dart';

// Optional widgets
export 'src/widgets/map_view.dart' show BimMapView;
export 'src/widgets/measurement_tools.dart' show MeasurementToolsPanel;

// Models (data classes users will interact with)
export 'src/core/models/model_info.dart';
export 'src/core/models/element_info.dart';
export 'src/core/models/geometry.dart';

// Services (if needed)
export 'src/core/services/ifc_service.dart' show IfcService;

// DO NOT export internal implementation (src/core/bridge/, etc.)
```

---

## 📝 pubspec.yaml for Package

```yaml
name: flutter_bim
description: A high-performance BIM (Building Information Modeling) viewer for Flutter with IFC file support, 3D rendering, and element inspection.
version: 0.1.0
homepage: https://github.com/YOUR_USERNAME/flutter_bim
repository: https://github.com/YOUR_USERNAME/flutter_bim
issue_tracker: https://github.com/YOUR_USERNAME/flutter_bim/issues
documentation: https://pub.dev/documentation/flutter_bim/latest/

environment:
  sdk: '>=3.2.0 <4.0.0'
  flutter: '>=3.16.0'

dependencies:
  flutter:
    sdk: flutter
  flutter_rust_bridge: ^2.11.1
  file_picker: ^6.0.0
  flutter_map: ^6.1.0       # Optional - for GIS features
  latlong2: ^0.9.0          # Optional - for GIS features

dev_dependencies:
  flutter_test:
    sdk: flutter
  flutter_lints: ^3.0.0

# Important: Include Rust binaries
flutter:
  plugin:
    platforms:
      android:
        ffiPlugin: true
      ios:
        ffiPlugin: true

# Platform-specific assets
assets:
  - assets/       # For any default assets

topics:
  - bim
  - ifc
  - 3d
  - rendering
  - architecture
  - construction
  - wgpu

screenshots:
  - description: 'BIM Viewer showing 3D model'
    path: screenshots/viewer.png
  - description: 'Element properties panel'
    path: screenshots/properties.png
```

---

## 🚧 Rust Library Distribution Strategy

### Option A: Pre-built Binaries (Recommended for v0.1.0)

**Pros**:
- Users don't need Rust toolchain
- Faster package installation
- Easier to get started

**Cons**:
- Larger package size
- Need to build for all platforms
- Platform matrix: Android (arm64-v8a, armeabi-v7a, x86_64), iOS (arm64, x86_64 sim)

**Implementation**:
1. Build Rust for all target platforms
2. Include compiled `.so`/`.dylib` files in package
3. Configure FFI to load correct library per platform

### Option B: Build from Source

**Pros**:
- Smaller package size
- Users can inspect/modify Rust code
- More transparent

**Cons**:
- Users need Rust toolchain installed
- Slower installation (compile time)
- More complex setup

**Implementation**:
1. Include Rust source in package
2. Provide build scripts
3. Require users to have Rust + cargo-ndk (Android)

### Option C: Hybrid (Future Enhancement)

- Ship pre-built binaries
- Provide source + build scripts for advanced users
- Auto-detect and use best option

### **Recommendation for v0.1.0**: Start with Option A (pre-built binaries)
- Focus on developer experience
- Can add source builds in v0.2.0+

---

## 📖 README.md Template

```markdown
# flutter_bim

A high-performance BIM (Building Information Modeling) viewer for Flutter applications.

Load, visualize, and interact with IFC files directly in your Flutter app with native 3D rendering powered by Rust and wgpu.

[![pub package](https://img.shields.io/pub/v/flutter_bim.svg)](https://pub.dev/packages/flutter_bim)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- 📦 **IFC File Support**: Load and parse IFC 2x3 and IFC 4 files
- 🎨 **3D Rendering**: High-performance rendering with wgpu (Vulkan/Metal)
- 🔍 **Element Inspection**: View properties, materials, and metadata
- 📐 **Measurement Tools**: Distance, area, and volume measurements
- 🗂️ **Multi-Model Support**: Load and manage multiple models
- 🗺️ **GIS Integration**: Display building location on OpenStreetMap
- 📱 **Cross-Platform**: iOS and Android support

## Demo

[Screenshots here]

## Installation

Add to your `pubspec.yaml`:

```yaml
dependencies:
  flutter_bim: ^0.1.0
```

## Quick Start

```dart
import 'package:flutter_bim/flutter_bim.dart';

class MyApp extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        body: BimViewer(
          onModelLoaded: (modelInfo) {
            print('Loaded: ${modelInfo.projectName}');
          },
        ),
      ),
    );
  }
}
```

## Platform Setup

### Android
- Min SDK: 26 (Android 8.0)
- Requires Vulkan support

### iOS
- Min iOS: 13.0
- Metal support required

[Full documentation link]

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md)

## License

MIT License - see [LICENSE](LICENSE)
```

---

## 🧪 Example App Structure

The example app should be minimal and focused on demonstrating package usage:

```dart
// example/lib/main.dart

import 'package:flutter/material.dart';
import 'package:flutter_bim/flutter_bim.dart';

void main() {
  runApp(const FlutterBimExampleApp());
}

class FlutterBimExampleApp extends StatelessWidget {
  const FlutterBimExampleApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Flutter BIM Example',
      theme: ThemeData.dark(),
      home: const ExampleHome(),
    );
  }
}

class ExampleHome extends StatelessWidget {
  const ExampleHome({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Flutter BIM Example')),
      body: BimViewer(
        onModelLoaded: (modelInfo) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text('Loaded: ${modelInfo.projectName}')),
          );
        },
      ),
    );
  }
}
```

---

## ✅ Pre-Publication Checklist

Before running `flutter pub publish`:

1. **Code Quality**
   - [ ] All files have dartdoc comments
   - [ ] `flutter analyze` shows 0 issues
   - [ ] Code is formatted with `dart format .`
   - [ ] All TODOs are resolved or documented

2. **Documentation**
   - [ ] README.md is comprehensive
   - [ ] CHANGELOG.md is updated
   - [ ] Example app runs and demonstrates features
   - [ ] API documentation is complete

3. **Legal**
   - [ ] LICENSE file added (MIT recommended)
   - [ ] All dependencies are compatible
   - [ ] No copyrighted assets included

4. **Testing**
   - [ ] Example app tested on Android
   - [ ] Example app tested on iOS (if available)
   - [ ] Package size is reasonable (<10MB preferred)

5. **Metadata**
   - [ ] pubspec.yaml has all required fields
   - [ ] Version number follows semver
   - [ ] Topics/tags are relevant
   - [ ] Screenshots added (optional but recommended)

6. **Dry Run**
   ```bash
   flutter pub publish --dry-run
   ```
   - [ ] No errors
   - [ ] Score prediction looks good (aim for 130+)

---

## 🎉 Publishing

1. **Create GitHub Repository** (optional but recommended)
   ```bash
   git init
   git add .
   git commit -m "Initial release v0.1.0"
   git tag v0.1.0
   git remote add origin https://github.com/YOUR_USERNAME/flutter_bim.git
   git push -u origin main --tags
   ```

2. **Publish to pub.dev**
   ```bash
   flutter pub publish
   ```

3. **Announce**
   - Share on r/FlutterDev
   - Tweet with #FlutterDev
   - Post on LinkedIn

---

## 📈 Post-Publication Roadmap

### v0.2.0
- [ ] Improve documentation
- [ ] Add more examples
- [ ] Performance optimizations
- [ ] Bug fixes from community feedback

### v0.3.0
- [ ] Web support (wgpu WebGL backend)
- [ ] More IFC entity types
- [ ] Advanced rendering features

### v1.0.0 (Stable)
- [ ] Full test coverage
- [ ] Comprehensive documentation
- [ ] Production-ready performance
- [ ] Desktop support (Windows, macOS, Linux)

---

## 🤝 Community

- **Issues**: Report bugs and request features on GitHub
- **Discussions**: Share ideas and get help
- **Contributing**: PRs welcome! See CONTRIBUTING.md

---

**Next Steps**: Start with Phase 1 - Project Restructuring
