# Flutter BIM Viewer - Development Roadmap

> **Package:** `flutter_bim` | **Renderer:** Rust/wgpu via flutter_rust_bridge | **Platforms:** Android & iOS (Web planned)
>
> Last updated: 2026-02-21

---

## Current State Summary

The viewer has a working Rust/wgpu 4x MSAA offscreen renderer with frustum culling (CPU + **GPU compute shader**), BVH picking, **multiple section planes** (up to 6 simultaneous, with animation), **section fill** (stencil-based cap rendering), section box clipping, edge rendering with depth-biased wireframe overlay, gamma-correct lighting (sRGB-linear workflow), **SSAO** (screen-space ambient occlusion), **shadow mapping** (directional light with PCF), **environment-based lighting**, and multi-model support. IFC STEP files are parsed and **real geometry is tessellated** (FacetedBrep, ExtrudedAreaSolid, RevolvedAreaSolid, SurfaceCurveSweptAreaSolid, TriangulatedFaceSet, PolygonalFaceSet, SweptDiskSolid, MappedItem, **BooleanResult with HalfSpaceClipping**, **B-spline curves**, and more — including C-shape and asymmetric I-shape profiles). Spatial hierarchy, property sets (including table values), type objects, materials, and quantity summaries are all parsed and displayed with **unit conversion** (SI ↔ Imperial). Visual analysis modes include X-ray/ghost, color-by-type/storey/material/property, grayscale, and **visual diff overlay**. Camera system supports orthographic projection, first-person walkthrough, turntable orbit, named viewpoints with smooth transitions, and **click-to-zoom** (element, storey, selection, type). Selection system supports multi-select, hide/isolate, selection sets, full-text search with **highlight-in-viewport**, and **smart groups** (saved filter criteria). **BCF 2.1/3.0** collaboration support with issues, viewpoints, comments, import/export, and **BCF API server integration** is implemented. **Clash detection** with AABB broad-phase, tolerance settings, grouping, and HTML/CSV reports is operational. **Model comparison** by GlobalID detects added/removed/modified elements with visual diff overlay and navigation. Export features include glTF 2.0, CSV properties, **HTML reports**, viewpoint sharing, and enhanced screenshots with metadata. **IFC georeferencing** (IFCMAPCONVERSION) is parsed. **File format imports** include OBJ, glTF/GLB, **DXF**, **LAS/LAZ point clouds**, and **PDF 2D drawings**. **IFC schema detection** handles IFC2x3, IFC4, and IFC4x3 with entity name normalization. **Streaming IFC parser** with parallel parsing, filtered/preview modes. **Annotations and markup** support pins, callouts, redlines, dimension lines, leader lines, and cloud regions with JSON persistence. **FXAA post-process** anti-aliasing and **screen-space size threshold** culling are implemented. **Parallel tessellation** via rayon and **geometry caching** to disk accelerate loading. **Criterion benchmarks** and **regression tests** cover parser and tessellator performance. GPU optimization includes **indirect draw**, **compute shader culling**, **compressed vertex format** (12-byte), **texture atlasing**, **vertex buffer streaming**, and **element geometry sharing**. **LOD-based geometry streaming** with 4-level chain and triangle budget. **Out-of-core rendering** with disk-backed geometry paging for 1GB+ files. **Memory tracking** with GPU/CPU estimates and diagnostic reports. **Touch gestures** refined with two-finger pan and three-finger orbit. **Accessibility** includes screen reader support (Semantics widgets with liveRegion), **high contrast mode**, **configurable text scaling** (0.8x–2.0x), and **dynamic type** (system font size passthrough). **CI/CD pipeline** with GitHub Actions (analyze, test, build-android), **automated Rust cross-compilation**, pre-built library caching, and **integration tests**. **Revit IFC export guidance** documented.

---

## Foundation (Complete)

