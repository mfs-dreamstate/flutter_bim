# Rendering Optimization Plan

Performance optimization roadmap for the Rust/wgpu BIM renderer to handle large-scale models (50K+ elements).

---

## Current State

| Metric | Current Value |
|---|---|
| Draw calls per frame | 1 (all geometry merged) |
| Vertex size | 40 bytes (3xf32 pos + 3xf32 normal + 4xf32 color) |
| Culling | Back-face only (GPU-side) |
| Frustum culling | None |
| Occlusion culling | None |
| LOD | None |
| Instancing | None |
| Picking | O(n) linear AABB scan |
| Readback | Synchronous (CPU blocks every frame) |
| Mesh rebuild | Full rebuild on any visibility/selection change |
| Spatial structure | None |

---

## Phase 0 — RAM Crisis (Fix First)
> These are the root cause of the crash during loading. Must fix before anything else.

### Root Cause: Loading Crash Sequence

When the user loads a real BIM file (e.g. STRUC_NordicLCA), this exact sequence runs:

```
Flutter                                    Rust                              Peak RAM
───────                                    ────                              ────────
loadIfcFile(path)                    →     read_to_string()                  1x file
                                           .replace("\r\n", "\n")            2x file
                                           IfcFile::parse() - many0          2x file + entities
                                           BimModel::from_ifc_file()         2x file + entities + model
                                           model.get_info()                  (trivial)
                                           registry.add_model()              model stored
                                    ←      content + ifc_file dropped        model only

onModelsChanged() fires immediately
  → loadAllModelsIntoRenderer()      →     generate_meshes()                 model + mesh (~22 MB/10K)
    (SYNC FFI - blocks Dart)               upload_mesh_from_arrays()         model + mesh + Vertex copy
                                           create GPU buffers                model + GPU buffers
                                    ←      mesh + Vertex copy dropped

ElementTree.initState() fires
  → getAllElementsFromAllModels()     →     generate_meshes() AGAIN          model + ANOTHER mesh copy
    (SYNC FFI - blocks Dart)         ←      elements returned, mesh dropped
```

**Why it crashes:** For a 30 MB IFC file with 20K elements:
- Step 1-2: ~120 MB peak (4x file during parse)
- Step 3: +44 MB (mesh generation)
- Step 4: +44 MB (mesh generated AGAIN for element list)
- **Total peak: ~200+ MB** before the user sees anything

On mobile or memory-constrained environments, this kills the app.

### 0.1 Cache Mesh Data on Model Load
- [ ] **Status: Not Started**
- **Files:** `rust/src/bim/model.rs`, `rust/src/api.rs`
- **Problem:** `generate_meshes()` is called on **13+ different API endpoints**, each time allocating a full new mesh (4 large `Vec`s + element info), using it briefly, then dropping it. For 10K elements, each call allocates ~10 MB of temporary data.
- **Affected calls:**
  | Function | File:Line | Why it rebuilds |
  |---|---|---|
  | `pick_element()` | api.rs:520 | Rebuilds all models per click |
  | `get_all_elements()` | api.rs:544 | Rebuild to list elements |
  | `get_all_elements_from_all_models()` | api.rs:560 | Rebuild all models |
  | `get_element_counts()` | api.rs:572 | Rebuild to count types |
  | `fit_camera_to_model()` | api.rs:450 | Rebuild for bounds |
  | `fit_camera_to_all_models()` | api.rs:475 | Rebuild all for bounds |
  | `get_geo_reference()` | api.rs:691 | Rebuild for dimensions |
  | `get_element_info()` | model.rs:1053 | Rebuild to find 1 element |
  | `get_bounds()` | model.rs:688 | Rebuild for bounding box |
  | `load_model_into_renderer()` | api.rs:349 | Expected (but data discarded) |
  | `load_all_models_into_renderer()` | api.rs:393 | Expected (but data discarded) |
  | `reload_model_mesh()` | api.rs:748 | Expected (for visibility) |
  | `reload_all_models_mesh()` | api.rs:783 | Expected (for visibility) |
