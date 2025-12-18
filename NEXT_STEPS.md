# Next Steps - BIM Viewer Setup

## Current Status
✅ **Phase 1 Setup: 85% Complete**

Most of the foundation is ready! The project structure, Flutter app, and Rust library skeleton are all created. We just need to install Visual Studio C++ components and generate the FFI bridge.

---

## What's Done ✅

1. ✅ Flutter project created with all platforms
2. ✅ All Flutter dependencies installed (flutter_rust_bridge, riverpod, flutter_map, etc.)
3. ✅ Rust library structure created (`rust/` directory)
4. ✅ Rust API functions written (initialize, get_version, test_async, etc.)
5. ✅ Flutter main.dart with test UI created
6. ✅ Flutter Rust Bridge configuration file created
7. ✅ Android NDK configuration added
8. ✅ Project directory structure created (lib/features/, rust/src/, etc.)

---

## What's Needed Next ⏳

### Step 1: Install Visual Studio C++ Components (Required)

Rust needs Microsoft Visual C++ Build Tools to compile on Windows.

**Instructions:**
1. Open **Visual Studio Installer** (search in Start menu)
2. Click **Modify** on "Visual Studio Community 2022"
3. Check the box for **"Desktop development with C++"**
4. In the right panel, ensure these are selected:
   - ✅ MSVC v143 - VS 2022 C++ x64/x86 build tools (latest)
   - ✅ Windows 11 SDK (or Windows 10 SDK)
   - ✅ C++ CMake tools for Windows
5. Click **Modify** and wait for installation
6. **Restart your terminal** after installation completes

**Estimated time:** 10-15 minutes

---

### Step 2: Switch Back to MSVC Toolchain

Once Visual Studio C++ components are installed, switch Rust back to the MSVC toolchain:

```bash
rustup default stable-x86_64-pc-windows-msvc
```

---

### Step 3: Install Flutter Rust Bridge Codegen

```bash
cargo install flutter_rust_bridge_codegen
```

This will take 5-10 minutes to compile and install.

---

### Step 4: Add Android Rust Targets

```bash
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
```

---

### Step 5: Install cargo-ndk (for Android builds)

```bash
cargo install cargo-ndk
```

---

### Step 6: Generate FFI Bridge Code

From the project root directory:

```bash
flutter_rust_bridge_codegen generate
```

This will create:
- `lib/core/bridge/ffi.dart` (Dart bindings)
- `rust/src/bridge/` (Rust glue code)
- Platform-specific C headers

---

### Step 7: Update Flutter main.dart

Uncomment the following in `lib/main.dart`:
1. Line 3: `import 'core/bridge/ffi.dart' as rust;`
2. Lines 53-54: `_initialize();` in initState
3. Lines 58-111: All the Rust function calls (_initialize, _testAsync, etc.)

---

### Step 8: Build and Test

```bash
# Build Rust library
cd rust
cargo build
cd ..

# Run Flutter app
flutter run
```

**Or run on specific device:**
```bash
flutter run -d windows     # Windows desktop
flutter run -d android     # Android emulator/device
```

---

## Expected Result

Once all steps are complete, you should see:
- ✅ App launches with "BIM Viewer - Phase 1 Test" screen
- ✅ Status shows "BIM Viewer initialized successfully"
- ✅ Version shows "v0.1.0"
- ✅ System information displays Rust version and OS
- ✅ Test buttons work (Test Async, Test Error)

---

## Troubleshooting

### Issue: "flutter_rust_bridge_codegen not found"
**Solution:** Make sure `~/.cargo/bin` is in your PATH. Restart terminal after installing.

### Issue: "dlltool.exe not found" or linker errors
**Solution:** Visual Studio C++ components not installed. Follow Step 1 above.

### Issue: "Could not find Rust library"
**Solution:** Run `cargo build` in the `rust/` directory first.

### Issue: Hot reload doesn't work
**Expected:** Hot reload only works for Flutter code. After Rust changes, you need to:
1. Run `cargo build`
2. Run `flutter_rust_bridge_codegen generate`
3. Fully restart the app with `flutter run`

---

## Quick Reference Commands

```bash
# Check environment
flutter doctor -v
rustc --version
cargo --version
flutter_rust_bridge_codegen --version

# Build Rust (debug)
cd rust && cargo build

# Build Rust (release)
cd rust && cargo build --release

# Generate FRB bindings
flutter_rust_bridge_codegen generate

# Run Flutter app
flutter run

# Run Flutter on specific device
flutter run -d <device-id>
flutter devices  # List available devices

# Clean build
flutter clean
cd rust && cargo clean

# Format code
cargo fmt                # Rust
dart format lib/         # Dart

# Lint code
cargo clippy             # Rust
flutter analyze          # Dart
```

---

## After Phase 1 is Complete

Once you can successfully run the app and call Rust functions, Phase 1 is complete!

**Next phases:**
- **Phase 2:** IFC file parsing (Weeks 3-4)
- **Phase 3:** 3D rendering with wgpu (Weeks 5-6)
- **Phase 4:** Materials and lighting (Week 7)
- **Phase 5:** Interaction and selection (Week 8)
- **Phase 6:** 2D GIS integration (Week 9)

See `QUICK_START.md` for detailed phase breakdowns.

---

## Questions?

- Check `SETUP_GUIDE.md` for detailed setup instructions
- Check `ARCHITECTURE.md` for system design details
- Check `API_DESIGN.md` for Rust API reference
- Check `PROGRESS.md` for current status

---

**You're almost there! Just need to install Visual Studio C++ components and you'll be ready to go! 🚀**
