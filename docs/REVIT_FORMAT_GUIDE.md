# Revit IFC Export Guide for Flutter BIM Viewer

This guide explains how to export IFC files from Autodesk Revit for use with the Flutter BIM Viewer. It covers recommended settings, step-by-step instructions, common issues, and what geometry types the viewer supports.

---

## 1. Recommended IFC Export Settings

### IFC Version

| Version | Recommendation | Notes |
|---------|---------------|-------|
| **IFC4 Reference View** | **Preferred** | Produces tessellated geometry (TriangulatedFaceSet, PolygonalFaceSet) that the viewer handles directly. Smaller file sizes. |
| IFC4 Design Transfer View | Use if you need editable geometry downstream | Produces solid geometry (ExtrudedAreaSolid, BooleanClippingResult). Larger files but richer data. |
| IFC2x3 Coordination View 2.0 | Fallback for older workflows | Widely supported. The viewer handles both IFC2x3 and IFC4 schemas. |
| IFC4x3 | Avoid for now | Infrastructure-focused extensions. The viewer recognizes the schema but some IFC4x3-specific entities may not yet be fully supported. |

**Bottom line**: Use **IFC4 Reference View** unless you have a specific reason not to. It produces the cleanest tessellated geometry and the smallest files.

### Export Scope

- **Visible elements in current view**: Best for controlled exports. Set up a 3D view with the elements you want, then export that view.
- **Selected elements**: Useful for exporting a subset (e.g., structural frame only).
- **Entire model**: Produces the largest file. Only use for full-model coordination.

### Geometry Options

| Setting | Recommended Value | Why |
|---------|-------------------|-----|
| Tessellation Level of Detail | Medium | Balance between file size and visual quality. High produces smoother curves but much larger files. |
| Triangulate Faces | On (for Reference View) | The viewer works with triangle meshes. Reference View enables this by default. |
| Include IFCSITE Geometry | Off | Site surfaces add complexity without much visual value in a BIM viewer. |
| Export Linked Files | Off (unless needed) | Each linked file adds to the model size significantly. |

### Property Sets

| Setting | Recommended Value | Why |
|---------|-------------------|-----|
| Export Revit property sets | On | Makes Revit parameters available as IFC property sets for element inspection. |
| Export IFC common property sets | On | Adds standard Pset_ property sets for interoperability. |
| Export base quantities | On (for QTO) | Adds Qto_ quantity sets (area, volume, length) useful for quantity takeoff. Off if you only need visualization. |
| Export schedules as property sets | Optional | Converts Revit schedule data into property sets. |

### Space Boundaries

| Setting | When to Use |
|---------|-------------|
| None | Visualization and coordination only. |
| 1st Level | Basic room/space boundary relationships. |
| 2nd Level | Energy analysis. Significant export time increase. |

For most BIM coordination and viewing workflows, set space boundaries to **None** or **1st Level**.

---

## 2. Step-by-Step Export Instructions

These steps apply to Revit 2020 through 2025. The dialogs are nearly identical across versions.

### Basic Export

1. Open your Revit model.
2. (Optional) Set up a 3D view showing only the elements you want to export. Use Visibility/Graphics Overrides to control which categories appear.
3. Go to **File > Export > IFC**.
4. Choose a save location and file name.
5. In the **IFC Class** dropdown, select the IFC version (see version recommendations above).
6. Click **Modify setup...** to configure advanced options.

### IFC Export Setup Dialog

The setup dialog has several tabs:

#### General Tab
- **IFC version**: Select IFC4 Reference View.
- **File type**: IFC (not IFCXML or IFCZIP, unless you specifically need compressed output).
- **Phase to export**: Select the appropriate design phase. "New Construction" is typical.

#### Additional Content Tab
- **Export 2D plan view elements**: Off (the BIM viewer renders 3D only).
- **Export linked Revit files**: Off unless you need linked models in the same IFC file.
- **Export only elements visible in view**: On if you prepared a filtered 3D view.

#### Property Sets Tab
- **Export Revit property sets**: On.
- **Export IFC Common property sets**: On.
- **Export base quantities**: On if you need measurement data, off otherwise.
- **Export user-defined property sets**: On if you have custom shared parameter mappings.