- **Solution:** Generate mesh data **once** on model load. Store `ModelMesh` (vertices, indices, normals, colors, elements, bounds) inside the `RegisteredModel`. All API functions read from cache instead of regenerating.
- **Expected impact:** Eliminates ~90% of transient memory allocations. Picks, queries, camera fits become zero-alloc.
- **Implementation:**
  ```
  1. Add cached_mesh: Option<ModelMesh> to RegisteredModel
  2. Populate on add_model() / add_model_with_id()
  3. Add invalidate_cache() for visibility/selection changes
  4. Replace all generate_meshes() calls with cache reads
  5. Only regenerate on: model load, visibility change, selection change
  ```

### 0.2 Reuse Frame Buffer Across FFI
- [ ] **Status: Not Started**
- **Files:** `rust/src/renderer/scene.rs`, `rust/src/api.rs`
- **Problem:** Every `render_frame()` call creates a new `Vec<u8>` with capacity `width * height * 4`. At 1920x1080 = **8.3 MB allocated per frame**. This crosses FFI to Flutter as a new Dart `Uint8List` each frame, causing GC pressure on both sides.
- **Solution:** Allocate a persistent `Vec<u8>` in the renderer. Rewrite into it each frame. Return a reference/slice rather than a new allocation. On the Flutter side, reuse the same `Uint8List` or use a shared memory approach.
- **Expected impact:** Eliminates 8.3 MB/frame of allocation churn (at 30fps = 250 MB/s of garbage)
- **Implementation:**
  ```
  1. Add persistent_pixels: Vec<u8> to SceneRenderer
  2. Allocate once in initialize() with correct capacity
  3. In render_frame(), write directly into persistent_pixels
  4. Return &[u8] slice instead of Vec<u8>
  5. On Flutter side: explore flutter_rust_bridge zero-copy if possible
  ```

### 0.3 Eliminate Redundant Data Copies
- [ ] **Status: Not Started**
- **Files:** `rust/src/renderer/scene.rs`, `rust/src/bim/geometry.rs`
- **Problem:** Mesh data exists in **5 copies simultaneously:**
  1. `BimModel` entity structs (walls, slabs, etc.)
  2. `ModelMesh` flat arrays (from `generate_meshes()`)
  3. `Vec<Vertex>` interleaved (from `upload_mesh_from_arrays()`)
  4. GPU vertex/index buffers
  5. Readback buffer (for pixels)
- Copy #3 is completely unnecessary — the flat arrays are converted to interleaved `Vertex` structs just to upload.
- **Solution:** Upload directly from flat arrays using `queue.write_buffer()` with interleaved layout, or change `ModelMesh` to store interleaved data from the start.
- **Expected impact:** Eliminates one full copy of all vertex data (~46 MB for 50K elements)

### 0.4 Drop IFC Parse Data After Model Build
- [ ] **Status: Not Started**
- **Files:** `rust/src/api.rs`
- **Problem:** In `load_ifc_file()` (api.rs:86-99), the file content string and parsed `IfcFile` stay alive until the function returns. For large IFC files (50-200 MB), this means both the raw string and parsed entity tree are in memory alongside the `BimModel` being built.
- **Solution:** Scope the parsing tighter. Drop `content` and `ifc_file` before storing the model in the registry.
- **Expected impact:** Peak memory during load reduced by ~2x file size
- **Implementation:**
  ```rust
  let model = {
      let content = tokio::fs::read_to_string(&file_path).await?;
      let ifc_file = IfcFile::parse(&content)?;
      BimModel::from_ifc_file(&ifc_file)?
      // content and ifc_file dropped here
  };
  registry.add_model(model, name, Some(file_path));
  ```

### 0.5 Fix Flutter Loading Sequence (Dart side)
- [ ] **Status: Not Started**
- **Files:** `lib/src/core/providers/model_state.dart`, `lib/src/widgets/element_tree.dart`
- **Problem:** After `loadIfcFile()` returns, `onModelsChanged()` fires immediately and calls `loadAllModelsIntoRenderer()` as a **synchronous FFI call** that blocks Dart. Simultaneously, `ElementTree.initState()` calls `getAllElementsFromAllModels()` which regenerates meshes a second time. Two massive allocations overlap.
- **Solution:**
  1. Make the post-load sequence **sequential and async** — don't let element tree trigger while renderer is loading
  2. Don't call `getAllElementsFromAllModels()` in `initState()` — read from cached data instead (after 0.1 is done)
  3. Add a loading gate: block UI interaction until initial load + render is complete
