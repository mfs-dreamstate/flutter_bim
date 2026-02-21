# flutter_bim

A high-performance BIM (Building Information Modeling) viewer for Flutter applications.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Load, visualize, and interact with IFC files directly in your Flutter app with native 3D rendering powered by Rust and wgpu.

## Features

- **IFC File Support** — Load and parse IFC 2x3 and IFC 4 files
- **High-Performance 3D Rendering** — Hardware-accelerated rendering via wgpu (Vulkan on Android, Metal on iOS)
- **Element Inspection** — View properties, materials, and metadata for any building element
- **Element Selection** — Tap to select and highlight elements in the 3D view
- **Multi-Model Support** — Load and manage multiple IFC models simultaneously
- **GIS Integration** — Display building location on OpenStreetMap (when IFC contains georeferencing)
- **Cross-Platform** — iOS and Android support

## Getting Started

### Installation

Add `flutter_bim` to your `pubspec.yaml`:

```yaml
dependencies:
  flutter_bim: ^0.1.0
```

Then run:
```bash
flutter pub get
```

### Platform Requirements

#### Android
- **Minimum SDK**: 26 (Android 8.0)
- **Vulkan Support**: Required for 3D rendering

#### iOS
- **Minimum iOS**: 13.0
- **Metal Support**: Required for 3D rendering
- **Xcode**: 14.0 or later

### Basic Usage

```dart
import 'package:flutter/material.dart';
import 'package:flutter_bim/flutter_bim.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        appBar: AppBar(title: const Text('BIM Viewer')),
        body: ViewerScreen(),
      ),
    );
  }
}
```

### Loading an IFC File

The viewer includes a built-in file picker and model manager. Users can:

1. Tap the "layers" icon in the app bar to open the model manager
2. Select "Load IFC File" to browse and load models from device storage
3. View model information and manage multiple loaded models

Programmatically:

```dart
import 'package:flutter_bim/flutter_bim.dart' as bim;

// Load from file path
final modelInfo = await bim.loadIfcFile(filePath: '/path/to/model.ifc');
print('Loaded: ${modelInfo.projectName}');
print('Elements: ${modelInfo.stats.totalEntities}');
```

## API Overview

### Widgets

| Widget | Description |
|--------|-------------|
| `ViewerScreen` | Main 3D viewer with built-in controls, touch gestures, element selection, and toolbar |
| `ElementTreeDrawer` | Hierarchical tree view of model elements |
| `ModelManagerDrawer` | Multi-model management drawer |
| `MapViewScreen` | GIS map view showing building location (requires georeferenced IFC) |

### Core API

```dart
import 'package:flutter_bim/flutter_bim.dart' as bim;

// Renderer
await bim.initRenderer(width: 1920, height: 1080);

// Camera
bim.orbitCamera(deltaX: 0.1, deltaY: 0.1);
bim.zoomCamera(delta: 10.0);

// Visibility
bim.setElementTypeVisible(elementType: 'Wall', visible: false);
bim.setModelVisible(modelId: 'mep', visible: false);

// Selection
bim.setSelectedElement(elementId: 'someElementId');
final element = bim.pickElement(screenX: 0.5, screenY: 0.5);

// Multi-model
await bim.loadModel(modelId: 'arch', filePath: 'architectural.ifc');
final models = bim.listLoadedModels();
```

## Development Setup

### Prerequisites

- [Flutter](https://flutter.dev/docs/get-started/install) 3.x+
- [Rust](https://rustup.rs/) (stable toolchain)
- [flutter_rust_bridge](https://cjycode.com/flutter_rust_bridge/) v2 CLI
- Android SDK with NDK (for Android builds)
- Xcode 14+ (for iOS builds, macOS only)

### Clone and Build

```bash
git clone https://github.com/mfs-dreamstate/flutter_bim.git
cd flutter_bim
```

### Android NDK Setup

For Android cross-compilation, Rust needs to know where your NDK linkers are. Copy the example Cargo config and fill in your local paths:

```bash
cp rust/.cargo/config.toml.example rust/.cargo/config.toml
```

Then edit `rust/.cargo/config.toml` and replace `<USERNAME>` and `<NDK_VERSION>` with your values. You can find your NDK version under:
- **Windows**: `C:\Users\<USERNAME>\AppData\Local\Android\Sdk\ndk\`
- **macOS**: `~/Library/Android/sdk/ndk/`
- **Linux**: `~/Android/Sdk/ndk/`

### Add Rust Targets

```bash
# Android
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android

# iOS (macOS only)
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```

### Run the Example

```bash
cd example
flutter run
```

## Project Structure

```
flutter_bim/
├── lib/src/
│   ├── core/
│   │   ├── bridge/       # Auto-generated FFI bindings (do not edit)
│   │   ├── constants/    # Shared constants
│   │   ├── errors/       # Error types
│   │   ├── providers/    # Riverpod state management
│   │   └── services/     # Service interfaces and implementations
│   └── widgets/          # UI components
├── rust/src/
│   ├── api/              # Public Rust API (exposed to Dart via FFI)
│   ├── bim/              # IFC parsing and BIM data structures
│   └── renderer/         # wgpu-based 3D renderer
├── test/                 # Dart tests
└── example/              # Example Flutter app
```

## Development Status

**Current Version**: 0.1.0 (Preview)

Core features are working, with some advanced features still in progress:

- [x] IFC parsing (IFC 2x3 and IFC 4)
- [x] 3D rendering (wgpu)
- [x] Element selection and properties
- [x] Multi-model support
- [x] Map view
- [ ] Measurement tools
- [ ] Section planes
- [ ] Advanced materials and lighting
- [ ] Animation and clash detection

## Contributing

Contributions are welcome! Areas where help is appreciated:

- Measurement tool calculations
- Section plane rendering
- Advanced materials (PBR)
- IFC 5 support
- Documentation improvements
- Bug reports and fixes

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Tech Stack

- [Flutter Rust Bridge](https://github.com/fzyzcjy/flutter_rust_bridge) — Dart-Rust FFI
- [wgpu](https://wgpu.rs/) — Cross-platform GPU rendering
- [nom](https://github.com/rust-bakery/nom) — IFC parser combinators
- [flutter_map](https://pub.dev/packages/flutter_map) — OpenStreetMap integration
- [Riverpod](https://riverpod.dev/) — State management

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.
