# Rendering Optimization Plan

Performance optimization roadmap for the Rust/wgpu BIM renderer. Goal: **best-in-class BIM viewer** competing with Autodesk Forge, xeokit, Trimble Connect, and BIMcollab ZOOM on models with 100K+ elements.

---

## Current State

| Metric | Before | Current | Target (Best-in-Class) |
|---|---|---|---|
| Draw calls per frame | N per element | **1 instanced** (frustum-culled) | 1 indirect draw per material batch |
| Vertex format | 40 bytes | **20 bytes** (packed normals+colors) | **8-12 bytes** (quantized pos + octahedral normal) |
| Culling | None | **Frustum + screen-space + GPU compute** | Frustum + HZB occlusion + compute shader |
| Picking | O(n) linear | **O(log n) BVH** | O(log n) BVH + ID buffer GPU pick |
| Mesh rebuild on toggle | Full rebuild | **Near-instant filter** | Zero-copy GPU flag flip |
| Selection highlight | Vertex color re-upload | Vertex color re-upload | **Post-process outline** (no re-upload) |
| Coordinate system | Z-up (IFC raw) | **Y-up transform on upload** | Y-up transform on upload |
| Camera orbit | Fixed speed | **Distance-scaled turntable** | Distance-scaled turntable |
| Render-when-idle | Every frame | **Dirty-flag skip** | Dirty-flag skip |
| Anti-aliasing | FXAA | **MSAA 4x + FXAA + SSAO** | MSAA 4x + FXAA fallback during interaction |
| Shadows | Shadow map | Shadow map | **Cascaded shadow maps (2-3 cascades)** |
| LOD | Screen-space cull only | Screen-space cull only | **Hierarchical LOD + mesh simplification** |
| Transparency | Alpha blend | Alpha blend | **Order-independent transparency (OIT)** |
| Back-face culling | Off for main geo | **On for solid geometry** | On for solid geometry |
| Interaction quality | Full quality always | **FastNav (skip SSAO/edges)** | FastNav + resolution reduction |
| GPU readback | Single-buffered sync | **Double-buffered async** | Double-buffered async |
| Loading | Full model at once | Full model at once | **Progressive/streaming** |
| Memory budget | Unbounded | ~2x file size peak | **Configurable cap** |

---

## Completed Phases

### Phase 0 — RAM Crisis (DONE)

| Task | What Changed | Impact |
|---|---|---|
| 0.1 Cache mesh on load | `RegisteredModel::new()` generates+caches mesh. 13+ API functions read from cache. | Eliminated ~90% transient allocations |
| 0.2 Reuse frame buffer | Persistent `pixel_buffer` in SceneRenderer, reused via `copy_from_slice()` | Saved 8.3 MB/frame allocation |
| 0.3 Zero-copy upload | `upload_mesh_from_arrays()` writes packed vertices directly into `mapped_at_creation` buffer | Eliminated full vertex data copy |
| 0.4 Drop parse data early | Scoped `content`/`ifc_file` blocks so they drop before storing model | ~2x peak load reduction |
| 0.5 Fix Flutter load sequence | Deferred renderer reload via `Future.microtask()`, yields before sync FFI | Loading spinner visible |
| 0.6 Remove empty HashMaps | `IfcProduct.properties` → `Option<HashMap>` defaulting to `None` | Minor cleanup |

### Phase 1 — Vertex Compression (DONE)

- `Vertex` struct: `[f32;3] position + [i8;4] normal_packed + [u8;4] color_packed` = **20 bytes**
- Pipeline: `Snorm8x4` normals, `Unorm8x4` colors (GPU auto-converts)
- **Impact:** 50% reduction in GPU memory and bandwidth

### Phase 2 — Frustum Culling (DONE)

- `Frustum` struct with Gribb-Hartmann extraction + p-vertex AABB test
- `ElementDrawRange` per element with AABB + index range
- Per-element culling in `render_frame()` — only visible elements drawn
- **Impact:** 60-80% triangle reduction at typical camera angles

### Phase 3 — BVH for Picking (DONE)

- `BvhNode` enum (Leaf/Internal), median-split along longest axis
- `ray_query()` with early rejection at each node
- Global `PICK_ACCELERATOR` rebuilt on load/reload
- **Impact:** Picking O(n) → O(log n), ~50K tests → ~16 tests

### Phase 4 — GPU Instancing (DONE)

- Shared 24-vertex unit box + `InstanceData` (28 bytes: position+scale+color)
- Single `draw_indexed` call for all visible instances
- **Impact:** 12x GPU memory reduction, single draw call per frame