- **Expected impact:** Prevents overlapping allocations during load. Combined with 0.1 (caching), eliminates the crash entirely.

### 0.6 Remove Empty HashMap Allocations
- [ ] **Status: Not Started**
- **Files:** `rust/src/bim/model.rs`, `rust/src/bim/entities.rs`
- **Problem:** Every `IfcProduct` has `properties: HashMap::new()` which is never populated. `HashMap::new()` doesn't allocate until first insert in Rust, so this is minor — but if the struct is ever serialized/cloned it adds overhead.
- **Solution:** Use `Option<HashMap<String, String>>` defaulting to `None`, or remove the field if unused.
- **Expected impact:** Minor — mainly prevents future problems if properties get populated

---

## RAM Budget Analysis

For a model with N elements (each a box = 24 vertices, 12 triangles):

| Data | Size per element | 10K elements | 50K elements |
|---|---|---|---|
| Vertex buffer (40 bytes/vert) | 960 B | 9.2 MB | 46 MB |
| Index buffer (4 bytes/idx) | 144 B | 1.4 MB | 6.9 MB |
| Normals (flat f32 array) | 288 B | 2.7 MB | 13.8 MB |
| Colors (flat f32 array) | 384 B | 3.7 MB | 18.4 MB |
| ElementInfo (per element) | ~200 B | 1.9 MB | 9.5 MB |
| BimModel entity structs | ~300 B | 2.9 MB | 14.3 MB |
| **Subtotal (one copy)** | **~2.3 KB** | **~22 MB** | **~109 MB** |
| Readback texture (1080p) | — | 8.3 MB | 8.3 MB |
| Pixel Vec per frame (1080p) | — | 8.3 MB | 8.3 MB |

**Current worst case** (uncached, mid-interaction): 3-4 copies of mesh data alive simultaneously = **330-440 MB for 50K elements**, plus 8.3 MB/frame allocation churn.

**After Phase 0 fixes**: 1 cached mesh + 1 GPU copy + 1 reused pixel buffer = **~120 MB for 50K elements** with zero per-frame allocation.

---

## Phase 1 — Immediate Wins
> Low effort, high impact. No architectural changes.

### 1.1 Double-Buffered Async Readback
- [ ] **Status: Not Started**
- **File:** `rust/src/renderer/scene.rs`
- **Problem:** `device.poll(Maintain::Wait)` blocks the CPU every frame waiting for GPU to finish. For 1920x1080, that's 8.3 MB copied synchronously per frame.
- **Solution:** Use two read buffers in ping-pong fashion. While GPU writes to buffer A, CPU reads from buffer B (previous frame). One frame of display latency, but CPU never stalls.
- **Expected impact:** ~2x framerate improvement
- **Implementation:**
  ```
  - Add second read buffer to SceneRenderer
  - Track current/previous buffer index
  - Submit render + copy to current buffer
  - Map and read previous buffer (non-blocking)
  - Swap buffers each frame
  ```

### 1.2 Cache ElementInfo
- [ ] **Status: Not Started**
- **Files:** `rust/src/bim/model.rs`, `rust/src/api.rs`
- **Problem:** `get_element_info()` calls `generate_meshes()` which rebuilds the entire model mesh just to look up one element. Same with `get_bounds()`.
- **Solution:** Cache the `Vec<ElementInfo>` and `BoundingBox` on model load. Return from cache instead of regenerating.
- **Expected impact:** Element lookups from O(n) mesh rebuild to O(1) cache hit