#### Level of Detail Tab
- **Level of detail for some element geometry**: Medium.
  - Low: Fastest export, coarsest curves.
  - Medium: Good balance.
  - High: Smoothest curves, largest files.

#### Advanced Tab
- **Export parts as building elements**: On if you use Revit Parts.
- **Allow mixed "Solid Model" and "Surface Model" representations**: On.
- **Use active view when creating geometry**: On (ensures V/G overrides are respected).
- **Store IFC GUID in element after export**: Optional, useful for round-tripping.

### Recommended Presets

For **BIM coordination** (clash detection, design review):
```
IFC Version: IFC4 Reference View
Phase: New Construction
Export visible in view: On
Revit property sets: On
IFC common property sets: On
Base quantities: Off
Level of detail: Medium
```

For **quantity takeoff** (cost estimation, material scheduling):
```
IFC Version: IFC4 Design Transfer View
Phase: New Construction
Export visible in view: Off (export everything)
Revit property sets: On
IFC common property sets: On
Base quantities: On
Level of detail: Low (geometry fidelity matters less)
```

---

## 3. Common Issues and Solutions

### Missing Geometry (Elements Export Without Visible Shape)

**Symptoms**: Elements appear in the IFC element tree but render as placeholder boxes or are invisible.

**Causes and fixes**:
- **Category is hidden in the export view**: Open the 3D view you are exporting from. Go to Visibility/Graphics (VV) and ensure the category is checked. Elements hidden in the view are excluded from the export.
- **Element is in a different Phase**: Check the phase filter on your view. Elements in demolished or future phases may be filtered out.
- **Design Option conflict**: If the element belongs to a non-active Design Option, it will not export. Set the correct Design Option before exporting.
- **Workset not loaded**: In workshared models, unloaded worksets mean their elements have no geometry to export. Load all relevant worksets.

### Wrong Scale or Misaligned Geometry

**Symptoms**: Model appears extremely large, extremely small, or offset from origin.

**Causes and fixes**:
- **Project units mismatch**: Check **Manage > Project Units** in Revit. The IFC export uses the project's length unit. The viewer detects the unit from the IFCUNITASSIGNMENT entity and converts to meters internally. If units still look wrong, confirm the IFC file's unit header is correct by inspecting the first few lines.
- **Project Base Point offset**: A large offset between the Project Base Point and the Internal Origin can push geometry far from the IFC origin. Consider temporarily moving the Project Base Point to the Internal Origin before export, or use the "Export using shared coordinates" option.
- **Survey Point / Shared Coordinates**: If shared coordinates are set up, the export may use the survey point as the origin. This is correct for GIS integration (IFCMAPCONVERSION) but may look offset in viewers that don't apply the geo-transform.

### Missing Properties

**Symptoms**: Element properties panel is empty or missing expected parameters.

**Causes and fixes**:
- **"Export Revit property sets" is off**: Enable it in the IFC Export Setup > Property Sets tab.
- **Parameters not in shared parameters file**: Instance parameters defined only within a single family may not export. Add them to the project's shared parameters file.
- **Custom property set mapping needed**: For fine-grained control, create an IFC Export property set mapping text file and reference it in the export settings.

### Large File Size

**Symptoms**: Exported IFC file is much larger than expected (hundreds of MB for a simple building).

**Causes and fixes**:
- **Use IFC4 Reference View instead of Design Transfer View**: Reference View produces tessellated geometry, which is typically 30-50% smaller than solid geometry representations.
- **Reduce Level of Detail**: High LOD produces very dense tessellation for curved elements (pipes, columns, railings). Switch to Medium or Low.
- **Export only visible elements**: Use a filtered 3D view instead of exporting the entire model.
- **Exclude room/space geometry**: Turn off "Export rooms in 3D views" if rooms are not needed.
- **Remove linked files**: Each linked file multiplied by instance count adds to file size. Export links separately if needed.

### Boolean Geometry Issues

**Symptoms**: Walls or slabs have missing cutouts, or openings are not visible.

