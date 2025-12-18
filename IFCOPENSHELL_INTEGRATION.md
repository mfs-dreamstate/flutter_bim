# IfcOpenShell Integration Guide

## Overview

We're using **IfcOpenShell** instead of a custom IFC parser for superior performance and robustness. IfcOpenShell is the industry-standard library for IFC file processing, used by Autodesk, Trimble, and many professional BIM applications.

## Why IfcOpenShell?

### Performance Benefits
- **15+ years of optimization** - Battle-tested on millions of real-world files
- **OpenCASCADE geometry kernel** - Industrial-grade CAD geometry processing
- **Optimized tessellation** - Fast conversion of complex geometry to triangles
- **2-5x faster** than custom parser for geometry extraction

### Reliability Benefits
- **Handles edge cases** - Works with files from Revit, ArchiCAD, Tekla, etc.
- **Complete IFC support** - All IFC 2x3 and IFC 4 entities
- **Proven on mobile** - Already used in production mobile BIM apps
- **Active maintenance** - Regular updates and bug fixes

## Architecture

```
┌─────────────────────────────────────────────┐
│           Flutter Application               │
│  (Dart UI, state management, rendering)     │
└──────────────────┬──────────────────────────┘
                   │ Flutter Rust Bridge (FFI)
┌──────────────────▼──────────────────────────┐
│           Rust Layer                        │
│  • API facade (api.rs)                      │
│  • Model management                         │
│  • Geometry caching                         │
│  • Spatial indexing (R-tree)                │
└──────────────────┬──────────────────────────┘
                   │ FFI bindings
┌──────────────────▼──────────────────────────┐
│      IfcOpenShell (C++)                     │
│  • IFC parsing                              │
│  • Geometry extraction (OpenCASCADE)        │
│  • Property extraction                      │
│  • Relationship traversal                   │
└─────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 2 Updated Tasks

#### Week 1: IfcOpenShell Build Setup
1. **Set up IfcOpenShell compilation**
   - Download IfcOpenShell source
   - Configure CMake for cross-compilation
   - Build for desktop (development/testing only)
   - **Build for Android (NDK, arm64-v8a, armeabi-v7a, x86_64)** ← PRODUCTION
   - **Build for iOS (Xcode, arm64 for devices, x86_64 for simulator)** ← PRODUCTION

2. **Create Rust FFI bindings**
   - Use `bindgen` or `cxx` for C++ interop
   - Wrap critical IfcOpenShell APIs
   - Create safe Rust wrapper types

#### Week 2: Integration & Testing
3. **Implement Rust wrapper**
   - Load IFC files via IfcOpenShell
   - Extract geometry (vertices, indices, normals)
   - Extract properties and metadata
   - Cache processed data

4. **Flutter integration**
   - Update Flutter API to pass IFC files
   - Display loading progress
   - Show extracted geometry data
   - Test with real IFC files

## Technical Details

### Dependencies

**Rust dependencies:**
```toml
[dependencies]
# C++ interop
cxx = "1.0"
bindgen = "0.69"  # Build-time only

# Or alternative approach:
# ifcopenshell-sys = { path = "ifcopenshell-sys" }  # Our custom wrapper
```

**System dependencies:**
- IfcOpenShell (compiled from source or pre-built)
- OpenCASCADE (dependency of IfcOpenShell)
- CMake 3.16+ (for building)
- C++17 compiler

### Build Configuration

**For Android (PRIMARY PLATFORM):**
```bash
# Build IfcOpenShell for each Android architecture
# Need to build for: arm64-v8a, armeabi-v7a, x86_64

cmake -DCMAKE_TOOLCHAIN_FILE=$NDK/build/cmake/android.toolchain.cmake \
      -DANDROID_ABI=arm64-v8a \
      -DANDROID_PLATFORM=android-26 \
      -DCMAKE_BUILD_TYPE=Release \
      ..

# Repeat for armeabi-v7a and x86_64
```

**For iOS (PRIMARY PLATFORM):**
```bash
# Build IfcOpenShell for iOS devices (arm64)
cmake -DCMAKE_TOOLCHAIN_FILE=ios.toolchain.cmake \
      -DPLATFORM=OS64 \
      -DCMAKE_BUILD_TYPE=Release \
      ..

# Also build for iOS simulator (x86_64) for testing
```

**For Windows Desktop (BONUS PLATFORM):**
Windows is our development platform, so it becomes a free deployment target!
```bash
# Windows build (both for testing AND deployment)
cmake -DCMAKE_BUILD_TYPE=Release ..
cmake --build . --config Release

