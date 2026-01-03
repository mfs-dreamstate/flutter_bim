# flutter_bim

A high-performance BIM (Building Information Modeling) viewer for Flutter applications.

[![pub package](https://img.shields.io/pub/v/flutter_bim.svg)](https://pub.dev/packages/flutter_bim)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Load, visualize, and interact with IFC files directly in your Flutter app with native 3D rendering powered by Rust and wgpu.

## 🎯 Features

- 📦 **IFC File Support** - Load and parse IFC 2x3 and IFC 4 files
- 🎨 **High-Performance 3D Rendering** - Hardware-accelerated rendering with wgpu (Vulkan on Android, Metal on iOS)
- 🏗️ **Element Inspection** - View properties, materials, and metadata for any building element
- 🔍 **Element Selection** - Tap to select and highlight elements in the 3D view
- 🗂️ **Multi-Model Support** - Load and manage multiple IFC models simultaneously
- 🗺️ **GIS Integration** - Display building location on OpenStreetMap (when IFC contains georeferencing)
- 📐 **Measurement Tools** - Measure distances, areas, and volumes (UI ready, calculations in progress)
- ✂️ **Section Planes** - Cut through the model to see internal structure (UI ready, rendering in progress)
- 📱 **Cross-Platform** - iOS and Android support

## 📸 Screenshots

[Coming soon - Add screenshots showing the 3D viewer, element properties, and map view]

## 🚀 Getting Started

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
- **NDK**: Automatically handled by the package

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

  // Initialize the Rust bridge
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
        body: ViewerScreen(), // The main 3D viewer widget
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

// Or parse from string content
final modelInfo = await bim.parseIfcContent(content: ifcContent);
```

## 📖 API Documentation

### Widgets

#### `ViewerScreen`
The main 3D viewer widget with built-in controls.

```dart
ViewerScreen()
```

Features:
- Interactive 3D rendering
- Touch gestures (orbit, zoom, pan)
- Element selection
- Built-in toolbar with visibility controls
- Lighting and render mode settings

#### `ElementTreeDrawer`
Hierarchical tree view of model elements.

```dart
ElementTreeDrawer(
  onElementSelected: (element) {
    print('Selected: ${element.name}');
  },
  selectedElementId: currentElementId,
)
```

#### `ModelManagerDrawer`
Multi-model management drawer.

```dart
ModelManagerDrawer(
  onModelsChanged: () {
    // Refresh UI when models change
  },
)
```

#### `MapViewScreen`
GIS map view showing building location (requires georeferenced IFC).

```dart
MapViewScreen()
```

### Data Models

#### `ModelInfo`
Information about a loaded IFC model.

```dart
class ModelInfo {
  final String projectName;
  final String buildingName;
  final String siteName;
  final ModelStats stats;
}
```

#### `ElementInfo`
Information about a specific building element.

```dart
class ElementInfo {
  final String id;
  final String name;
  final String elementType;
  final String? description;
  final Map<String, String> properties;
}
```

### Core API

For advanced use cases, you can access the Rust API directly:

```dart
import 'package:flutter_bim/flutter_bim.dart' as bim;

// Initialize renderer
await bim.initRenderer(width: 1920, height: 1080);

// Camera controls
bim.orbitCamera(deltaX: 0.1, deltaY: 0.1);
bim.zoomCamera(delta: 10.0);

// Element visibility
bim.setElementTypeVisible(elementType: 'Wall', visible: false);

// Get model information
if (bim.isModelLoaded()) {
  final info = bim.getModelInfo();
}
```

## 🎨 Customization

### Theming

The viewer respects your app's Material theme:

```dart
MaterialApp(
  theme: ThemeData.dark(), // Dark mode
  home: ViewerScreen(),
)
```

### Custom Controls

You can build custom UI around the core viewer:

```dart
import 'package:flutter_bim/flutter_bim.dart' as bim;

// Custom load button
ElevatedButton(
  onPressed: () async {
    final result = await FilePicker.platform.pickFiles(
      type: FileType.custom,
      allowedExtensions: ['ifc'],
    );
    if (result != null) {
      await bim.loadIfcFile(filePath: result.files.first.path!);
    }
  },
  child: Text('Load Model'),
)
```

## 🛠️ Advanced Features

### Multi-Model Federation

Load and view multiple discipline models together:

```dart
// Load architectural model
await bim.loadModel(modelId: 'arch', filePath: 'architectural.ifc');

// Load structural model
await bim.loadModel(modelId: 'struct', filePath: 'structural.ifc');

// Load MEP model
await bim.loadModel(modelId: 'mep', filePath: 'mep.ifc');

// List all models
final models = bim.listLoadedModels();
```

### Element Selection

```dart
// Select element by ID
bim.setSelectedElement(elementId: 'someElementId');

// Clear selection
bim.setSelectedElement(elementId: null);

// Get selected element info
final element = bim.pickElement(screenX: 0.5, screenY: 0.5);
```

### Visibility Control

```dart
// Hide all walls
bim.setElementTypeVisible(elementType: 'Wall', visible: false);

// Hide specific model
bim.setModelVisible(modelId: 'mep', visible: false);
```

## 📋 Example App

Check out the [example](https://pub.dev/packages/flutter_bim/example) directory for a complete sample app demonstrating:

- Loading IFC files
- 3D navigation
- Element selection and properties
- Multi-model management
- Map integration

Run the example:

```bash
cd example
flutter run
```

## 🧪 Development Status

**Current Version**: 0.1.0 (Preview)

This package is in early development. Core features work, but some advanced features are still being implemented:

- ✅ IFC parsing
- ✅ 3D rendering
- ✅ Element selection
- ✅ Properties display
- ✅ Multi-model support
- ✅ Map view (basic)
- 🚧 Measurement tools (UI ready, calculations in progress)
- 🚧 Section planes (UI ready, rendering in progress)
- 🚧 Advanced materials and lighting
- 📅 Animation and clash detection (planned)

## 🤝 Contributing

Contributions are welcome! This is an open-source project.

### Areas where we'd love help:
- 📐 Measurement tool calculations
- ✂️ Section plane rendering
- 🎨 Advanced materials (PBR)
- 🌐 IFC 5 support
- 📝 Documentation improvements
- 🐛 Bug reports and fixes

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Flutter Rust Bridge](https://github.com/fzyzcjy/flutter_rust_bridge) for seamless Dart ↔ Rust FFI
- 3D rendering powered by [wgpu](https://wgpu.rs/)
- IFC parsing using custom Rust implementation with [nom](https://github.com/rust-bakery/nom) parser combinators
- Map integration via [flutter_map](https://pub.dev/packages/flutter_map) and OpenStreetMap

## 📧 Support

- 📚 [Documentation](https://pub.dev/documentation/flutter_bim/latest/)
- 🐛 [Issue Tracker](https://github.com/mfs-dreamstate/flutter_bim/issues)
- 💬 [Discussions](https://github.com/mfs-dreamstate/flutter_bim/discussions)

## 🗺️ Roadmap

### v0.2.0
- Complete measurement tools implementation
- Section plane rendering
- Performance optimizations

### v0.3.0
- Web support (wgpu WebGL backend)
- More IFC entity types
- Advanced materials

### v1.0.0
- Full test coverage
- Production-ready stability
- Comprehensive documentation

---

**Made with ❤️ for the AEC (Architecture, Engineering, Construction) community**