**Causes and fixes**:
- **Increase tessellation quality**: Some Boolean operations (wall openings, slab penetrations) require higher tessellation precision. Try the High LOD setting.
- **BooleanClippingResult complexity**: Revit exports openings as IFCBOOLEANCLIPPINGRESULT or IFCOPENINGELEMENT. The viewer attempts to resolve these, but deeply nested Boolean trees may fall back to bounding boxes. Simplifying the model (e.g., editing in-place families with many voids) can help.
- **Use Reference View**: The Reference View tessellates Boolean results during export, producing simpler triangle meshes instead of CSG trees.

### Missing Materials / Wrong Colors

**Symptoms**: All elements are gray or colors do not match Revit materials.

**Causes and fixes**:
- **Assign materials in Revit**: Elements without material assignments export with no color information. The viewer applies default colors by element type (walls are light beige, columns are blue-gray, etc.) when no material color is present.
- **Material mapping**: The IFC export maps Revit materials to IFCSTYLEDITEM / IFCSURFACESTYLE entities. Check that materials have a Shading color defined (not just a render appearance).
- **Check "Export Revit materials"**: Some IFC export configurations skip material export. Ensure it is enabled.

---

## 4. Supported Geometry from Revit

### Revit Category to IFC Entity Mapping

| Revit Category | IFC Entity | Viewer Support |
|---------------|------------|----------------|
| Walls | IfcWall, IfcWallStandardCase | Full tessellation |
| Floors | IfcSlab (FLOOR) | Full tessellation |
| Roofs | IfcRoof, IfcSlab (ROOF) | Full tessellation |
| Ceilings | IfcCovering (CEILING) | Full tessellation |
| Columns | IfcColumn | Full tessellation |
| Beams, Framing | IfcBeam, IfcMember | Full tessellation |
| Doors | IfcDoor | Full tessellation |
| Windows | IfcWindow | Full tessellation |
| Stairs | IfcStair, IfcStairFlight | Full tessellation |
| Railings | IfcRailing | Full tessellation |
| Curtain Walls | IfcCurtainWall, IfcPlate | Full tessellation |
| Furniture | IfcFurnishingElement | Full tessellation |
| Generic Models | IfcBuildingElementProxy | Full tessellation |
| Footings | IfcFooting | Full tessellation |
| Pipes | IfcPipeSegment | Full tessellation |
| Ducts | IfcDuctSegment | Full tessellation |
| Cable Trays | IfcCableCarrierSegment | Full tessellation |
| Mechanical Equipment | IfcFlowTerminal | Full tessellation |
| Rooms / Spaces | IfcSpace | Element tree only (no 3D rendering) |
| Grids | IfcGrid | Grid lines displayed as overlay |

### Geometry Representation Types

The viewer's Rust tessellation engine handles the following IFC geometry types. These are listed in order from most commonly produced by Revit to least common:

| IFC Geometry Type | Typical Revit Source | Viewer Handling |
|-------------------|---------------------|-----------------|
| **IfcExtrudedAreaSolid** | Walls, slabs, beams, columns with simple profiles | Direct tessellation with extrusion path |
| **IfcBooleanClippingResult** | Walls/slabs with openings, joined geometry | Resolves Boolean tree and tessellates |
| **IfcFacetedBrep** | Complex families, in-place models | Direct face tessellation |
| **IfcTriangulatedFaceSet** | IFC4 Reference View export | Direct triangle import (fastest path) |
| **IfcPolygonalFaceSet** | IFC4 Reference View export | Triangulated and imported |
| **IfcMappedItem** | Repeated families (windows, doors, furniture) | Cached tessellation with per-instance transforms |
| **IfcFaceBasedSurfaceModel** | Sheet-based geometry, surfaces | Face tessellation |
| **IfcShellBasedSurfaceModel** | Open shells, surface models | Shell tessellation |
| **IfcSweptDiskSolid** | Pipes, circular sweep paths | Swept profile tessellation |
| **IfcRevolvedAreaSolid** | Revolved families | Revolution tessellation |

### Fallback Behavior

