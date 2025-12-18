# Development Environment Setup Guide

## Prerequisites

### Required Software

1. **Flutter SDK** (Latest Stable)
   - Download: https://flutter.dev/docs/get-started/install
   - Minimum version: 3.16.0
   - Verify: `flutter --version`

2. **Rust Toolchain**
   - Install rustup: https://rustup.rs/
   - Minimum version: 1.75.0
   - Verify: `rustc --version` and `cargo --version`

3. **Git**
   - Download: https://git-scm.com/downloads
   - Verify: `git --version`

### Platform-Specific Requirements

#### macOS (Required for iOS development)
```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install Homebrew (if not already installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Add Rust targets
rustup target add aarch64-apple-darwin  # For Apple Silicon
rustup target add x86_64-apple-darwin   # For Intel Macs
rustup target add aarch64-apple-ios     # For iOS
rustup target add x86_64-apple-ios      # For iOS Simulator
```

#### Android Development (Required)
Both macOS and Windows users need Android development tools:

```bash
# Android Studio will install required SDK components
# Or manually install:
# - Android SDK Platform (API 21+)
# - Android SDK Build-Tools
# - Android NDK

# Add Rust targets for Android
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android

# Install cargo-ndk for Android builds
cargo install cargo-ndk
```

### IDE Setup

#### Visual Studio Code (Recommended)

**Required Extensions**
```json
{
  "recommendations": [
    "Dart-Code.flutter",
    "Dart-Code.dart-code",
    "rust-lang.rust-analyzer",
    "vadimcn.vscode-lldb",
    "tamasfe.even-better-toml",
    "serayuzgur.crates"
  ]
}
```

**Install Extensions**
```bash
code --install-extension Dart-Code.flutter
code --install-extension rust-lang.rust-analyzer
code --install-extension vadimcn.vscode-lldb
```

**Workspace Settings** (`.vscode/settings.json`)
```json
{
  "dart.flutterSdkPath": "[path-to-flutter-sdk]",
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy",
  "[rust]": {
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "[dart]": {
    "editor.formatOnSave": true,
    "editor.selectionHighlight": false,
    "editor.suggest.snippetsPreventQuickSuggestions": false,
    "editor.suggestSelection": "first",
    "editor.tabCompletion": "onlySnippets",
    "editor.wordBasedSuggestions": "off"
  }
}
```

#### Android Studio (Alternative)

**Required Plugins**
- Flutter plugin
- Dart plugin
- Rust plugin

## Project Setup

### Step 1: Create Flutter Project

```bash
# Navigate to your workspace
cd ~/projects  # or C:\Users\YourName\projects on Windows

# Create new Flutter project
flutter create --org com.yourcompany bim_viewer

# Navigate into project
cd bim_viewer

# Verify Flutter setup
flutter doctor -v
```

### Step 2: Initialize Rust Library

```bash
# Create rust directory
mkdir rust
cd rust

# Initialize Cargo project
cargo init --lib

# Verify Rust setup
cargo build
```

### Step 3: Configure Flutter Dependencies

**Edit `pubspec.yaml`**
```yaml
name: bim_viewer
description: A BIM viewer application with Rust backend
publish_to: 'none'
version: 0.1.0+1

environment:
  sdk: '>=3.2.0 <4.0.0'

dependencies:
  flutter:
    sdk: flutter

  # Flutter Rust Bridge
  flutter_rust_bridge: ^2.0.0
  ffi: ^2.1.0

  # State Management
  flutter_riverpod: ^2.4.0

  # UI
  cupertino_icons: ^1.0.6

  # File handling
  file_picker: ^6.1.1
  path_provider: ^2.1.1

  # Utilities
  logger: ^2.0.2
  freezed_annotation: ^2.4.1
  json_annotation: ^4.8.1

dev_dependencies:
  flutter_test:
    sdk: flutter
  flutter_lints: ^3.0.1

  # Code generation
  build_runner: ^2.4.6
  freezed: ^2.4.5
  json_serializable: ^6.7.1
  ffigen: ^9.0.1

  # Integration testing
  integration_test:
    sdk: flutter

flutter:
  uses-material-design: true
```

**Install dependencies**
```bash
flutter pub get
```

### Step 4: Configure Rust Dependencies

**Edit `rust/Cargo.toml`**
```toml
[package]
name = "rust"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "staticlib"]

[dependencies]
# Flutter Rust Bridge
flutter_rust_bridge = "2.0.0"

# Async runtime
tokio = { version = "1.35", features = ["full"] }

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Graphics
wgpu = "0.18"
winit = "0.29"

# Math
nalgebra = "0.32"
glam = "0.25"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# File parsing
nom = "7.1"

# Spatial indexing
rstar = "0.11"

[dev-dependencies]
criterion = "0.5"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1

[profile.dev]
opt-level = 0

[[bench]]
name = "rendering"
harness = false
```

**Install Rust dependencies**
```bash
cd rust
cargo build
```

### Step 5: Install Flutter Rust Bridge CLI

```bash
# Install FRB codegen tool
cargo install flutter_rust_bridge_codegen

# Verify installation
flutter_rust_bridge_codegen --version
```

### Step 6: Setup FRB Configuration

**Create `rust/flutter_rust_bridge.yaml`**
```yaml
rust_input:
  - rust/src/api.rs
dart_output:
  - lib/bridge/ffi.dart
c_output:
  - ios/Runner/bridge_generated.h
  - macos/Runner/bridge_generated.h
extra:
  rust_crate_dir: rust
  rust_output_dir: rust/src/bridge
```

### Step 7: Create Initial Rust API

