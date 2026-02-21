# Future Features - Dalux Comparison Gap

Features we are currently missing compared to Dalux BIM Viewer and other industry leaders.
These should be implemented in future iterations.

---

## Measurement Tools

### Laser Measurement
- Point-and-shoot distance measurement from a surface to the nearest opposing geometry
- User taps a surface point, the system ray-casts in the surface normal direction
- Displays distance to the first intersection
- Use case: quickly measure wall thickness, room depth, clearance

### 3-Axis Laser
- From a picked 3D point, project three orthogonal rays (X, Y, Z)
- Display distance from the point to the nearest geometry along each axis
- Visual: three colored lines (red=X, green=Y, blue=Z) with distance labels
- Use case: verify element positioning relative to grid or reference planes

### 2D Length and Area (Enhanced)
- Current: basic distance, area, and volume measurement via point-picking
- Needed: dedicated 2D measurement mode when in 2D/floor plan view
- Snapping to element edges and vertices for precision
- Persistent dimensions that remain visible after creation

---

## Views & Navigation

### 2D/3D Side-by-Side
- Split-screen view: 2D floor plan on one side, 3D model on the other
- Synchronized selection: tapping an element in one view highlights it in both
- Synchronized section: floor plan shows the current storey's section cut
- Implementation: second viewport rendering a top-down orthographic view

---

## Collaboration & Cloud

### BIM Authoring Tool Plugins
- Direct export plugins for:
  - Autodesk Revit
  - Autodesk Navisworks
  - Trimble Tekla Structures
  - Graphisoft ArchiCAD
  - Nemetschek Allplan
- Each plugin would export IFC + upload to a cloud endpoint
- Requires: cloud storage backend, plugin SDKs for each tool

### Cloud Upload & Federation
- Cloud-hosted model storage (upload from app or web)
- Multi-user model federation (combine models from different disciplines)
- Shareable project links with view state (camera, selection, section planes)
- Real-time collaboration: see other users' cursors and selections
- Requires: backend infrastructure (API server, storage, auth)

### BCF Cloud Sync (Enhanced)
- Current: local BCF project management with import/export
- Needed: direct BCF API server integration (BIMcollab, Trimble Connect, etc.)
- Auto-sync topics, comments, and viewpoints with cloud BCF servers
- Push notifications for new issues

---

## Priority Order

| Priority | Feature | Effort | Impact |
|----------|---------|--------|--------|
| 1 | Laser Measurement | Medium | High - core BIM workflow |
| 2 | 3-Axis Laser | Medium | High - positioning verification |
| 3 | 2D/3D Side-by-Side | Large | High - essential for floor plan review |
| 4 | Enhanced 2D Measurement | Medium | Medium - precision workflows |
| 5 | Cloud Upload & Federation | Very Large | Very High - multi-user workflows |
| 6 | BIM Tool Plugins | Very Large | Very High - adoption driver |
| 7 | BCF Cloud Sync | Medium | Medium - collaboration workflows |

---

## Notes

- Features 1-4 are implementable with current architecture (Rust renderer + Flutter UI)
- Features 5-7 require backend infrastructure and are a separate project scope
- Laser measurement requires extending the ray-casting system (currently used for element picking) to return surface normals and intersection distances
- 2D/3D side-by-side requires a second render target or a separate 2D canvas
