# Claude Context - Flutter Rust BIM Viewer Project

**Last Updated**: 2025-12-18
**Project Status**: Phases 1 & 2 Complete - 3D Rendering Next
**Current Phase**: Phase 3 - 3D Rendering (Preparation)

---

## 🎯 Project Overview

This is a **BIM (Building Information Modeling) viewer** application being built with:
- **Flutter** for cross-platform UI (iOS & Android)
- **Rust** for high-performance 3D rendering and IFC file parsing
- **Flutter Rust Bridge (FRB)** for seamless Rust ↔ Dart communication

### Core Purpose
Build a professional mobile BIM viewer that can:
1. Load and parse IFC files (industry standard for BIM data)
2. Render 3D building models with high performance (60 FPS)
3. Display element properties and metadata
4. Provide 2D map view showing building location (GIS integration)
5. Allow users to navigate, select, measure, and analyze building elements

### Target Platforms
- iOS 13+ (iPhone & iPad)
- Android 8+ (API level 26+)

---

## 📁 Project Structure

```
bim viewer test/
├── README.md                    # Main project overview & navigation
├── BIM_VIEWER_PLAN.md          # Complete implementation plan (10 steps, 5 phases)
├── ARCHITECTURE.md             # Technical architecture & design patterns
├── API_DESIGN.md               # Rust API specification (50+ methods)
├── SETUP_GUIDE.md              # Environment setup (Flutter, Rust, FRB)
├── QUICK_START.md              # 9-phase implementation guide (15 weeks)
├── PROGRESS.md                 # Live progress tracking (200+ tasks)
├── claude.md                   # This file - AI assistant context
│
├── android/                    # Android platform code (not yet created)
├── ios/                        # iOS platform code (not yet created)
│
├── lib/                        # Flutter/Dart code (not yet created)
│   ├── main.dart
│   ├── features/
│   │   ├── viewer/            # 3D BIM viewer
│   │   ├── map/               # 2D GIS map view
│   │   ├── properties/        # Element properties panel
│   │   └── file_manager/      # File handling
│   ├── core/
│   │   ├── bridge/            # FRB generated bindings
│   │   └── models/            # Data models
│   └── shared/
│       ├── widgets/           # Reusable widgets
│       └── utils/
│
└── rust/                       # Rust code (not yet created)
    ├── src/
    │   ├── lib.rs             # Library entry point
    │   ├── api.rs             # Flutter-exposed API
    │   ├── bim/               # BIM file parsing
    │   │   ├── ifc_parser.rs  # IFC STEP parser
    │   │   ├── geometry.rs    # Geometry extraction
    │   │   └── model.rs       # Model representation
    │   ├── renderer/          # 3D rendering engine
    │   │   ├── engine.rs      # wgpu rendering
    │   │   ├── camera.rs      # Camera system
    │   │   └── scene.rs       # Scene graph
    │   └── gis/               # GIS/georeferencing
    │       └── georef.rs      # Coordinate transformations
    ├── Cargo.toml
    └── build.rs
```

---

## 🛠️ Technology Stack

### Flutter (Frontend)
- **Version**: 3.16.0+
- **Language**: Dart 3.2+
- **State Management**: Riverpod (planned)
- **UI Framework**: Material Design 3
- **Key Packages**:
  - `flutter_rust_bridge: ^2.0.0` - FFI bridge
  - `flutter_map: ^6.1.0` - OpenStreetMap integration
  - `file_picker: ^6.0.0` - File selection
  - `path_provider: ^2.1.0` - File paths

### Rust (Backend)
- **Version**: 1.75.0+
- **Edition**: 2021
- **Key Crates**:
  - `flutter_rust_bridge = "2.0"` - FFI bridge
  - `wgpu = "0.18"` - Graphics (Vulkan/Metal abstraction)
  - `nalgebra = "0.32"` - Linear algebra
  - `tokio = "1.35"` - Async runtime
  - `nom = "7.1"` - Parser combinators (for IFC)
  - `rstar = "0.11"` - Spatial indexing (R-tree)

### Build Tools
- `flutter_rust_bridge_codegen` - Code generation
- `cargo-ndk` - Android builds
- Android NDK - Native Android compilation
- Xcode - iOS builds (macOS only)

---

## 📊 Implementation Phases (15 weeks total)