When the viewer encounters a geometry representation it cannot tessellate (e.g., NURBS surfaces, advanced Boolean trees, or corrupted geometry data), it falls back to rendering a **placeholder bounding box** for that element. The element still appears in the element tree and its properties remain inspectable.

The viewer logs the count of real tessellations vs. fallback boxes during model loading. If you see a high fallback count, try:
1. Re-exporting with IFC4 Reference View (pre-tessellated geometry).
2. Increasing the tessellation Level of Detail in Revit.
3. Simplifying complex in-place families.

---

## 5. Alternative Approaches

### Third-Party IFC Export Plugins

If Revit's built-in IFC exporter does not meet your needs, consider:

- **Autodesk IFC Exporter (open source)**: The built-in exporter is actually the open-source IFC exporter maintained by Autodesk on GitHub. You can install a newer version from the Autodesk App Store that may fix bugs present in your Revit version's bundled exporter.
- **Geometry Gym IFC**: A Revit add-in with more control over IFC mapping and property set configuration.
- **Simplebim**: Post-processes IFC files to clean up, merge, or trim content. Useful for reducing file size and fixing property issues after export.

### Validating Your Export

Before loading an IFC file in the Flutter BIM Viewer, you can validate it:

1. **Revit "Open IFC"**: Use File > Open > IFC to re-import the exported file into a new Revit project. This shows you exactly what geometry and data survived the round-trip.
2. **BIM Vision** (free desktop viewer): Quick check of geometry and properties.
3. **IFC Syntax Check**: Open the `.ifc` file in a text editor and verify the FILE_SCHEMA line matches your expected version (`IFC4` or `IFC2X3`).
4. **buildingSMART Validation Service**: Upload your IFC file to the buildingSMART validation service for schema compliance checking.

### Direct Revit to glTF (Visualization Only)

If you only need visualization without BIM data (no properties, no element tree, no IFC entity types):

- Use plugins like **Revit to glTF** or **3D Repo** for mesh-only export.
- These produce lightweight triangle meshes but lose all IFC metadata.
- This is **not recommended** for BIM workflows, only for quick visual previews.

---

## 6. Performance Tips for Large Models

### Preparing the Model

- **Split by discipline**: Export architectural, structural, and MEP models as separate IFC files rather than one combined file.
- **Use Revit view filters**: Create a 3D view that excludes non-essential categories (detail items, annotations, model lines).
- **Purge unused families**: Reduce the model complexity before export. (Manage > Purge Unused)

### Export Settings for Performance

- **IFC4 Reference View**: Always the best choice for viewing performance because geometry arrives pre-tessellated.
- **Level of Detail: Low or Medium**: High LOD on a large model can produce IFC files exceeding 500 MB.
- **Exclude spaces and rooms**: Room geometry is complex and usually unnecessary for coordination viewing.

### Viewer Behavior with Large Files

The viewer uses several strategies for large models:

- **Streaming/chunked parsing**: Files over 50 MB are parsed incrementally to avoid doubling memory usage.
- **Parallel tessellation**: Multi-threaded geometry processing using rayon.
- **LOD system**: Distant elements automatically switch to lower detail levels (Medium at 50% triangles, Low at 25%, bounding box beyond a distance threshold).
- **MappedItem caching**: Repeated geometry (e.g., 500 identical windows) is tessellated once and instanced with per-element transforms.

For the best experience with large models (100+ MB IFC files), ensure the device has at least 4 GB of available memory.

---

## Quick Reference: Export Checklist

- [ ] Set up a filtered 3D view with only the categories you need
- [ ] Verify project units (Manage > Project Units > Length)
- [ ] Check that materials are assigned to elements you want colored
- [ ] File > Export > IFC
- [ ] Select IFC4 Reference View
- [ ] Click Modify Setup:
  - [ ] General: IFC file type, correct phase
  - [ ] Additional Content: Export visible in view = On
  - [ ] Property Sets: Revit property sets = On, IFC common = On
  - [ ] Level of Detail: Medium
- [ ] Export
- [ ] Validate: Open the exported .ifc file in the viewer and check element count, geometry quality, and properties