### 1.3 Compress Vertex Format
- [ ] **Status: Not Started**
- **Files:** `rust/src/renderer/vertex.rs`, `rust/src/renderer/pipeline.rs` (shader)
- **Problem:** Each vertex is 40 bytes. Color stored as 4x f32 (16 bytes) when BIM models use <20 distinct colors. Normals are axis-aligned (box faces) so 3x f32 (12 bytes) is overkill.
- **Solution:**
  - Color: `[f32; 4]` → `[u8; 4]` (RGBA 0-255) = 4 bytes
  - Normal: `[f32; 3]` → `[i8; 4]` (snorm, padded) = 4 bytes
  - New vertex: 12 + 4 + 4 = **20 bytes** (50% reduction)
- **Expected impact:** 50% reduction in vertex buffer size and GPU bandwidth
- **Implementation:**
  ```
  - Update Vertex struct and desc()
  - Update vertex shader input types
  - Update upload_mesh_from_arrays() conversion
  - Update generate_box_with_normals() output
  ```

---

## Phase 2 — Frustum Culling
> Medium effort, critical for large models. Eliminates 60-80% of triangles.

### 2.1 Frustum Plane Extraction
- [ ] **Status: Not Started**
- **File:** `rust/src/renderer/camera.rs`
- **Problem:** No frustum representation exists. All geometry is always submitted.
- **Solution:** Extract 6 frustum planes from the view-projection matrix each frame.
- **Implementation:**
  ```
  - Add extract_frustum_planes(view_proj: Mat4) -> [Vec4; 6]
  - Call on camera update
  - Planes: left, right, top, bottom, near, far
  ```

### 2.2 Per-Element AABB Frustum Test
- [ ] **Status: Not Started**
- **Files:** `rust/src/bim/geometry.rs`, `rust/src/renderer/scene.rs`
- **Problem:** Every element's triangles are sent to the GPU regardless of camera position.
- **Solution:** Before building the merged mesh, test each element's AABB against the frustum. Skip elements fully outside.
- **Expected impact:** 60-80% triangle reduction at typical camera angles
- **Implementation:**
  ```
  - Add is_aabb_in_frustum(planes, min, max) -> bool
  - In generate_meshes_filtered(), skip elements outside frustum
  - Or better: maintain element list, rebuild only visible subset
  ```

### 2.3 Storey-Based Culling
- [ ] **Status: Not Started**
- **Files:** `rust/src/bim/model.rs`
- **Problem:** BIM models naturally group by storey but this isn't used for culling.
- **Solution:** Group elements by storey. Compute storey bounding boxes. Cull entire storeys before testing individual elements.
- **Expected impact:** Reduces per-element frustum tests by storey count factor (typically 3-10x fewer tests)

---

## Phase 3 — Spatial Acceleration
> Medium effort, required for 50K+ element picking and culling.

### 3.1 BVH for Picking
- [ ] **Status: Not Started**
- **Files:** `rust/src/bim/model.rs`, `rust/src/renderer/camera.rs`, `rust/src/api.rs`
- **Problem:** `pick_element()` in api.rs iterates all elements with `ray_aabb_intersect()`. O(n) per click.
- **Solution:** Build a BVH (Bounding Volume Hierarchy) from element AABBs on model load. Traverse O(log n) nodes per pick.
- **Expected impact:** Picking from O(n) to O(log n). For 50K elements: ~50,000 tests → ~16 tests.
- **Implementation:**
  ```
  - Add bvh module with BvhNode { aabb, left, right, element_indices }
  - Build BVH using SAH (Surface Area Heuristic) on model load
  - Replace linear scan in pick_element() with BVH traversal
  - Reuse for frustum culling (traverse BVH, collect visible elements)
  ```

### 3.2 Octree for Frustum Culling
- [ ] **Status: Not Started**
- **File:** New `rust/src/bim/octree.rs`
- **Problem:** Per-element frustum testing is still O(n) even with the AABB test being fast.
- **Solution:** Build octree from element centers. Frustum cull entire octree nodes. Only test leaf elements.
- **Note:** BVH from 3.1 can serve double duty here. Evaluate whether separate octree is needed or if BVH frustum traversal is sufficient.

---

## Phase 4 — GPU Instancing
> Medium effort, massive win for BIM (80%+ repeated geometry).