### **Phase 1: Foundation** (Weeks 1-2) - Status: ✅ Complete (2025-12-18)
- ✅ Set up Flutter + Rust project structure
- ✅ Configure Flutter Rust Bridge
- ✅ Verify FFI communication works
- ✅ Test on Android (iOS deferred - Windows dev environment)

### **Phase 2: BIM Parsing** (Weeks 3-4) - Status: ✅ Complete (2025-12-18)
- ✅ Implement IFC STEP file parser (using nom combinators)
- ✅ Extract geometry (meshes, vertices, indices) - foundation laid
- ✅ Build spatial index for fast queries - basic implementation
- ✅ Parse element properties and model hierarchy

### **Phase 3: 3D Rendering** (Weeks 5-6) - Status: ⏳ Not Started
- Initialize wgpu graphics backend
- Implement camera system (orbit, pan, zoom)
- Render meshes with basic shading
- 60 FPS performance target

### **Phase 4: Materials & Lighting** (Week 7) - Status: ⏳ Not Started
- PBR (Physically Based Rendering) materials
- Directional and ambient lighting
- Wireframe and shaded render modes

### **Phase 5: Interaction** (Week 8) - Status: ⏳ Not Started
- Ray casting for element selection
- Properties panel UI
- Search and filter functionality
- Layer/type visibility controls

### **Phase 6: 2D GIS Integration** (Week 9) - Status: ⏳ Not Started
- Extract georeferencing from IFC (IfcSite, IfcMapConversion)
- Integrate OpenStreetMap with flutter_map
- Display building footprint on map
- Dual view mode (3D ↔ Map tabs)
- Coordinate transformations (local ↔ geographic)

### **Phase 7: Advanced Features** (Weeks 10-11) - Status: ⏳ Not Started
- Measurement tools (distance, area, volume)
- Section planes
- Color coding by properties
- Export functionality

### **Phase 8: Optimization & Polish** (Weeks 12-13) - Status: ⏳ Not Started
- Performance profiling and optimization
- LOD (Level of Detail) system
- Error handling and logging
- Settings and preferences
- Unit and integration tests

### **Phase 9: Deployment** (Weeks 14-15) - Status: ⏳ Not Started
- App icons and branding
- CI/CD pipeline (GitHub Actions)
- App Store and Play Store submissions
- Beta testing (TestFlight, Play Store Beta)

---

## 🔑 Key Concepts

### IFC (Industry Foundation Classes)
- Open standard for BIM data exchange
- Based on STEP file format (ISO 10303-21)
- Contains building geometry, properties, relationships
- Versions: IFC 2x3, IFC 4 (both supported)

### Flutter Rust Bridge (FRB)
- Generates type-safe FFI bindings automatically
- Supports async/await across languages
- Data types serialize/deserialize automatically
- Code generation via `flutter_rust_bridge_codegen generate`

### wgpu
- Cross-platform graphics API (Rust)
- Abstracts over Vulkan (Android), Metal (iOS), DX12, WebGPU
- Modern, safe, high-performance rendering

### Georeferencing
- IFC files can include geographic coordinates
- `IfcSite` entity contains location data
- `IfcMapConversion` defines coordinate transformation
- Allows placing 3D model on 2D map

---

## 📝 Current State

### ✅ Completed
- [x] Complete project planning (all 6 documentation files)
- [x] Architecture design
- [x] API specification (50+ Rust methods defined)
- [x] Technology stack decisions
- [x] 9-phase implementation roadmap
- [x] Progress tracking system
- [x] **Phase 1: Foundation** - Flutter + Rust FFI working
- [x] **Phase 2: BIM Parsing** - Custom IFC parser implemented
- [x] Development environment setup (Windows + Android)
- [x] Project initialization
- [x] Android build with native Rust libraries
- [x] Sample IFC file loading and parsing
- [x] Model information extraction and display

### 🔄 In Progress
- [ ] **Phase 3: 3D Rendering** - Next up!

### ⏳ Not Started
- [ ] Phase 4-9
- [ ] iOS build configuration (Windows development)
- [ ] Deployment