- [x] IFC STEP file parser (nom combinators, ISO 10303-21)
- [x] 15 IFC entity types extracted (walls, slabs, columns, beams, doors, windows, roofs, stairs, footings, pipes, ducts, flow terminals, cable carriers, building element proxies, furniture)
- [x] Multi-model registry (load/unload/visibility per model)
- [x] wgpu offscreen renderer (RGBA pixel readback to Flutter)
- [x] GPU instanced rendering (shared unit box + per-instance transform/color)
- [x] Frustum culling (Gribb-Hartmann 6-plane extraction, per-instance AABB test)
- [x] BVH ray-cast picking (median-split, O(log n) closest-hit query)
- [x] Directional + ambient lighting (Lambert diffuse in WGSL)
- [x] Section plane clipping (half-space discard in fragment shader)
- [x] Wireframe mode (polygon mode Line, device-gated)
- [x] Camera: orbit, pan, zoom, fit-to-bounds, screen-to-ray
- [x] Element selection with highlight color
- [x] Element type visibility filtering
- [x] Structural grid lines (from IFC or auto-generated)
- [x] Screenshot export (PNG via `image` crate)
- [x] Measurement tool (distance, area, volume point collection)
- [x] Flutter UI: toolbar, element tree, model manager, properties panel
- [x] Section plane UI (axis selector + position slider)
- [x] Drawing overlay UI (stub backend)
- [x] Performance optimizations (zero-copy frame buffer, cached texture views, lazy BVH, pre-allocated Vecs, zero-alloc string matching)

---

## Phase 1: Real IFC Geometry

**Goal:** Replace placeholder boxes with actual building shapes from IFC data. This is the critical path — without real geometry, the viewer cannot display buildings correctly.

### 1.1 Placement & Coordinate Systems
- [x] Parse `IFCLOCALPLACEMENT` (relative placement chain)
- [x] Parse `IFCAXIS2PLACEMENT3D` (location + axis + ref direction → 4x4 matrix)
- [x] Parse `IFCAXIS2PLACEMENT2D` (for profile definitions)
- [x] Resolve placement hierarchy (child → parent chain up to world origin)
- [x] Parse `IFCCARTESIANPOINT` (2D and 3D)
- [x] Parse `IFCDIRECTION` (2D and 3D unit vectors)
- [x] Apply `IFCMAPCONVERSION` / `IFCPROJECTEDCRS` for georeferenced models

### 1.2 Profile Definitions (2D Cross-Sections)
- [x] Parse `IFCRECTANGLEPROFILEDEF` (width × depth rectangle)
- [x] Parse `IFCCIRCLEPROFILEDEF` (radius → polygon approximation)
- [x] Parse `IFCISHAPEPROFILEDEF` (I/H steel beams)
- [x] Parse `IFCLSHAPEPROFILEDEF` (angle sections)
- [x] Parse `IFCTSHAPEPROFILEDEF` (T-sections)
- [x] Parse `IFCUSHAPEPROFILEDEF` (channel sections)
- [x] Parse `IFCCSHAPEPROFILEDEF` (C-sections)
- [x] Parse `IFCASYMMETRICISHAPEPROFILEDEF`
- [x] Parse `IFCARBITRARYCLOSEDPROFILEDEF` (polyline outline → 2D polygon)
- [x] Parse `IFCARBITRARYPROFILEDEFWITHVOIDS` (outer boundary + inner holes)
- [x] Parse `IFCCOMPOSITEPROFILEDEF` (combined profiles)
- [x] Parse `IFCCIRCLEHOLLOWPROFILEDEF` (pipe/tube sections)
- [x] Parse `IFCRECTANGLEHOLLOWPROFILEDEF` (box sections)