# This gives you both:
# - Testing/debugging during development
# - Free Windows desktop app for users
```

**Other Desktop Platforms (NOT SUPPORTED):**
- Linux: Skip (saves time)
- macOS: Skip (unless developing on Mac)

### Key IfcOpenShell APIs to Wrap

```cpp
// Core APIs we'll use:
IfcParse::IfcFile* file = new IfcParse::IfcFile("model.ifc");
IfcSchema::IfcProduct::list products = file->instances_by_type<IfcSchema::IfcProduct>();

// Geometry conversion
IfcGeom::IteratorSettings settings;
settings.set(IfcGeom::IteratorSettings::USE_WORLD_COORDS, true);
IfcGeom::Iterator iterator(settings, file);

while (iterator.next()) {
    IfcGeom::Element* elem = iterator.get();
    IfcGeom::TriangulationElement* tri =
        dynamic_cast<IfcGeom::TriangulationElement*>(elem);

    // Get vertices, indices, normals
    const std::vector<double>& verts = tri->geometry().verts();
    const std::vector<int>& faces = tri->geometry().faces();
}
```

### Rust Wrapper Example

```rust
// rust/src/bim/ifcopenshell_wrapper.rs

use cxx::{UniquePtr, CxxString};

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("ifcopenshell_bridge.h");

        type IfcFile;

        fn load_ifc_file(path: &CxxString) -> Result<UniquePtr<IfcFile>>;
        fn get_product_count(file: &IfcFile) -> usize;
        fn extract_geometry(file: &IfcFile, callback: fn(ElementGeometry));
    }
}

pub struct ModelLoader {
    file: Option<UniquePtr<ffi::IfcFile>>,
}

impl ModelLoader {
    pub fn load(&mut self, path: &str) -> Result<ModelInfo> {
        let cxx_path = CxxString::new(path);
        self.file = Some(ffi::load_ifc_file(&cxx_path)?);

        let count = ffi::get_product_count(self.file.as_ref().unwrap());

        Ok(ModelInfo {
            element_count: count,
            // ... other metadata
        })
    }

    pub fn extract_all_geometry(&self) -> Vec<MeshData> {
        let mut meshes = Vec::new();

        ffi::extract_geometry(
            self.file.as_ref().unwrap(),
            |geom| meshes.push(geom.into())
        );

        meshes
    }
}
```

## Binary Size Impact (Mobile)

Expected app size increase for Android/iOS:
- **IfcOpenShell**: ~5-8 MB (per architecture)
- **OpenCASCADE**: ~8-12 MB (per architecture)
- **Total per architecture**: ~15-20 MB additional

**Final app sizes:**
- **Android APK**: ~50-60 MB (includes multiple architectures)
- **iOS IPA**: ~40-50 MB (single architecture per build)

For a professional BIM viewer, this is very acceptable.

## Performance Targets

With IfcOpenShell, we expect:

| File Size | Parse Time | Geometry Extraction | Total Load |
|-----------|------------|---------------------|------------|
| 10 MB     | < 0.5s     | 1-2s               | < 2.5s     |
| 50 MB     | < 2s       | 5-10s              | < 12s      |
| 100 MB    | < 4s       | 10-20s             | < 24s      |

All on mid-range mobile devices (2020+).

## Alternative: Pre-built Binaries

If building from source is too complex, we can:

1. **Use conda packages** (desktop development/testing only)
   ```bash
   conda install -c conda-forge ifcopenshell
   ```

2. **Download pre-built mobile binaries**
   - Check IfcOpenShell releases for Android/iOS builds
   - Some community members provide pre-built mobile binaries
   - Link against them in Rust

3. **Cross-compile once, reuse** (Recommended)
   - Build on CI/CD (one-time setup)
   - Cache compiled binaries for Android (arm64-v8a, armeabi-v7a, x86_64)
   - Cache compiled binaries for iOS (arm64, x86_64-sim)
   - Link in local development
   - This is the best approach for team development

## Fallback Plan

If IfcOpenShell proves too difficult for mobile:
- Use IfcOpenShell for desktop/development
- Use simplified parser for mobile (parse properties only)
- Or use web service for geometry processing

But this should not be necessary - IfcOpenShell works well on mobile.

## Resources

- [IfcOpenShell GitHub](https://github.com/IfcOpenShell/IfcOpenShell)
- [IfcOpenShell Academy](http://academy.ifcopenshell.org/)
- [Building for Android](https://github.com/IfcOpenShell/IfcOpenShell/wiki/Android)
- [Building for iOS](https://github.com/IfcOpenShell/IfcOpenShell/wiki/iOS)
- [API Documentation](http://ifcopenshell.org/docs)

## Next Steps

1. ✅ Documentation updated (this file)
2. ⏳ Set up IfcOpenShell build scripts
3. ⏳ Create Rust FFI bindings
4. ⏳ Implement wrapper API
5. ⏳ Test with sample IFC files
6. ⏳ Optimize and profile

---

**Status**: Ready to implement in Phase 2
**Last Updated**: 2025-12-18