### 📂 Files to Reference
1. **[PROGRESS.md](PROGRESS.md)** - Check this FIRST for current implementation status
2. **[BIM_VIEWER_PLAN.md](BIM_VIEWER_PLAN.md)** - Overall plan and steps
3. **[QUICK_START.md](QUICK_START.md)** - Phase-by-phase tasks
4. **[API_DESIGN.md](API_DESIGN.md)** - Rust API reference
5. **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design
6. **[SETUP_GUIDE.md](SETUP_GUIDE.md)** - Environment setup

---

## 🎯 Performance Targets

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| IFC Load Time (10MB file) | < 2 seconds | TBD | ⏳ |
| Rendering FPS | 60 FPS | TBD | ⏳ |
| Frame Time | < 16ms | TBD | ⏳ |
| Memory Usage | < 500MB | TBD | ⏳ |
| App Cold Start | < 3 seconds | TBD | ⏳ |

---

## 💡 Important Decisions Made

| Date | Decision | Rationale |
|------|----------|-----------|
| 2025-12-18 | Target iOS & Android only (not desktop) | Simplify initial release, focus on most important platforms |
| 2025-12-18 | Add 2D GIS as Phase 6 | IFC files often include georeferencing; adds significant value |
| 2025-12-18 | Use OpenStreetMap (not Google Maps) | Free, open source, no API keys required |
| 2025-12-18 | Use wgpu for graphics | Modern, safe, cross-platform (Vulkan/Metal) |
| 2025-12-18 | Use Riverpod for state management | Compile-safe, testable, Flutter community standard |
| 2025-12-18 | Use nom for IFC parsing | Powerful parser combinators, good for STEP format |

---

## 🚀 How to Help (For Claude in Future Sessions)

### When User Asks for Help

1. **Check Current Status**
   - Always read [PROGRESS.md](PROGRESS.md) FIRST
   - Check which phase they're in
   - See what tasks are complete/in-progress

2. **Implementation Guidance**
   - Reference [QUICK_START.md](QUICK_START.md) for current phase tasks
   - Reference [API_DESIGN.md](API_DESIGN.md) for Rust API signatures
   - Reference [ARCHITECTURE.md](ARCHITECTURE.md) for design patterns

3. **Code Generation**
   - Follow the architecture defined in ARCHITECTURE.md
   - Use the API signatures from API_DESIGN.md
   - Keep Rust code safe (no unwrap() across FFI)
   - Keep Flutter code clean (widget composition)

4. **Troubleshooting**
   - Check [SETUP_GUIDE.md](SETUP_GUIDE.md) troubleshooting section
   - Common issues: FRB codegen, platform builds, FFI errors

5. **Progress Tracking**
   - After completing tasks, update [PROGRESS.md](PROGRESS.md)
   - Mark tasks with `[x]` instead of `[ ]`
   - Update phase percentages
   - Add session notes

### Code Style Guidelines

#### Rust
```rust
// Use Result types, not panics
pub fn load_model(path: String) -> Result<ModelInfo, BimError> { }

// Avoid unwrap() in API surface
// ❌ Bad
let value = some_option.unwrap();

// ✅ Good
let value = some_option.ok_or(BimError::NoModelLoaded)?;

// Use #[frb(sync)] for fast functions, async for slow
#[frb(sync)]
pub fn get_element_info(id: String) -> Option<ElementInfo> { }

pub async fn load_model(path: String) -> Result<ModelInfo> { }
```

#### Flutter/Dart
```dart
// Use Riverpod for state management
final viewerProvider = StateNotifierProvider<ViewerController, ViewerState>(...);

// Use const constructors when possible
const Text('Hello')

// Prefer composition over inheritance
class ViewerScreen extends StatelessWidget {
  const ViewerScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: ViewerCanvas(),
    );
  }
}
```

### Common Commands

```bash
# Generate FRB bindings (after Rust API changes)
flutter_rust_bridge_codegen generate

# Build Rust (debug)
cd rust && cargo build

# Build Rust (release)
cd rust && cargo build --release

# Run Flutter (Android)
flutter run -d android

# Run Flutter (iOS)
flutter run -d ios

# Run tests
cargo test                  # Rust tests
flutter test                # Flutter tests
flutter test integration_test  # Integration tests

# Format code
cargo fmt                   # Rust
dart format .               # Dart

# Lint code
cargo clippy                # Rust
flutter analyze             # Dart
```

---

## 🐛 Known Issues / Gotchas

### Flutter Rust Bridge
- **Must regenerate bindings** after any Rust API change
- Hot reload doesn't work after Rust changes (need full restart)
- Some Rust types don't auto-convert (use manual mapping)