### 1.3 Solid Geometry Tessellation
- [x] **`IFCEXTRUDEDAREASOLID`** — sweep profile along direction by depth (most common solid type, covers ~70% of BIM geometry)
- [x] `IFCREVOLVEDAREASOLID` — revolve profile around axis
- [x] `IFCSURFACECURVESWEPTAREASOLID` — sweep along a guide curve
- [x] `IFCSWEPTDISKSOLID` — sweep a circular cross-section along a curve (pipes, cables)
- [x] `IFCFACETEDBREP` — boundary representation with planar faces (direct triangle output)
- [x] `IFCTRIANGULATEDFACESET` (IFC4) — pre-tessellated triangles (direct passthrough)
- [x] `IFCPOLYGONALFACE` / `IFCINDEXEDPOLYGONALFACE` (IFC4)
- [x] `IFCSHELLBASEDSURFACEMODEL` — open/closed shell collections
- [x] `IFCFACEBASEDSURFACEMODEL` — face-based geometry

### 1.4 Boolean Operations
- [x] `IFCBOOLEANCLIPPINGRESULT` — tessellate first operand (full clipping is Phase 1b)
- [x] `IFCBOOLEANRESULT` — union/difference/intersection of two solids
- [x] `IFCHALFSPACESOLID` — infinite half-space for clipping (mesh-plane clipping)
- [x] `IFCPOLYGONALBOUNDEDHALFSPACE` — half-space bounded by polygon