### Phase 5 — Fast Visibility/Color Toggles (DONE)

- `instances_from_mesh_filtered()` reads cached mesh with visibility filter + highlight
- No mesh regeneration on toggle — just filter + color map
- **Impact:** Near-instant visibility and selection changes

### Phase 6 — Camera & Interaction Polish (DONE)

- IFC Z-up → Y-up coordinate transform on mesh upload
- Turntable orbit (Y-up constrained, no flip)
- Distance-scaled orbit/pan/zoom (consistent feel at any scale)
- Vertex centroid as orbit target (robust to outlier elements)
- Dynamic viewport resolution (matches device screen at configurable scale)
- Dirty-flag rendering (skip frames when nothing changed)
- Orbit/Pan mode toggle FAB
- Real-time FPS counter
- Toolbar overflow menu for mobile

---

## Phase 7 — Quick Wins (DONE)

### 7.1 Enable Back-Face Culling for Solid Geometry (DONE)
- **Technique:** `cull_mode: Some(wgpu::Face::Back)` on main + instanced pipelines
- **Caveat:** `cull_mode: None` kept for X-ray, wireframe, and section stencil passes
- **Impact:** ~40-50% fragment shader reduction

### 7.2 Adaptive Quality During Interaction / FastNav (DONE)
- **Technique:** `interaction_active` flag on SceneRenderer. During interaction: skip SSAO, skip edge rendering, increase min_screen_pixels threshold (2px → 8px). Restore full quality 150ms after last gesture.
- **Dart integration:** `setInteractionActive(true)` on gesture start, `false` after 150ms idle timer
- **Impact:** 2-4x faster interaction rendering
- **Reference:** [xeokit FastNavPlugin](https://xeokit.github.io/xeokit-sdk/docs/class/src/plugins/FastNavPlugin/FastNavPlugin.js~FastNavPlugin.html)

### 7.3 Double-Buffered Async GPU Readback (DONE)
- **Technique:** Two read buffers (A/B) alternate each frame. Current frame copies to one buffer while CPU reads from the other (previous frame's data). First frame falls back to synchronous readback.
- **Impact:** Overlaps GPU copy with CPU readback, ~30% throughput improvement

### 7.4 MSAA 4x (DONE)
- **Technique:** 4x multisampling on main color+depth render targets. MSAA resolve target feeds into FXAA post-process.
- **Impact:** Significantly sharper edges, especially on thin geometry

---

## Phase 8 — GPU-Driven Rendering

### 8.1 Compute Shader Frustum Culling (DONE)
- **Technique:** Compute shader dispatched each frame after geometry pass. Each invocation reads an element's AABB from storage buffer, tests against 6 frustum planes, writes visibility flag. Results read back from staging buffer using previous-frame approach (1 frame latency, no extra submit).
- **Auto-init:** AABBs uploaded automatically when `set_element_draw_ranges()` is called. Compute pipeline initialized on first model load.
- **Impact:** 10-20x faster culling for large models (100K+ elements)
- **Reference:** [vkguide.dev GPU-Driven Rendering](https://vkguide.dev/docs/gpudriven/compute_culling/)

### 8.2 Indirect Draw Calls
- **Status:** Not started
- **Technique:** Compute shader writes `DrawIndexedIndirect` arguments into a GPU buffer. A single `draw_indexed_indirect()` call per material batch renders everything. Zero CPU involvement in draw submission.
- **Why:** Eliminates CPU-GPU round-trips for draw call submission. Combined with 8.1, the entire cull→draw pipeline runs on GPU.
- **Critical:** All indirect draw args must be in a single buffer (Chrome/D3D12 has 300x overhead with separate buffers)
- **Impact:** Enables 100K+ objects at 60fps
- **Effort:** High
- **Reference:** [WebGPU Indirect Draws Best Practices](https://toji.dev/webgpu-best-practices/indirect-draws.html)

### 8.3 Hierarchical Z-Buffer (HZB) Occlusion Culling
- **Status:** Not started
- **Technique:** After depth prepass, build a mipmap chain of the depth buffer. Compute shader tests each element's AABB against the HZB — if the element's nearest depth is behind the HZB value at that screen location, it's occluded and skipped.
- **Why:** In a multi-storey building, most interior elements are occluded by walls/slabs. Frustum culling alone can't detect this. HZB occlusion culling skips 50-80% of frustum-visible geometry in interior views.
- **Two-pass variant:** First pass: render known-visible objects (from last frame). Build HZB. Second pass: cull remaining objects against HZB. This avoids false culling from stale depth data.
- **Impact:** 50-80% additional geometry reduction in interior views
- **Effort:** High
- **Reference:** [VTK WebGPU Occlusion Culling](https://www.kitware.com/webgpu-occlusion-culling-in-vtk/) — 5-6x speedup demonstrated

---

## Phase 9 — Selection & Highlighting Without Re-Upload

### 9.1 ID Buffer (Object ID Render Target)
- **Status:** Not started
- **Technique:** Render each element's unique ID as a color into a separate render target (R32Uint or RG16Uint). For picking: read the pixel under the cursor — instant O(1) GPU pick. For outline: post-process compares each pixel's ID with neighbors; where IDs differ, draw outline.
- **Why:** Current picking uses CPU ray-BVH intersection. ID buffer moves picking entirely to GPU with pixel-perfect accuracy. Also enables highlight outlines without modifying geometry.
- **Impact:** Instant picking + outline highlighting with zero geometry re-upload
- **Effort:** Medium
- **Reference:** [Omar Shehata — Surface ID Outlines](https://omar-shehata.medium.com/better-outline-rendering-using-surface-ids-with-webgl-e13cdab1fd94)

### 9.2 Per-Element Flag Texture (xeokit DTX approach)
- **Status:** Not started
- **Technique:** Store per-element flags (selected, highlighted, hidden, x-rayed, colorized) in a 1D texture or SSBO. Shader reads flags per-element and applies visual effects. Changing selection only updates a few texels — no geometry re-upload.
- **Why:** xeokit uses this for massive BIM models (100K+ elements) with instant selection/visibility toggling
- **Impact:** Zero-cost selection state changes
- **Effort:** Medium
- **Reference:** [xeokit Data Textures](https://xeokit.io/blog/compact-model-representation-using-data-textures/)

---

## Phase 10 — Level of Detail (LOD)

### 10.1 Screen-Space Error Metric
- **Status:** Partial (screen-space size culling exists at 2px threshold)
- **Technique:** Project each element's bounding sphere to screen pixels. If projected size < threshold, render a simplified version or skip entirely. Refine threshold: 2px = skip, 10px = low LOD, 50px+ = full detail.
- **Impact:** 50-90% triangle reduction for distant elements
- **Effort:** Medium

### 10.2 BIM-Aware LOD Generation
- **Status:** Not started
- **Technique:** Generate simplified geometry per element type:
  - Walls/slabs → single quad (oriented bounding rectangle)
  - Columns → simple box or cylinder
  - Complex elements → mesh simplification (quadric error decimation)
  - Fittings/fasteners → point sprites or skip
- **Why:** BIM elements have well-defined types; LOD can exploit semantic knowledge (a wall at distance is always a flat surface, never needs full mesh detail)
- **Impact:** Massive triangle reduction while maintaining recognizable building shape
- **Effort:** High

### 10.3 Hierarchical LOD (HLOD) — Cesium 3D Tiles Approach
- **Status:** Not started
- **Technique:** Organize elements into a spatial tree (octree/kd-tree). Each node has a `geometricError` value. At runtime, traverse the tree; if a node's screen-space error < threshold, render the node's simplified proxy mesh instead of descending to children.
- **Why:** Cesium 3D Tiles uses this to render entire cities. The `skipLevelOfDetail` optimization allows jumping multiple LOD levels for 30-50% faster refinement.
- **Impact:** Enables rendering of city-scale BIM models (millions of elements)
- **Effort:** Very High
- **Reference:** [Cesium 3D Tiles Selection Algorithm](https://cesium.com/learn/cesium-native/ref-doc/selection-algorithm-details.html)

---

## Phase 11 — Draw Call Optimization

### 11.1 Material-Based Batching
- **Status:** Not started
- **Technique:** Group elements by material (concrete, steel, glass, wood). Merge geometry sharing a material into a single vertex buffer. One draw call per material batch.
- **Why:** BIM models typically have <20 unique materials. Reducing from N per-element draw calls to ~20 material batches is a massive win. Autodesk Forge's "consolidation" does exactly this.
- **Impact:** Draw calls: N → ~20
- **Effort:** Medium
- **Reference:** [Forge Consolidated Geometry](https://aps.autodesk.com/blog/forge-viewer-consolidated-geometry)

### 11.2 Parametric Geometry Reuse (BIM Instancing)
- **Status:** Not started
- **Technique:** Detect identical IFC geometry definitions (IFCMAPPEDITEM, repeated IFCEXTRUDEDAREASOLID profiles). Share one GPU mesh across all instances with per-instance transforms. One draw call renders all windows of the same type.
- **Why:** BIM models contain massive repetition (every standard door, every identical window). Research shows 65% memory reduction from geometry reuse.
- **Impact:** Dramatic memory and draw call reduction for repetitive elements
- **Effort:** Medium
- **Reference:** IFC IFCREPRESENTATIONMAP / IFCMAPPEDITEM mechanism

### 11.3 Ubershader with Material SSBO
- **Status:** Not started
- **Technique:** Single shader for all materials. Material parameters (albedo, roughness, metallic) stored in a Shader Storage Buffer Object indexed by material ID. Eliminates pipeline state changes between materials.
- **Impact:** Zero pipeline switches during rendering
- **Effort:** Medium

---

## Phase 12 — Vertex & Memory Optimization

### 12.1 Vertex Quantization (20 → 8 bytes)
- **Status:** Not started
- **Technique:** Quantize positions to 16-bit integers relative to a tile origin (sufficient for mm precision within a 65m tile). Octahedral encoding of normals (2 bytes instead of 4). Vertex format: `[u16;3] pos_quantized + [u8;2] normal_oct` = 8 bytes.
- **Why:** meshoptimizer demonstrates 2-4x compression with fast GPU-side decode. xeokit uses data textures for similar compression.
- **Impact:** 60% further reduction in vertex buffer size
- **Effort:** Medium
- **Reference:** [meshoptimizer](https://meshoptimizer.org/)

### 12.2 Index Buffer Optimization
- **Status:** Not started
- **Technique:** Use meshoptimizer's `optimizeVertexCache()` for vertex cache optimization and `optimizeOverdraw()` to minimize pixel overdraw. Triangle strip generation for further index compression.
- **Impact:** 10-20% faster vertex processing from better cache utilization
- **Effort:** Low (meshoptimizer is a Rust crate)

### 12.3 Memory Budget System
- **Status:** Not started
- **Technique:** Configurable memory cap (e.g., 512MB). Track GPU buffer allocations. When budget is exceeded: evict lowest-priority data (distant LODs, hidden model meshes), compress vertex data more aggressively, or refuse to load additional models.
- **Why:** Autodesk Forge v2.15+ has memory budgeting to prevent browser crashes on large models. Essential for mobile.
- **Impact:** Prevents OOM crashes, enables controlled degradation
- **Effort:** Medium

---

## Phase 13 — Shadows & Lighting

### 13.1 Cascaded Shadow Maps (CSM)
- **Status:** Single shadow map exists
- **Technique:** Split the view frustum into 2-3 cascades. Each cascade has its own shadow map rendered from the light's perspective. Near cascades get higher resolution (sharp nearby shadows), far cascades get lower resolution (soft distant shadows).
- **Why:** Single shadow map causes perspective aliasing — near shadows are pixelated. CSM is the standard technique for architectural scenes.
- **Impact:** High-quality shadows at all distances
- **Effort:** Medium
- **Reference:** [Microsoft CSM](https://learn.microsoft.com/en-us/windows/win32/dxtecharts/cascaded-shadow-maps)

### 13.2 Contact-Hardening Soft Shadows (PCSS)
- **Status:** Not started
- **Technique:** Percentage-Closer Soft Shadows — shadow penumbra width varies with distance from occluder. Close contact shadows are sharp, distant shadows are soft.
- **Impact:** Physically plausible shadow appearance
- **Effort:** Medium

### 13.3 Image-Based Lighting (IBL)
- **Status:** Partially implemented (environment map support exists)
- **Technique:** Pre-filtered environment cubemap for specular reflections + irradiance map for diffuse. Gives materials a realistic appearance without expensive real-time global illumination.
- **Impact:** Professional-quality material appearance (glass, metal, polished surfaces)
- **Effort:** Medium

---

## Phase 14 — Streaming & Progressive Loading

### 14.1 Chunk-Based Spatial Streaming
- **Status:** Not started
- **Technique:** Divide the model spatially into chunks (e.g., octree cells or building storeys). Load chunks on-demand based on camera position. Prioritize chunks that appear largest on screen.
- **Why:** Autodesk Forge SVF2 does exactly this. Only geometry in the field of view is loaded, starting with the largest on-screen elements.
- **Impact:** Enables loading 1GB+ models on mobile devices
- **Effort:** Very High
- **Reference:** [Forge SVF2 Streaming](https://aps.autodesk.com/blog/update-svf2-ga-new-streaming-web-format-forge-viewer-now-production-ready)

### 14.2 Progressive IFC Parsing
- **Status:** Streaming parser exists in Rust
- **Technique:** Parse IFC file in chunks, generating and rendering geometry progressively. User sees the model build up in real-time rather than waiting for full parse.
- **Impact:** Perceived load time reduced to seconds even for large files
- **Effort:** Medium (parser exists, needs integration with progressive rendering)

### 14.3 Background Tessellation
- **Status:** Not started
- **Technique:** Tessellate complex geometry (B-rep, NURBS) on background threads while rendering already-tessellated elements. Use placeholder boxes for untessellated elements, replace with real geometry as it becomes available.
- **Impact:** Faster time-to-first-frame
- **Effort:** Medium

---

## Phase 15 — Transparency & Visual Quality

### 15.1 Order-Independent Transparency (OIT)
- **Status:** Not started
- **Technique:** Weighted Blended OIT — accumulate transparent fragment colors and weights in separate render targets, composite in a final pass. No sorting required.
- **Why:** BIM models have significant transparency (glass facades, X-ray mode for clash detection). Current alpha blend requires back-to-front sorting which is expensive and imperfect with merged geometry.
- **Impact:** Correct transparency rendering without sorting
- **Effort:** Medium

### 15.2 Edge Enhancement
- **Status:** Edge rendering exists (wireframe overlay)
- **Technique:** Screen-space edge detection using depth+normal discontinuities as a post-process. More efficient than drawing wireframe geometry and works on all geometry regardless of mesh structure.
- **Why:** xeokit uses edge enhancement to clarify BIM element boundaries — critical for architectural visualization
- **Impact:** Clearer element boundaries, professional appearance
- **Effort:** Medium

### 15.3 Temporal Anti-Aliasing (TAA)
- **Status:** Not started
- **Technique:** Jitter the projection matrix slightly each frame, accumulate multiple frames. Produces sub-pixel anti-aliasing. Requires motion vectors for reprojection to avoid ghosting.
- **Why:** TAA produces the highest quality anti-aliasing for still/slow camera movement. Good for architectural walkthroughs.
- **Caveat:** Can ghost on fast camera movement. Best combined with Phase 7.2 (fall back to FXAA during interaction).
- **Impact:** Best-in-class anti-aliasing quality on idle
- **Effort:** High

---

## Priority Roadmap

| Priority | Phase | Technique | Impact | Effort | Status |
|----------|-------|-----------|--------|--------|--------|
| ~~1~~ | ~~7.1~~ | ~~Back-face culling~~ | ~~~40% fragment reduction~~ | ~~Trivial~~ | **DONE** |
| ~~2~~ | ~~7.2~~ | ~~Adaptive quality (FastNav)~~ | ~~2-4x interaction FPS~~ | ~~Low~~ | **DONE** |
| ~~3~~ | ~~7.3~~ | ~~Double-buffered readback~~ | ~~~30% throughput~~ | ~~Low~~ | **DONE** |
| ~~4~~ | ~~7.4~~ | ~~MSAA 4x~~ | ~~Sharp edges~~ | ~~Low~~ | **DONE** |
| ~~5~~ | ~~8.1~~ | ~~Compute shader culling~~ | ~~10-20x cull speed~~ | ~~Medium~~ | **DONE** |
| **6** | **9.1** | **ID buffer (GPU pick + outline)** | **Instant pick, no re-upload** | **Medium** | Next |
| **7** | **8.2** | **Indirect draws** | **100K+ elements @ 60fps** | **High** | |
| **8** | **11.1** | **Material batching** | **N → ~20 draw calls** | **Medium** | |
| **9** | **11.2** | **Parametric geometry reuse** | **65% memory reduction** | **Medium** | |
| **10** | **9.2** | **Per-element flag texture** | **Zero-cost selection** | **Medium** | |
| **11** | **8.3** | **HZB occlusion culling** | **50-80% interior reduction** | **High** | |
| **12** | **12.1** | **Vertex quantization (8 bytes)** | **60% vertex memory** | **Medium** | |
| **13** | **10.2** | **BIM-aware LOD** | **50-90% distant triangles** | **High** | |
| **14** | **13.1** | **Cascaded shadow maps** | **Quality shadows** | **Medium** | |
| **15** | **15.1** | **Order-independent transparency** | **Correct glass/X-ray** | **Medium** | |
| **16** | **14.1** | **Chunk-based streaming** | **1GB+ models on mobile** | **Very High** | |
| **17** | **12.3** | **Memory budget** | **Prevents OOM** | **Medium** | |
| **18** | **10.3** | **Hierarchical LOD** | **City-scale models** | **Very High** | |

---

## Completed Progress

| Phase | Task | Status | Impact |
|---|---|---|---|
| **0.1** | Cache mesh on load | **DONE** | Eliminates rebuild allocs |
| **0.2** | Reuse frame buffer | **DONE** | 8.3 MB/frame saved |
| **0.3** | Zero-copy upload | **DONE** | No intermediate copy |
| **0.4** | Drop parse data early | **DONE** | 2x peak reduction |
| **0.5** | Fix Flutter load sequence | **DONE** | Loading spinner works |
| **0.6** | Remove empty HashMaps | **DONE** | Minor cleanup |
| **1** | Vertex compression (40→20 bytes) | **DONE** | 50% GPU memory |
| **2** | Frustum culling | **DONE** | 60-80% triangle reduction |
| **3** | BVH for picking | **DONE** | O(n) → O(log n) |
| **4** | GPU instancing | **DONE** | 12x GPU memory, 1 draw call |
| **5** | Fast visibility/color toggles | **DONE** | Near-instant toggle |
| **6** | Camera & interaction polish | **DONE** | Y-up, turntable, distance-scaled |
| **7.1** | Back-face culling | **DONE** | ~40% fragment reduction |
| **7.2** | FastNav (adaptive quality) | **DONE** | 2-4x interaction FPS |
| **7.3** | Double-buffered readback | **DONE** | ~30% throughput |
| **7.4** | MSAA 4x | **DONE** | Sharp edges on geometry |
| **8.1** | Compute shader frustum culling | **DONE** | GPU culling for 100K+ elements |

---

## Key Files Reference

| File | Purpose |
|---|---|
| `rust/src/renderer/scene.rs` | Frame rendering, instanced draw, GPU readback, SSAO, FXAA |
| `rust/src/renderer/pipeline.rs` | WGSL shaders, render pipeline config, SSAO pipeline |
| `rust/src/renderer/camera.rs` | Camera, turntable orbit, frustum extraction |
| `rust/src/renderer/vertex.rs` | Vertex, BoxVertex, InstanceData structs, unit box |
| `rust/src/renderer/bvh.rs` | BVH for O(log n) ray picking + spatial hash |
| `rust/src/bim/model.rs` | Mesh generation, element tracking |
| `rust/src/bim/geometry.rs` | Mesh struct, box generation, merge |
| `rust/src/bim/model_registry.rs` | Multi-model management, mesh caching |
| `rust/src/api/rendering.rs` | FFI surface, Z-up→Y-up transform, mesh upload |
| `rust/src/api/camera.rs` | Camera API (orbit, pan, zoom, fit) |
| `rust/src/api/selection.rs` | Picking, selection state, color modes |

---

## Industry References

| Viewer | Key Technique | Source |
|---|---|---|
| **Autodesk Forge** | Geometry consolidation, SVF2 streaming, memory budgeting | [Forge Blog](https://aps.autodesk.com/blog) |
| **xeokit** | Data textures (DTX), FastNavPlugin, SAO, kd-tree culling | [xeokit Performance Tips](https://xeokit.io/blog/viewer-performance-tips/) |
| **Cesium 3D Tiles** | Hierarchical LOD, screen-space error, tile streaming | [Cesium Selection Algorithm](https://cesium.com/learn/cesium-native/ref-doc/selection-algorithm-details.html) |
| **Trimble Connect** | Direct parametric rendering, multi-threaded loading | [Trimble Docs](https://help.trimble.com) |
| **Unreal Nanite** | Cluster-based LOD, mesh shaders, GPU-driven | [Epic Nanite Docs](https://dev.epicgames.com/documentation/en-us/unreal-engine/nanite-virtualized-geometry-in-unreal-engine) |
| **VTK WebGPU** | Compute-based occlusion culling (5-6x speedup) | [Kitware Blog](https://www.kitware.com/webgpu-occlusion-culling-in-vtk/) |
| **meshoptimizer** | Vertex quantization, cache optimization, octahedral normals | [meshoptimizer.org](https://meshoptimizer.org/) |
| **vkguide.dev** | GPU-driven indirect draws, compute culling | [GPU-Driven Engines](https://vkguide.dev/docs/gpudriven/gpu_driven_engines/) |