### 4.1 Geometry Deduplication
- [ ] **Status: Not Started**
- **Files:** `rust/src/bim/geometry.rs`, `rust/src/bim/model.rs`
- **Problem:** Every element has its own copy of box geometry. 500 identical columns = 500 copies of the same 24-vertex box.
- **Solution:** Hash element geometry by (size, type). Store unique meshes. Map elements to their prototype mesh + per-instance transform.
- **Expected impact:** Vertex buffer size reduction 10-50x for typical BIM models

### 4.2 Instance Buffer & Draw
- [ ] **Status: Not Started**
- **Files:** `rust/src/renderer/scene.rs`, `rust/src/renderer/pipeline.rs`, `rust/src/renderer/vertex.rs`
- **Problem:** Single monolithic draw call with all geometry baked in. No instancing support.
- **Solution:**
  - Create per-instance data struct: `{ model_matrix: mat4, color: u32 }`
  - Group elements by prototype mesh
  - One `draw_indexed_indirect()` per unique mesh with instance count
  - Shader reads instance data from storage buffer
- **Expected impact:** 5-20x fewer vertices processed by GPU
- **Implementation:**
  ```
  - Add InstanceData struct (transform + color)
  - Create instance buffer alongside vertex/index buffers
  - Add instance attributes to vertex shader
  - Replace single draw_indexed with per-prototype instanced draws
  - Update upload pipeline to separate geometry from instances
  ```

---

## Phase 5 — LOD System
> High effort, important for very large models at varying zoom levels.

### 5.1 Distance-Based LOD Selection
- [ ] **Status: Not Started**
- **Files:** `rust/src/renderer/scene.rs`, `rust/src/bim/model.rs`
- **Problem:** All elements render at full detail regardless of distance from camera.
- **Solution:** Define LOD levels based on element screen-space size:
  - **LOD 0:** Full geometry (element covers >50px on screen)
  - **LOD 1:** Simplified box (element covers 10-50px)
  - **LOD 2:** Skip / point sprite (element covers <10px)
- **Note:** Since current geometry IS boxes, LOD 0 and 1 are identical right now. This becomes important when real IFC geometry parsing is added.

### 5.2 Screen-Space Error Metric
- [ ] **Status: Not Started**
- **Problem:** Need a fast way to estimate element size on screen.
- **Solution:** Project element bounding sphere to screen. Use projected radius to select LOD.
  ```
  screen_size = (sphere_radius / distance_to_camera) * screen_height / (2 * tan(fov/2))
  ```

---

## Phase 6 — Incremental Updates
> High effort, eliminates full mesh rebuilds on interaction.

### 6.1 Indirect Draw with Visibility Buffer
- [ ] **Status: Not Started**
- **Files:** `rust/src/renderer/scene.rs`, `rust/src/renderer/pipeline.rs`
- **Problem:** Any visibility toggle or selection change triggers full mesh regeneration + GPU re-upload.
- **Solution:**
  - Store each element as a separate draw range in the index buffer
  - Use `multi_draw_indexed_indirect` with a buffer of draw commands
  - Toggle visibility by zeroing out an element's draw command (no mesh rebuild)
- **Expected impact:** Visibility/selection changes from ~100ms to <1ms

### 6.2 Per-Element Color Override via SSBO
- [ ] **Status: Not Started**
- **Files:** `rust/src/renderer/pipeline.rs` (shader), `rust/src/renderer/scene.rs`
- **Problem:** Selection highlighting currently rebuilds entire mesh with new vertex colors.
- **Solution:** Add a storage buffer with per-element state (selected, hidden, color override). Shader reads element ID and applies override. Only update the small state buffer on interaction.
- **Implementation:**
  ```
  - Add element_id as vertex attribute (or derive from gl_VertexIndex)
  - Add storage buffer with per-element flags
  - Shader: if element_state[id].selected, use highlight color
  - On selection change, update only the 4-byte flag in the buffer
  ```

---

## Phase 7 — Advanced (Future)
> For when real IFC geometry parsing is added.

### 7.1 Compute Shader Culling
- [ ] Run frustum + occlusion culling entirely on GPU via compute shader
- [ ] Output visible element list to indirect draw buffer
- [ ] Zero CPU involvement in per-frame culling

