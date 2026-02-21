# Flutter BIM Viewer - Development Roadmap

> **Package:** `flutter_bim` | **Renderer:** Rust/wgpu via flutter_rust_bridge | **Platforms:** Android & iOS (Web planned)
>
> Last updated: 2026-02-21

---

## Current State Summary

The viewer has a working Rust/wgpu offscreen renderer with frustum culling, BVH picking, section plane clipping, and multi-model support. IFC STEP files are parsed and **real geometry is tessellated** (FacetedBrep, ExtrudedAreaSolid, RevolvedAreaSolid, TriangulatedFaceSet, PolygonalFaceSet, SweptDiskSolid, MappedItem, and more). Spatial hierarchy (Project → Site → Building → Storey → Elements), IFC property sets, type objects, and materials are all parsed and displayed. Visual analysis modes include X-ray/ghost rendering and color-by-type/storey/material. Storey isolation lets users focus on individual floors. The next gaps are boolean operations, section box, edge rendering, and navigation improvements.

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
- [ ] Apply `IFCMAPCONVERSION` / `IFCPROJECTEDCRS` for georeferenced models

### 1.2 Profile Definitions (2D Cross-Sections)
- [x] Parse `IFCRECTANGLEPROFILEDEF` (width × depth rectangle)
- [x] Parse `IFCCIRCLEPROFILEDEF` (radius → polygon approximation)
- [x] Parse `IFCISHAPEPROFILEDEF` (I/H steel beams)
- [x] Parse `IFCLSHAPEPROFILEDEF` (angle sections)
- [x] Parse `IFCTSHAPEPROFILEDEF` (T-sections)
- [x] Parse `IFCUSHAPEPROFILEDEF` (channel sections)
- [ ] Parse `IFCCSHAPEPROFILEDEF` (C-sections)
- [ ] Parse `IFCASYMMETRICISHAPEPROFILEDEF`
- [x] Parse `IFCARBITRARYCLOSEDPROFILEDEF` (polyline outline → 2D polygon)
- [x] Parse `IFCARBITRARYPROFILEDEFWITHVOIDS` (outer boundary + inner holes)
- [x] Parse `IFCCOMPOSITEPROFILEDEF` (combined profiles)
- [x] Parse `IFCCIRCLEHOLLOWPROFILEDEF` (pipe/tube sections)
- [x] Parse `IFCRECTANGLEHOLLOWPROFILEDEF` (box sections)

### 1.3 Solid Geometry Tessellation
- [x] **`IFCEXTRUDEDAREASOLID`** — sweep profile along direction by depth (most common solid type, covers ~70% of BIM geometry)
- [x] `IFCREVOLVEDAREASOLID` — revolve profile around axis
- [ ] `IFCSURFACECURVESWEPTAREASOLID` — sweep along a guide curve
- [x] `IFCSWEPTDISKSOLID` — sweep a circular cross-section along a curve (pipes, cables)
- [x] `IFCFACETEDBREP` — boundary representation with planar faces (direct triangle output)
- [x] `IFCTRIANGULATEDFACESET` (IFC4) — pre-tessellated triangles (direct passthrough)
- [x] `IFCPOLYGONALFACE` / `IFCINDEXEDPOLYGONALFACE` (IFC4)
- [x] `IFCSHELLBASEDSURFACEMODEL` — open/closed shell collections
- [x] `IFCFACEBASEDSURFACEMODEL` — face-based geometry

### 1.4 Boolean Operations
- [x] `IFCBOOLEANCLIPPINGRESULT` — tessellate first operand (full clipping is Phase 1b)
- [ ] `IFCBOOLEANRESULT` — union/difference/intersection of two solids
- [ ] `IFCHALFSPACESOLID` — infinite half-space for clipping
- [ ] `IFCPOLYGONALBOUNDEDHALFSPACE` — half-space bounded by polygon

### 1.5 Curves (for swept solids and profiles)
- [x] `IFCPOLYLINE` (connected line segments)
- [x] `IFCTRIMMEDCURVE` (trimmed arc/line)
- [x] `IFCCOMPOSITECURVE` (joined curve segments)
- [x] `IFCCIRCLE` (full circle, used in profiles and sweeps)
- [ ] `IFCLINE` (parametric line)
- [ ] `IFCBSPLINECURVE` / `IFCRATIONALBSPLINECURVEWITHKNOTS`

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
- [ ] Click-to-zoom: select spatial node → fit camera to its contents

### 2.2 Property Sets
- [x] Parse `IFCPROPERTYSET` (named property containers)
- [x] Parse `IFCRELDEFINESBYPROPERTIES` (property set → object links)
- [x] Parse `IFCPROPERTYSINGLEVALUE` (name + nominal value + unit)
- [x] Parse `IFCPROPERTYENUMERATEDVALUE`
- [x] Parse `IFCPROPERTYLISTVALUE`
- [x] Parse `IFCPROPERTYBOUNDEDVALUE` (upper/lower bounds)
- [ ] Parse `IFCPROPERTYTABLEVALUE`
- [x] Display property sets in Properties Panel (grouped by pset name, expandable sections)