**Create `rust/src/api.rs`**
```rust
use flutter_rust_bridge::frb;

/// Initialize the BIM viewer
#[frb(sync)]
pub fn initialize() -> String {
    "BIM Viewer initialized".to_string()
}

/// Get library version
#[frb(sync)]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Test async functionality
pub async fn test_async() -> String {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    "Async test completed".to_string()
}
```

**Update `rust/src/lib.rs`**
```rust
mod api;
mod bridge;

pub use api::*;
```

### Step 8: Generate Bridge Code

```bash
# From project root
flutter_rust_bridge_codegen generate

# This creates:
# - lib/bridge/ffi.dart (Dart bindings)
# - rust/src/bridge/ (Rust glue code)
# - Platform-specific headers
```

### Step 9: Configure Platform Builds

#### Android Configuration

**Edit `android/app/build.gradle`**
```gradle
android {
    // ... existing config

    ndkVersion flutter.ndkVersion

    defaultConfig {
        // ... existing config
        ndk {
            abiFilters 'armeabi-v7a', 'arm64-v8a', 'x86_64'
        }
    }
}
```

**Create `android/app/build.gradle.kts` for Rust**
```kotlin
tasks.register("buildRust") {
    doLast {
        exec {
            workingDir = file("../../")
            commandLine("flutter_rust_bridge_codegen", "build-android")
        }
    }
}

tasks.named("preBuild") {
    dependsOn("buildRust")
}
```

#### iOS Configuration

**Edit `ios/Podfile`**
```ruby
# Add before target 'Runner' do
platform :ios, '13.0'

target 'Runner' do
  use_frameworks!
  use_modular_headers!

  flutter_install_all_ios_pods File.dirname(File.realpath(__FILE__))

  # Add Rust library
  pod 'rust_lib', :path => '../rust'
end

# Add build script
post_install do |installer|
  installer.pods_project.targets.each do |target|
    flutter_additional_ios_build_settings(target)
  end

  # Build Rust library
  system("cd .. && flutter_rust_bridge_codegen build-ios")
end
```


### Step 10: Create Test Flutter App

**Update `lib/main.dart`**
```dart
import 'package:flutter/material.dart';
import 'bridge/ffi.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'BIM Viewer',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.blue),
        useMaterial3: true,
      ),
      home: const HomePage(),
    );
  }
}

class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  String _message = '';

  @override
  void initState() {
    super.initState();
    _initialize();
  }

  Future<void> _initialize() async {
    final message = await initialize();
    final version = await getVersion();
    setState(() {
      _message = '$message\nVersion: $version';
    });
  }

  Future<void> _testAsync() async {
    final result = await testAsync();
    setState(() {
      _message = result;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('BIM Viewer'),
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
      ),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Text(
              _message,
              style: Theme.of(context).textTheme.headlineSmall,
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 20),
            ElevatedButton(
              onPressed: _testAsync,
              child: const Text('Test Async'),
            ),
          ],
        ),
      ),
    );
  }
}
```

### Step 11: Build and Run

```bash
# Generate bridge code
flutter_rust_bridge_codegen generate

# Build Rust library
cd rust
cargo build
cd ..

# Run Flutter app
flutter run

# Or for specific platform
flutter run -d android
flutter run -d ios
```

## Verification Checklist

- [ ] Flutter doctor shows no critical issues
- [ ] Rust compiles without errors
- [ ] FRB generates code successfully
- [ ] App runs on at least one platform
- [ ] Rust functions callable from Flutter
- [ ] No FFI-related crashes
- [ ] Hot reload works correctly

## Troubleshooting

### Issue: "flutter_rust_bridge not found"
```bash
# Ensure it's in pubspec.yaml and run
flutter pub get
```

### Issue: "cargo not found"
```bash
# Ensure Rust is installed and in PATH
rustup default stable
export PATH="$HOME/.cargo/bin:$PATH"  # Linux/Mac
# or add to PATH on Windows
```

### Issue: Build fails on Android
```bash
# Install required targets
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add i686-linux-android
rustup target add x86_64-linux-android

# Install cargo-ndk
cargo install cargo-ndk
```

### Issue: Hot reload doesn't work
- Restart the app completely when changing Rust code
- Only Dart changes support hot reload
- Use `flutter run` after Rust changes

## Next Steps

After successful setup:
1. Review the [ARCHITECTURE.md](ARCHITECTURE.md) document
2. Follow the [BIM_VIEWER_PLAN.md](BIM_VIEWER_PLAN.md) implementation steps
3. Start with Phase 1: Foundation features
4. Set up CI/CD for automated builds

## Useful Commands Reference

```bash
# Flutter
flutter doctor -v              # Check Flutter installation
flutter clean                  # Clean build artifacts
flutter pub get                # Install dependencies
flutter run -d [device]        # Run on specific device
flutter build [platform]       # Build release

# Rust
cargo build                    # Build debug
cargo build --release          # Build release
cargo test                     # Run tests
cargo clippy                   # Lint code
cargo fmt                      # Format code

# FRB
flutter_rust_bridge_codegen generate    # Generate bindings
flutter_rust_bridge_codegen clean       # Clean generated files

# Combined workflow
flutter_rust_bridge_codegen generate && flutter run
```

## Resources

- Flutter Documentation: https://flutter.dev/docs
- Rust Book: https://doc.rust-lang.org/book/
- Flutter Rust Bridge: https://cjycode.com/flutter_rust_bridge/
- wgpu Tutorial: https://sotrh.github.io/learn-wgpu/

---

Setup complete! You're ready to start building the BIM viewer.