### 7.2 Mesh Simplification
- [ ] Implement quadric error mesh decimation for real geometry LODs
- [ ] Pre-compute simplified meshes on model load

### 7.3 Texture Atlas
- [ ] Pack material textures into atlas for single bind group
- [ ] Reduces texture binding overhead for textured models

### 7.4 Occlusion Culling (Hi-Z)
- [ ] Generate depth hierarchy from previous frame
- [ ] Test element AABBs against Hi-Z buffer in compute shader
- [ ] Skip fully occluded elements

---

## Progress Tracker

| Phase | Task | Status | Impact | Effort |
|---|---|---|---|---|
| **0.1** | **Cache mesh data on load** | **Not Started** | **Fixes RAM — eliminates rebuild allocs** | **Low** |
| **0.2** | **Reuse frame buffer across FFI** | **Not Started** | **Fixes RAM — 8.3 MB/frame saved** | **Low** |
| **0.3** | **Eliminate redundant data copies** | **Not Started** | **Fixes RAM — 1 fewer full copy** | **Low** |
| **0.4** | **Drop IFC parse data early** | **Not Started** | **Fixes RAM — 2x peak load reduction** | **Low** |
| **0.5** | **Fix Flutter loading sequence** | **Not Started** | **Prevents overlapping allocs** | **Low** |
| **0.6** | **Remove empty HashMap allocs** | **Not Started** | **Minor RAM cleanup** | **Trivial** |
| 1.1 | Async readback | Not Started | ~2x FPS | Low |
| 1.2 | Cache ElementInfo | Not Started | Merged into 0.1 | — |
| 1.3 | Vertex compression | Not Started | 50% bandwidth | Low |
| 2.1 | Frustum plane extraction | Not Started | Enables culling | Low |
| 2.2 | Per-element frustum cull | Not Started | 60-80% tri reduction | Medium |
| 2.3 | Storey-based culling | Not Started | Fewer cull tests | Medium |
| 3.1 | BVH for picking | Not Started | Picking 100x faster | Medium |
| 3.2 | Octree for culling | Not Started | Culling O(n) → O(log n) | Medium |
| 4.1 | Geometry deduplication | Not Started | 10-50x vertex reduction | Medium |
| 4.2 | Instance buffer & draw | Not Started | 5-20x GPU perf | Medium |
| 5.1 | Distance-based LOD | Not Started | 50-90% tri reduction | High |
| 5.2 | Screen-space error metric | Not Started | Enables LOD | Medium |
| 6.1 | Indirect draw + visibility | Not Started | <1ms toggle | High |
| 6.2 | Per-element color SSBO | Not Started | <1ms selection | High |
| 7.1 | Compute shader culling | Not Started | Full GPU culling | High |
| 7.2 | Mesh simplification | Not Started | Real geometry LOD | High |
| 7.3 | Texture atlas | Not Started | Texture perf | Medium |
| 7.4 | Hi-Z occlusion culling | Not Started | Skip hidden geometry | High |

---

## Scaling Targets

| Element Count | Current Performance | Target Performance |
|---|---|---|
| 1K | Smooth | Smooth |
| 10K | Sluggish | Smooth |
| 50K | Unusable | 30+ FPS |
| 100K | Unusable | 20+ FPS |
| 500K | Unusable | 15+ FPS (with LOD) |

---

## Key Files Reference

| File | Purpose |
|---|---|
| `rust/src/renderer/scene.rs` | Frame rendering, GPU readback, buffer management |
| `rust/src/renderer/pipeline.rs` | WGSL shaders, render pipeline config |
| `rust/src/renderer/camera.rs` | Camera, orbit, ray casting |
| `rust/src/renderer/vertex.rs` | Vertex struct, buffer layout |
| `rust/src/bim/model.rs` | Mesh generation, element tracking |
| `rust/src/bim/geometry.rs` | Mesh struct, box generation, merge |
| `rust/src/bim/model_registry.rs` | Multi-model management |
| `rust/src/api.rs` | FFI surface, picking, visibility |