### 2.3 Type Objects
- [x] Parse `IFCRELDEFINESBYTYPE` (element → type links)
- [x] Parse type entities (`IFCWALLTYPE`, `IFCSLABTYPE`, `IFCCOLUMNTYPE`, `IFCBEAMTYPE`, `IFCDOORTYPE`, `IFCWINDOWTYPE`, etc.)
- [x] Show type properties alongside instance properties
- [ ] Group elements by type in element tree

### 2.4 Quantity Takeoffs
- [x] Parse `IFCELEMENTQUANTITY` (area, volume, weight, length, count)
- [x] Parse `IFCQUANTITYAREA`, `IFCQUANTITYLENGTH`, `IFCQUANTITYVOLUME`, `IFCQUANTITYWEIGHT`, `IFCQUANTITYCOUNT`
- [ ] Unit conversion (SI ↔ Imperial) using `IFCUNITASSIGNMENT`
- [ ] Quantity summary table (per type, per storey)

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
- [ ] Enable MSAA (sample count 4) with device capability check
- [ ] Edge rendering (silhouette + crease edges via geometry shader or screen-space)
- [ ] Ambient occlusion (SSAO screen-space pass)
- [ ] Shadow mapping (directional light shadow cascade)
- [ ] Environment-based lighting (IBL / cubemap)
- [ ] Anti-aliasing post-process (FXAA fallback for non-MSAA devices)
- [ ] Gamma-correct rendering (linear workflow, sRGB output)

### 3.2 Visual Analysis Modes
- [x] X-ray / ghost mode (transparent with visible edges)
- [x] Color-by-type (type → color mapping applied to vertex buffer)
- [x] Color-by-storey (floor level → color gradient)
- [x] Color-by-material
- [ ] Color-by-property (user-selected numeric property → gradient)
- [ ] Grayscale mode (desaturate all except selected)

### 3.3 Section Tools
- [ ] Section box (6-plane bounding box clipping)
- [ ] Section fill (cap cut faces with solid color/hatch)
- [ ] Multiple section planes (combine X + Y + Z simultaneously)
- [ ] Section animation (sweep plane through model)

### 3.4 Level of Detail (LOD)
- [ ] Distance-based LOD (simplified geometry for far elements)
- [ ] Progressive loading (render low-detail first, refine)
- [ ] Occlusion culling (skip fully hidden elements)
- [ ] Screen-space size threshold (skip sub-pixel elements)

### 3.5 Annotations & Markup
- [ ] 3D text labels (element names, dimensions)
- [ ] Dimension lines (snapped to geometry edges/faces)
- [ ] Leader lines (annotation → element connection)
- [ ] 2D overlay annotations (pins, callouts, redlines)
- [ ] Markup persistence (save/load annotation sets)

### Milestone: Publication-quality rendering with analysis visualization.

---

## Phase 4: Navigation & UX

**Goal:** Fluid navigation and professional interaction patterns.

### 4.1 Camera Modes
- [ ] First-person walkthrough (WASD + mouse look)
- [ ] Turntable orbit (constrained to vertical axis)
- [ ] Orthographic projection toggle
- [ ] Named viewpoints (save/restore camera state)
- [ ] Smooth animated camera transitions (lerp position + target)
- [ ] Touch gesture refinement (two-finger pan, three-finger orbit)

### 4.2 Selection & Isolation
- [ ] Multi-select (long-press + tap, box select)
- [ ] Isolate selection (hide everything else)
- [ ] Hide selected / show all
- [ ] Selection sets (named groups for quick recall)
- [ ] Select by property filter (e.g., all elements where material = "Concrete")

### 4.3 Search & Filter
- [ ] Full-text search across all properties
- [ ] Filter by property value ranges
- [ ] Smart groups (saved filter criteria)
- [ ] Search results with highlight-in-viewport