### Platform Differences
- **iOS**: Requires macOS for building
- **Android**: Requires NDK, multiple target architectures
- **Graphics**: Metal on iOS, Vulkan on Android (wgpu handles this)

### IFC Format
- **Complex**: STEP format is verbose and tricky to parse
- **Versions**: IFC 2x3 and IFC 4 have differences
- **Geometry**: Multiple representation types (extrusions, B-reps, etc.)
- **Coordinates**: Can be in local or geographic coordinate systems

---

## 📚 External Resources

### Documentation
- [Flutter Docs](https://flutter.dev/docs)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Flutter Rust Bridge Guide](https://cjycode.com/flutter_rust_bridge/)
- [wgpu Tutorial](https://sotrh.github.io/learn-wgpu/)
- [IFC Specification](https://www.buildingsmart.org/standards/bsi-standards/industry-foundation-classes/)

### Example Projects
- [FRB Examples](https://github.com/fzyzcjy/flutter_rust_bridge/tree/master/frb_example)
- [IFC.js](https://ifcjs.github.io/info/) - Web-based IFC viewer (reference)

### Test Data
- [IFC Sample Files](https://www.ifcwiki.org/index.php?title=KIT_IFC_Examples)
- BuildingSMART sample models

---

## 🔄 Session Workflow

### Starting a New Session
1. **Read** [PROGRESS.md](PROGRESS.md) to see current status
2. **Review** last session notes
3. **Check** for any blockers
4. **Identify** current phase and tasks

### During Session
1. **Implement** tasks from current phase
2. **Test** functionality as you go
3. **Document** issues or decisions
4. **Commit** code regularly

### Ending Session
1. **Update** [PROGRESS.md](PROGRESS.md):
   - Mark completed tasks [x]
   - Update phase percentages
   - Add session notes
   - Document blockers
2. **Set** goals for next session
3. **Commit** and push changes

---

## ✅ Quick Reference Checklist

**Before writing code:**
- [ ] Is environment set up? (Check SETUP_GUIDE.md)
- [ ] Do you know current phase? (Check PROGRESS.md)
- [ ] Do you understand the API? (Check API_DESIGN.md)
- [ ] Do you understand architecture? (Check ARCHITECTURE.md)

**When implementing Rust:**
- [ ] Added to `api.rs` for Flutter exposure?
- [ ] Used `Result<T, BimError>` for error handling?
- [ ] Avoided `unwrap()` in public APIs?
- [ ] Ran `cargo clippy` and `cargo fmt`?
- [ ] Generated FRB bindings?

**When implementing Flutter:**
- [ ] Used state management (Riverpod)?
- [ ] Followed widget composition?
- [ ] Handled async properly?
- [ ] Used const constructors?
- [ ] Ran `flutter analyze`?

**After completing tasks:**
- [ ] Updated PROGRESS.md?
- [ ] Added session notes?
- [ ] Documented any decisions?
- [ ] Committed changes?

---

## 🎓 Learning Path for New Contributors

If you're new to this stack:

1. **Flutter Basics** (if needed)
   - Complete Flutter's "First App" codelab
   - Understand StatelessWidget vs StatefulWidget
   - Learn about build() and setState()

2. **Rust Basics** (if needed)
   - Read Rust Book chapters 1-10
   - Understand ownership, borrowing, lifetimes
   - Practice with Result and Option types

3. **Flutter Rust Bridge**
   - Read FRB documentation
   - Understand how FFI works
   - Try the "hello world" example

4. **This Project**
   - Read all 6 planning documents in order
   - Follow SETUP_GUIDE.md
   - Start with Phase 1 tasks

---

## 🎯 Success Criteria

Project is successful when:
- ✅ Loads and displays IFC models correctly
- ✅ Renders at 60 FPS on target devices
- ✅ Works reliably on iOS and Android
- ✅ Map view shows building in correct location
- ✅ All Phase 1-7 features implemented
- ✅ Professional UI/UX
- ✅ No critical bugs
- ✅ Test coverage > 75%
- ✅ Published to App Store and Play Store

---

**This file should be updated whenever:**
- Project structure changes
- Major decisions are made
- New phases begin
- Technology choices change

**Last major update**: 2025-12-18 - Initial project planning complete