### 1.5 Curves (for swept solids and profiles)
- [x] `IFCPOLYLINE` (connected line segments)
- [x] `IFCTRIMMEDCURVE` (trimmed arc/line)
- [x] `IFCCOMPOSITECURVE` (joined curve segments)
- [x] `IFCCIRCLE` (full circle, used in profiles and sweeps)
- [x] `IFCLINE` (parametric line)
- [x] `IFCBSPLINECURVE` / `IFCRATIONALBSPLINECURVEWITHKNOTS` (De Boor's algorithm)

### 1.6 Representation Dispatch
- [x] Parse `IFCPRODUCTDEFINITIONSHAPE` → `IFCSHAPEREPRESENTATION`
- [x] Resolve representation context (Body, Axis, FootPrint, BoundingBox)
- [x] Map `IFCSHAPEREPRESENTATION.RepresentationType` to tessellation path:
  - `SweptSolid` → extrusion/revolution
  - `Brep` → faceted BRep
  - `Tessellation` → indexed triangle set
  - `Clipping` → boolean operations
  - `MappedRepresentation` → shared geometry with transform
- [x] Parse `IFCMAPPEDITEM` (geometry reuse with `IFCCARTESIANTRANSFORMATIONOPERATOR3D`)
- [x] Fallback: generate bounding box when geometry type is unsupported

### Milestone: Buildings render with recognizable shapes, correct positions, and proper scale.

---

## Phase 2: Spatial Hierarchy & Properties

**Goal:** Navigate the building structure (Project → Site → Building → Storey → Elements) and display rich property data.

### 2.1 Spatial Structure
- [x] Parse `IFCRELAGGREGATES` (spatial decomposition tree)
- [x] Parse `IFCRELCONTAINEDINSPATIALSTRUCTURE` (elements → storey mapping)
- [x] Build spatial tree: Project → Site → Building → Storey → Elements
- [x] Storey navigation (isolate/highlight by floor level)
- [x] Spatial tree view in Flutter (expandable tree widget with By Type / By Storey toggle)
- [x] Click-to-zoom: select spatial node → fit camera to its contents

### 2.2 Property Sets
- [x] Parse `IFCPROPERTYSET` (named property containers)
- [x] Parse `IFCRELDEFINESBYPROPERTIES` (property set → object links)
- [x] Parse `IFCPROPERTYSINGLEVALUE` (name + nominal value + unit)
- [x] Parse `IFCPROPERTYENUMERATEDVALUE`
- [x] Parse `IFCPROPERTYLISTVALUE`
- [x] Parse `IFCPROPERTYBOUNDEDVALUE` (upper/lower bounds)
- [x] Parse `IFCPROPERTYTABLEVALUE`
- [x] Display property sets in Properties Panel (grouped by pset name, expandable sections)

### 2.3 Type Objects
- [x] Parse `IFCRELDEFINESBYTYPE` (element → type links)
- [x] Parse type entities (`IFCWALLTYPE`, `IFCSLABTYPE`, `IFCCOLUMNTYPE`, `IFCBEAMTYPE`, `IFCDOORTYPE`, `IFCWINDOWTYPE`, etc.)
- [x] Show type properties alongside instance properties
- [x] Group elements by type in element tree

### 2.4 Quantity Takeoffs
- [x] Parse `IFCELEMENTQUANTITY` (area, volume, weight, length, count)
- [x] Parse `IFCQUANTITYAREA`, `IFCQUANTITYLENGTH`, `IFCQUANTITYVOLUME`, `IFCQUANTITYWEIGHT`, `IFCQUANTITYCOUNT`
- [x] Unit conversion (SI ↔ Imperial) using `IFCUNITASSIGNMENT`
- [x] Quantity summary table (per type, per storey)

### 2.5 Materials
- [x] Parse `IFCMATERIAL` and `IFCMATERIALLAYER`
- [x] Parse `IFCMATERIALLAYERSET` / `IFCMATERIALLAYERSETUSAGE`
- [x] Parse `IFCRELASSOCIATESMATERIAL`
- [x] Display material info in Properties Panel
- [x] Color-by-material rendering mode

### Milestone: Full spatial navigation, rich property display, quantity summaries.

---

## Phase 3: Advanced Rendering

**Goal:** Visual quality and analysis tools matching professional BIM viewers.

### 3.1 Rendering Quality
- [x] Enable MSAA (sample count 4) with device capability check
- [x] Edge rendering (wireframe overlay with depth bias, dark semi-transparent edges)
- [x] Ambient occlusion (SSAO screen-space pass, 32 hemisphere samples, edge-aware blur, composite)
- [x] Shadow mapping (directional light shadow cascade, PCF, depth bias)
- [x] Environment-based lighting (IBL / environment uniform with sky/ground/horizon colors)
- [x] Anti-aliasing post-process (FXAA fallback for non-MSAA devices)
- [x] Gamma-correct rendering (sRGB→linear input conversion, linear lighting, sRGB output via texture format)

### 3.2 Visual Analysis Modes
- [x] X-ray / ghost mode (transparent with visible edges)
- [x] Color-by-type (type → color mapping applied to vertex buffer)
- [x] Color-by-storey (floor level → color gradient)
- [x] Color-by-material
- [x] Color-by-property (user-selected numeric property → gradient)
- [x] Grayscale mode (desaturate all except selected)

### 3.3 Section Tools
- [x] Section box (6-plane bounding box clipping)
- [x] Section fill (stencil-based cap with checkerboard pattern)
- [x] Multiple section planes (combine X + Y + Z simultaneously, up to 6)
- [x] Section animation (sweep plane through model, linear interpolation)

### 3.4 Level of Detail (LOD)
- [x] Distance-based LOD (5-level classification by camera distance)
- [x] Progressive loading (render low-detail first, refine)
- [x] Occlusion culling (hierarchical Z-buffer software occlusion)
- [x] Screen-space size threshold (skip sub-pixel elements)

### 3.5 Annotations & Markup
- [x] 3D text labels (element names, dimensions) — data structures + billboard rendering
- [x] Dimension lines (snapped to geometry edges/faces)
- [x] Leader lines (annotation → element connection)
- [x] 2D overlay annotations (pins, callouts, redlines, cloud regions)
- [x] Markup persistence (save/load annotation sets as JSON)

### Milestone: Publication-quality rendering with analysis visualization.

---

## Phase 4: Navigation & UX

**Goal:** Fluid navigation and professional interaction patterns.

### 4.1 Camera Modes
- [x] First-person walkthrough (WASD + mouse look)
- [x] Turntable orbit (constrained to vertical axis)
- [x] Orthographic projection toggle
- [x] Named viewpoints (save/restore camera state)
- [x] Smooth animated camera transitions (lerp position + target)
- [x] Touch gesture refinement (two-finger pan, three-finger orbit)

### 4.2 Selection & Isolation
- [x] Multi-select (long-press + tap, box select)
- [x] Isolate selection (hide everything else)
- [x] Hide selected / show all
- [x] Selection sets (named groups for quick recall)
- [x] Select by property filter (e.g., all elements where material = "Concrete")

### 4.3 Search & Filter
- [x] Full-text search across all properties
- [x] Filter by property value ranges
- [x] Smart groups (saved filter criteria with AND/OR logic)
- [x] Search results with highlight-in-viewport

### 4.4 Performance UX
- [x] Loading progress bar (parse → build → upload stages, MeshProgress FFI)
- [x] Async model loading (tokio::spawn background loading with state tracking)
- [x] Background model streaming (load visible storeys first)
- [x] Memory usage indicator (GPU/CPU estimates, diagnostic report)

### Milestone: Intuitive navigation comparable to desktop BIM viewers.

---

## Phase 5: Collaboration & Interoperability

**Goal:** BCF support, model comparison, and data exchange.

### 5.1 BCF (BIM Collaboration Format)
- [x] BCF 2.1 XML import/export
- [x] BCF 3.0 JSON support
- [x] Create issues with viewpoint snapshots
- [x] Navigate to saved viewpoints
- [x] Issue status tracking (open/in-progress/closed)
- [x] Comment threads on issues
- [x] Component selection/visibility in viewpoints
- [x] BCF API server integration (BIMcollab, Trimble Connect) — data layer with request builders, response parsers, sync helpers

### 5.2 Model Comparison
- [x] Diff two model versions (added/removed/modified elements)
- [x] Visual diff overlay (green = added, red = removed, yellow = modified)
- [x] Side-by-side and overlay comparison views
- [x] Change report generation

### 5.3 Clash Detection
- [x] AABB broad-phase collision detection
- [x] Mesh-level narrow-phase intersection testing
- [x] Clash grouping and categorization
- [x] Tolerance settings (hard clash, soft clash, clearance)
- [x] Clash report export (HTML/CSV)

### 5.4 Export & Sharing
- [x] Screenshot with metadata (PNG + camera/model info JSON)
- [x] HTML report generation (standalone HTML with embedded CSS, quantities, materials)
- [x] glTF/GLB export (for AR/VR handoff)
- [x] CSV/Excel property export
- [x] Share viewpoint links (deep link to camera + selection state)

### Milestone: Team collaboration workflows with issue tracking and model diffing.

---

## Phase 6: File Format Support

**Goal:** Go beyond IFC to support the broader BIM ecosystem.

### 6.1 IFC Schema Coverage
- [x] IFC2x3 full support (most common in practice) — schema detection + entity parsing
- [x] IFC4 ADD2 TC1 support (current standard) — schema detection + entity parsing
- [x] IFC4x3 support (infrastructure: bridges, roads, rails) — schema detection + entity normalization
- [x] Handle schema-specific entity differences gracefully (normalize_entity_name mapping)

### 6.2 Additional Formats
- [x] glTF/GLB import (pre-tessellated models, data URI + binary chunk)
- [x] OBJ import (simple mesh interchange, all face formats)
- [x] Point cloud (LAS/LAZ) overlay — LAS/LAZ parser, subsampling, classification coloring, spatial filtering
- [x] PDF 2D drawing overlay (floor plans) — minimal PDF parser extracting vector graphics (lines, rectangles, text)
- [x] DXF 2D import (site plans, details) — LINE/CIRCLE/ARC/POLYLINE/TEXT/INSERT entities
- [x] Revit lightweight format (via IFC export guidance) — docs/REVIT_FORMAT_GUIDE.md

### 6.3 Large File Handling
- [x] Streaming IFC parser (line-by-line parsing, no full DOM)
- [x] Geometry caching (serialize tessellated mesh to disk, mtime validation)
- [x] Memory-mapped file access for 1GB+ files (chunked parsing, memory estimation)
- [x] Background parsing with progress callbacks (AtomicUsize + MeshProgress)
- [x] Partial model loading (filter by type, preview mode, IFC summary scan)

### Milestone: Handle real-world project files from any major BIM authoring tool.

---

## Phase 7: Platform & Deployment

**Goal:** Production-ready mobile deployment on Android and iOS.

### 7.1 Platform Support
- [x] Android (ARM64) — working
- [x] Android (x86_64 emulator) — working
- [x] iOS (ARM64) — Rust cross-compilation + static library linking via `-force_load`
- [ ] iOS Simulator (x86_64 / ARM64)
- [ ] Web (WebGPU/WebGL via wasm) — future

### 7.2 Build Pipeline
- [x] Automated Rust cross-compilation for Android + iOS targets — .github/workflows/build-rust.yml, scripts/build_rust.sh
- [x] CI/CD pipeline (build + test + deploy) — .github/workflows/ci.yml (analyze, test-dart, test-rust, build-android)
- [x] Pre-built native library caching — GitHub Actions artifact caching
- [x] Release size optimization (strip debug symbols, LTO) — Cargo.toml profile.release
- [x] Automated integration tests on device/emulator — integration_test/app_test.dart

### 7.3 Accessibility
- [x] Screen reader support for element tree and properties — Semantics widgets with liveRegion for announcements
- [x] High contrast mode — ColorScheme.highContrastLight/Dark toggle
- [x] Configurable text scaling — Accessibility section in SettingsScreen with slider (0.8x–2.0x)
- [x] Dynamic type / system font size support — MediaQuery.textScaler passthrough or custom override

### Milestone: Ship on Android and iOS app stores with automated builds.

---

## Phase 8: Performance at Scale

**Goal:** Handle large real-world projects (100K+ elements, 1GB+ files) at interactive frame rates.

### 8.1 GPU Optimization
- [x] Indirect draw (GPU-driven rendering, IndirectDrawCommand + multi-draw buffer preparation)
- [x] Compute shader frustum culling (WGSL compute shader, AABB-vs-6-planes, 64-wide workgroups)
- [x] Geometry compression (vertex deduplication + Forsyth vertex cache optimization + degenerate removal)
- [x] Texture atlasing for material rendering (shelf-next-fit packing, solid color swatches, defragmentation)
- [x] Vertex buffer streaming (chunk-based spatial partitioning, priority upload, LRU eviction)

### 8.2 CPU Optimization
- [x] Parallel IFC parsing (rayon par_iter across entity lines)
- [x] Parallel tessellation (rayon work-stealing across elements)
- [x] Incremental BVH updates (insert/remove/update/refit without full rebuild)
- [x] Spatial hash for broad-phase queries (uniform grid, AABB/sphere/kNN)

### 8.3 Memory Optimization
- [x] LOD-based geometry streaming (LodManager with 4-level chain, hysteresis, triangle budget)
- [x] Element geometry sharing (MappedItem deduplication via GeometryCache)
- [x] Compressed vertex format (12-byte CompressedVertex: f16 positions, octahedral normals, RGB565 color)
- [x] Out-of-core rendering (disk-backed geometry paging, LRU eviction, frustum-based loading)

### 8.4 Benchmarking
- [x] Criterion benchmarks for parser, tessellator, renderer
- [x] Frame time profiling (per-frame render time tracking, peak detection)
- [x] Memory usage tracking (GPU/CPU estimation, per-model stats)
- [x] Regression testing against reference models (10 integration regression tests)

### Milestone: 60fps with 100K+ elements on mid-range Android/iOS devices.

---

## Priority Order

| Priority | Phase | Impact | Effort |
|----------|-------|--------|--------|
| **P0** | 1.1 Placement & Coordinates | Unlocks all real geometry | Medium |
| **P0** | 1.2 Profiles (rectangle + arbitrary) | Needed for extrusions | Medium |
| **P0** | 1.3 ExtrudedAreaSolid + FacetedBrep | Covers ~80% of real geometry | Large |
| **P0** | 1.6 Representation Dispatch | Routes shapes to tessellation | Medium |
| **P1** | 2.1 Spatial Hierarchy | Floor navigation | Medium |
| **P1** | 2.2 Property Sets | Rich data display | Medium |
| **P1** | 1.3 Boolean Operations | Door/window openings | Large |
| **P1** | 1.3 TriangulatedFaceSet | IFC4 geometry | Small |
| **P2** | 3.1 Edge Rendering | Visual quality | Medium |
| **P2** | 3.2 X-ray Mode | Analysis tool | Small |
| **P2** | 3.3 Section Box | Common analysis | Medium |
| **P2** | 4.1 Orthographic + Walkthrough | Navigation | Medium |
| **P2** | 2.5 Materials | Visual + data | Medium |
| **P3** | 5.1 BCF Support | Collaboration | Large |
| **P3** | 5.3 Clash Detection | Coordination | Large |
| **P3** | 6.1 IFC4 Full Support | Compatibility | Large |
| **P4** | 8.1 GPU-driven Rendering | Scale | Large |

---

## Reference: Professional BIM Viewer Features

The following features are found in leading BIM viewers (Navisworks, Solibri, BIMcollab ZOOM, xeokit, Open IFC Viewer) and inform this roadmap:

| Feature | Navisworks | Solibri | BIMcollab | xeokit | This Viewer |
|---------|:---:|:---:|:---:|:---:|:---:|
| IFC geometry rendering | Yes | Yes | Yes | Yes | Yes |
| Spatial tree navigation | Yes | Yes | Yes | Yes | Yes |
| Property set display | Yes | Yes | Yes | Yes | Yes |
| Section planes/box | Yes | Yes | Yes | Yes | Yes (6 planes) |
| Section animation | No | No | No | Yes | Yes |
| BCF issues | No | Yes | Yes | Yes | Yes |
| Clash detection | Yes | Yes | No | Plugin | Yes |
| Multi-model federated | Yes | Yes | Yes | Yes | Yes |
| Annotations/markup | Yes | Yes | Yes | Yes | Yes |
| Measurement tools | Yes | Yes | Yes | Yes | Partial |
| Color-by rules | Yes | Yes | Yes | Yes | Yes |
| Quantity takeoffs | Yes | Yes | No | Plugin | Yes |
| Unit conversion | Yes | Yes | Yes | Yes | Yes |
| Model comparison | Yes | Yes | No | No | Yes |
| Visual diff overlay | Yes | No | No | No | Yes |
| Smart groups/filters | Yes | Yes | Yes | Yes | Yes |
| Click-to-zoom | Yes | Yes | Yes | Yes | Yes |
| First-person walkthrough | Yes | No | No | Yes | Yes |
| Edge rendering | Yes | Yes | Yes | Yes | Yes |
| Boolean geometry | Yes | Yes | Yes | Yes | Yes |
| B-spline curves | Yes | Yes | Yes | Yes | Yes |
| glTF export | No | No | No | Yes | Yes |
| CSV property export | Yes | Yes | No | No | Yes |
| HTML reports | Yes | Yes | No | No | Yes |
| OBJ/glTF import | No | No | No | Yes | Yes |
| IFC2x3/4/4x3 schemas | Yes | Yes | Yes | Yes | Yes |
| FXAA post-process | Yes | Yes | Yes | Yes | Yes |
| Point cloud support | Yes | No | No | Yes | Yes |
| 60fps at 100K elements | Yes | Yes | Yes | Yes | Untested |