### 4.4 Performance UX
- [ ] Loading progress bar (parse → build → upload stages)
- [ ] Async model loading (don't block UI during parse)
- [ ] Background model streaming (load visible storeys first)
- [ ] Memory usage indicator

### Milestone: Intuitive navigation comparable to desktop BIM viewers.

---

## Phase 5: Collaboration & Interoperability

**Goal:** BCF support, model comparison, and data exchange.

### 5.1 BCF (BIM Collaboration Format)
- [ ] BCF 2.1 XML import/export
- [ ] BCF 3.0 JSON support
- [ ] Create issues with viewpoint snapshots
- [ ] Navigate to saved viewpoints
- [ ] Issue status tracking (open/in-progress/closed)
- [ ] Comment threads on issues
- [ ] Component selection/visibility in viewpoints
- [ ] BCF API server integration (BIMcollab, Trimble Connect)

### 5.2 Model Comparison
- [ ] Diff two model versions (added/removed/modified elements)
- [ ] Visual diff overlay (green = added, red = removed, yellow = modified)
- [ ] Side-by-side and overlay comparison views
- [ ] Change report generation

### 5.3 Clash Detection
- [ ] AABB broad-phase collision detection
- [ ] Mesh-level narrow-phase intersection testing
- [ ] Clash grouping and categorization
- [ ] Tolerance settings (hard clash, soft clash, clearance)
- [ ] Clash report export (HTML/CSV)

### 5.4 Export & Sharing
- [ ] Screenshot with annotations
- [ ] PDF report generation (selected views + properties)
- [ ] glTF/GLB export (for AR/VR handoff)
- [ ] CSV/Excel property export
- [ ] Share viewpoint links (deep link to camera + selection state)

### Milestone: Team collaboration workflows with issue tracking and model diffing.

---

## Phase 6: File Format Support

**Goal:** Go beyond IFC to support the broader BIM ecosystem.

### 6.1 IFC Schema Coverage
- [ ] IFC2x3 full support (most common in practice)
- [ ] IFC4 ADD2 TC1 support (current standard)
- [ ] IFC4x3 support (infrastructure: bridges, roads, rails)
- [ ] Handle schema-specific entity differences gracefully

### 6.2 Additional Formats
- [ ] glTF/GLB import (pre-tessellated models)
- [ ] OBJ import (simple mesh interchange)
- [ ] Point cloud (LAS/LAZ) overlay
- [ ] PDF 2D drawing overlay (floor plans)
- [ ] DXF 2D import (site plans, details)
- [ ] Revit lightweight format (via IFC export guidance)

### 6.3 Large File Handling
- [ ] Streaming IFC parser (don't load entire file into memory)
- [ ] Geometry caching (serialize tessellated mesh to disk)
- [ ] Memory-mapped file access for 1GB+ files
- [ ] Background parsing with progress callbacks
- [ ] Partial model loading (filter by type/storey before full parse)

### Milestone: Handle real-world project files from any major BIM authoring tool.

---

## Phase 7: Platform & Deployment

**Goal:** Production-ready mobile deployment on Android and iOS.

### 7.1 Platform Support
- [x] Android (ARM64) — working
- [x] Android (x86_64 emulator) — working
- [ ] iOS (ARM64) — Rust cross-compilation + CocoaPods integration
- [ ] iOS Simulator (x86_64 / ARM64)
- [ ] Web (WebGPU/WebGL via wasm) — future

### 7.2 Build Pipeline
- [ ] Automated Rust cross-compilation for Android + iOS targets
- [ ] CI/CD pipeline (build + test + deploy)
- [ ] Pre-built native library caching
- [ ] Release size optimization (strip debug symbols, LTO)
- [ ] Automated integration tests on device/emulator

### 7.3 Accessibility
- [ ] Screen reader support for element tree and properties
- [ ] High contrast mode
- [ ] Configurable text scaling
- [ ] Dynamic type / system font size support

### Milestone: Ship on Android and iOS app stores with automated builds.

---

## Phase 8: Performance at Scale

**Goal:** Handle large real-world projects (100K+ elements, 1GB+ files) at interactive frame rates.

### 8.1 GPU Optimization
- [ ] Indirect draw (GPU-driven rendering, single draw call)
- [ ] Compute shader frustum culling (move CPU culling to GPU)
- [ ] Geometry compression (meshopt quantization + vertex cache optimization)
- [ ] Texture atlasing for material rendering
- [ ] Vertex buffer streaming (upload visible chunks only)

### 8.2 CPU Optimization
- [ ] Parallel IFC parsing (split DATA section across threads)
- [ ] Parallel tessellation (rayon work-stealing across elements)
- [ ] Incremental BVH updates (insert/remove without full rebuild)
- [ ] Spatial hash for broad-phase queries

### 8.3 Memory Optimization
- [ ] LOD-based geometry streaming (keep only visible detail level in GPU memory)
- [ ] Element geometry sharing (MappedItem deduplication)
- [ ] Compressed vertex format (16-bit positions, octahedral normals)
- [ ] Out-of-core rendering (disk-backed geometry for huge models)

### 8.4 Benchmarking
- [ ] Criterion benchmarks for parser, tessellator, renderer
- [ ] Frame time profiling (per-pass GPU timing queries)
- [ ] Memory usage tracking
- [ ] Regression testing against reference models

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
| Section planes/box | Yes | Yes | Yes | Yes | Plane only |
| BCF issues | No | Yes | Yes | Yes | No |
| Clash detection | Yes | Yes | No | Plugin | No |
| Multi-model federated | Yes | Yes | Yes | Yes | Yes |
| Measurement tools | Yes | Yes | Yes | Yes | Partial |
| Color-by rules | Yes | Yes | Yes | Yes | Yes |
| Quantity takeoffs | Yes | Yes | No | Plugin | No |
| Model comparison | Yes | Yes | No | No | No |
| First-person walkthrough | Yes | No | No | Yes | No |
| Point cloud support | Yes | No | No | Yes | No |
| 60fps at 100K elements | Yes | Yes | Yes | Yes | Untested |
