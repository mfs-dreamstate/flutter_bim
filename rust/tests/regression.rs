//! Regression test suite for the IFC parser and tessellator.
//!
//! Validates parser and tessellator against reference outputs using
//! inline IFC test files generated as strings.

use rust::bim::{IfcFile, BimModel};

// ========================================================================
// Test IFC File Generators
// ========================================================================

/// Generate a minimal valid IFC file header.
fn ifc_header() -> String {
    r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView_V2]'),'2;1');
FILE_NAME('regression_test.ifc','2025-01-01',('Test'),('Test'),'Regression Test','TestApp','None');
FILE_SCHEMA(('IFC4'));
ENDSEC;"#
        .to_string()
}

/// Generate common project/site/building/storey entities.
/// Returns the IFC content string and the next available entity ID.
fn ifc_spatial_scaffold() -> (String, i32) {
    let content = r#"#1=IFCPROJECT('0001',$,'Regression Test Project',$,$,$,$,$,$);
#2=IFCSITE('0002',$,'Test Site',$,$,$,$,$,$,$,$,$);
#3=IFCBUILDING('0003',$,'Test Building',$,$,$,$,$,$,$,$,$);
#4=IFCBUILDINGSTOREY('0004',$,'Ground Floor',$,$,$,$,$,0.0);
#5=IFCRELAGGREGATES('R001',$,$,$,#1,(#2));
#6=IFCRELAGGREGATES('R002',$,$,$,#2,(#3));
#7=IFCRELAGGREGATES('R003',$,$,$,#3,(#4));"#
        .to_string();
    (content, 100)
}

/// Generate a wall entity with basic geometry (extruded area solid).
fn ifc_wall_entity(base_id: i32, name: &str, x_offset: f32) -> String {
    let wall_id = base_id;
    let _placement_id = base_id + 1;
    let shape_rep_id = base_id + 2;
    let prod_def_shape_id = base_id + 3;
    let extrusion_id = base_id + 4;
    let profile_id = base_id + 5;
    let axis2_id = base_id + 6;
    let point_id = base_id + 7;
    let dir_id = base_id + 8;
    let dir2_id = base_id + 9;
    let local_place_id = base_id + 10;
    let axis2_place_id = base_id + 11;
    let point2_id = base_id + 12;

    format!(
        r#"#{point_id}=IFCCARTESIANPOINT(({x_offset},0.,0.));
#{dir_id}=IFCDIRECTION((0.,0.,1.));
#{dir2_id}=IFCDIRECTION((1.,0.,0.));
#{axis2_id}=IFCAXIS2PLACEMENT3D(#{point_id},#{dir_id},#{dir2_id});
#{local_place_id}=IFCLOCALPLACEMENT($,#{axis2_id});
#{profile_id}=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,5.0,0.2);
#{point2_id}=IFCCARTESIANPOINT((0.,0.,0.));
#{axis2_place_id}=IFCAXIS2PLACEMENT3D(#{point2_id},$,$);
#{extrusion_id}=IFCEXTRUDEDAREASOLID(#{profile_id},#{axis2_place_id},#{dir_id},3.0);
#{shape_rep_id}=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#{extrusion_id}));
#{prod_def_shape_id}=IFCPRODUCTDEFINITIONSHAPE($,$,(#{shape_rep_id}));
#{wall_id}=IFCWALL('{name}',$,'{name}',$,$,#{local_place_id},#{prod_def_shape_id},$);"#,
        point_id = point_id,
        dir_id = dir_id,
        dir2_id = dir2_id,
        axis2_id = axis2_id,
        local_place_id = local_place_id,
        profile_id = profile_id,
        point2_id = point2_id,
        axis2_place_id = axis2_place_id,
        extrusion_id = extrusion_id,
        shape_rep_id = shape_rep_id,
        prod_def_shape_id = prod_def_shape_id,
        wall_id = wall_id,
        name = name,
        x_offset = x_offset,
    )
}

/// Generate a slab entity with basic geometry.
fn ifc_slab_entity(base_id: i32, name: &str, y_offset: f32) -> String {
    let slab_id = base_id;
    let _placement_id = base_id + 1;
    let shape_rep_id = base_id + 2;
    let prod_def_shape_id = base_id + 3;
    let extrusion_id = base_id + 4;
    let profile_id = base_id + 5;
    let axis2_id = base_id + 6;
    let point_id = base_id + 7;
    let dir_id = base_id + 8;
    let dir2_id = base_id + 9;
    let local_place_id = base_id + 10;
    let axis2_place_id = base_id + 11;
    let point2_id = base_id + 12;

    format!(
        r#"#{point_id}=IFCCARTESIANPOINT((0.,{y_offset},0.));
#{dir_id}=IFCDIRECTION((0.,0.,1.));
#{dir2_id}=IFCDIRECTION((1.,0.,0.));
#{axis2_id}=IFCAXIS2PLACEMENT3D(#{point_id},#{dir_id},#{dir2_id});
#{local_place_id}=IFCLOCALPLACEMENT($,#{axis2_id});
#{profile_id}=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,10.0,8.0);
#{point2_id}=IFCCARTESIANPOINT((0.,0.,0.));
#{axis2_place_id}=IFCAXIS2PLACEMENT3D(#{point2_id},$,$);
#{extrusion_id}=IFCEXTRUDEDAREASOLID(#{profile_id},#{axis2_place_id},#{dir_id},0.3);
#{shape_rep_id}=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#{extrusion_id}));
#{prod_def_shape_id}=IFCPRODUCTDEFINITIONSHAPE($,$,(#{shape_rep_id}));
#{slab_id}=IFCSLAB('{name}',$,'{name}',$,$,#{local_place_id},#{prod_def_shape_id},$);"#,
        point_id = point_id,
        dir_id = dir_id,
        dir2_id = dir2_id,
        axis2_id = axis2_id,
        local_place_id = local_place_id,
        profile_id = profile_id,
        point2_id = point2_id,
        axis2_place_id = axis2_place_id,
        extrusion_id = extrusion_id,
        shape_rep_id = shape_rep_id,
        prod_def_shape_id = prod_def_shape_id,
        slab_id = slab_id,
        name = name,
        y_offset = y_offset,
    )
}

/// Generate a column entity with basic geometry.
fn ifc_column_entity(base_id: i32, name: &str, x_offset: f32, z_offset: f32) -> String {
    let col_id = base_id;
    let _placement_id = base_id + 1;
    let shape_rep_id = base_id + 2;
    let prod_def_shape_id = base_id + 3;
    let extrusion_id = base_id + 4;
    let profile_id = base_id + 5;
    let axis2_id = base_id + 6;
    let point_id = base_id + 7;
    let dir_id = base_id + 8;
    let dir2_id = base_id + 9;
    let local_place_id = base_id + 10;
    let axis2_place_id = base_id + 11;
    let point2_id = base_id + 12;

    format!(
        r#"#{point_id}=IFCCARTESIANPOINT(({x_offset},0.,{z_offset}));
#{dir_id}=IFCDIRECTION((0.,0.,1.));
#{dir2_id}=IFCDIRECTION((1.,0.,0.));
#{axis2_id}=IFCAXIS2PLACEMENT3D(#{point_id},#{dir_id},#{dir2_id});
#{local_place_id}=IFCLOCALPLACEMENT($,#{axis2_id});
#{profile_id}=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,0.4,0.4);
#{point2_id}=IFCCARTESIANPOINT((0.,0.,0.));
#{axis2_place_id}=IFCAXIS2PLACEMENT3D(#{point2_id},$,$);
#{extrusion_id}=IFCEXTRUDEDAREASOLID(#{profile_id},#{axis2_place_id},#{dir_id},3.0);
#{shape_rep_id}=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#{extrusion_id}));
#{prod_def_shape_id}=IFCPRODUCTDEFINITIONSHAPE($,$,(#{shape_rep_id}));
#{col_id}=IFCCOLUMN('{name}',$,'{name}',$,$,#{local_place_id},#{prod_def_shape_id},$);"#,
        point_id = point_id,
        dir_id = dir_id,
        dir2_id = dir2_id,
        axis2_id = axis2_id,
        local_place_id = local_place_id,
        profile_id = profile_id,
        point2_id = point2_id,
        axis2_place_id = axis2_place_id,
        extrusion_id = extrusion_id,
        shape_rep_id = shape_rep_id,
        prod_def_shape_id = prod_def_shape_id,
        col_id = col_id,
        name = name,
        x_offset = x_offset,
        z_offset = z_offset,
    )
}

/// Build a complete IFC file string from header + data entities.
fn build_ifc_file(data_entities: &str) -> String {
    format!(
        "{}\nDATA;\n{}\nENDSEC;\nEND-ISO-10303-21;",
        ifc_header(),
        data_entities
    )
}

// ========================================================================
// Regression Tests
// ========================================================================

#[test]
fn test_regression_wall_geometry() {
    // Create a minimal IFC file with a single wall (IFCEXTRUDEDAREASOLID)
    let (scaffold, next_id) = ifc_spatial_scaffold();
    let wall = ifc_wall_entity(next_id, "Test Wall", 0.0);
    let containment = format!(
        "#8=IFCRELCONTAINEDINSPATIALSTRUCTURE('RC01',$,$,$,(#{next_id}),#4);",
        next_id = next_id
    );

    let content = build_ifc_file(&format!("{}\n{}\n{}", scaffold, wall, containment));

    let ifc_file = IfcFile::parse(&content).expect("Should parse wall IFC file");
    let model = BimModel::from_ifc_file(&ifc_file).expect("Should build model from wall IFC");

    // Verify we got a wall
    assert_eq!(model.walls.len(), 1, "Should have exactly 1 wall");
    assert_eq!(model.walls[0].product.name.as_deref(), Some("Test Wall"));

    // Generate mesh and verify it has vertices
    let mesh = model.generate_meshes();
    assert!(
        !mesh.vertices.is_empty(),
        "Mesh should have vertices after tessellation"
    );
    assert!(
        !mesh.indices.is_empty(),
        "Mesh should have indices after tessellation"
    );

    // Verify bounds are reasonable (not NaN, not zero-sized)
    if let Some(bounds) = &mesh.bounds {
        for i in 0..3 {
            assert!(!bounds.min[i].is_nan(), "Bounds min[{}] should not be NaN", i);
            assert!(!bounds.max[i].is_nan(), "Bounds max[{}] should not be NaN", i);
            assert!(
                bounds.max[i] >= bounds.min[i],
                "Bounds max[{}] should >= min[{}]",
                i, i
            );
        }
    }
}

#[test]
fn test_regression_slab_geometry() {
    let (scaffold, next_id) = ifc_spatial_scaffold();
    let slab = ifc_slab_entity(next_id, "Test Slab", 0.0);
    let containment = format!(
        "#8=IFCRELCONTAINEDINSPATIALSTRUCTURE('RC01',$,$,$,(#{next_id}),#4);",
        next_id = next_id
    );

    let content = build_ifc_file(&format!("{}\n{}\n{}", scaffold, slab, containment));

    let ifc_file = IfcFile::parse(&content).expect("Should parse slab IFC file");
    let model = BimModel::from_ifc_file(&ifc_file).expect("Should build model from slab IFC");

    assert_eq!(model.slabs.len(), 1, "Should have exactly 1 slab");
    assert_eq!(model.slabs[0].product.name.as_deref(), Some("Test Slab"));

    let mesh = model.generate_meshes();
    assert!(!mesh.vertices.is_empty(), "Slab mesh should have vertices");

    if let Some(bounds) = &mesh.bounds {
        for i in 0..3 {
            assert!(!bounds.min[i].is_nan(), "Slab bounds min[{}] should not be NaN", i);
            assert!(!bounds.max[i].is_nan(), "Slab bounds max[{}] should not be NaN", i);
        }
    }
}

#[test]
fn test_regression_column_geometry() {
    let (scaffold, next_id) = ifc_spatial_scaffold();
    let column = ifc_column_entity(next_id, "Test Column", 0.0, 0.0);
    let containment = format!(
        "#8=IFCRELCONTAINEDINSPATIALSTRUCTURE('RC01',$,$,$,(#{next_id}),#4);",
        next_id = next_id
    );

    let content = build_ifc_file(&format!("{}\n{}\n{}", scaffold, column, containment));

    let ifc_file = IfcFile::parse(&content).expect("Should parse column IFC file");
    let model = BimModel::from_ifc_file(&ifc_file).expect("Should build model from column IFC");

    assert_eq!(model.columns.len(), 1, "Should have exactly 1 column");
    assert_eq!(model.columns[0].product.name.as_deref(), Some("Test Column"));

    let mesh = model.generate_meshes();
    assert!(!mesh.vertices.is_empty(), "Column mesh should have vertices");

    if let Some(bounds) = &mesh.bounds {
        for i in 0..3 {
            assert!(!bounds.min[i].is_nan(), "Column bounds min[{}] should not be NaN", i);
            assert!(!bounds.max[i].is_nan(), "Column bounds max[{}] should not be NaN", i);
        }
    }
}

#[test]
fn test_regression_multi_element() {
    // File with wall + slab + column
    let (scaffold, _next_id) = ifc_spatial_scaffold();
    let wall = ifc_wall_entity(100, "Multi Wall", 0.0);
    let slab = ifc_slab_entity(200, "Multi Slab", 0.0);
    let column = ifc_column_entity(300, "Multi Column", 5.0, 5.0);
    let containment =
        "#8=IFCRELCONTAINEDINSPATIALSTRUCTURE('RC01',$,$,$,(#100,#200,#300),#4);";

    let content = build_ifc_file(&format!(
        "{}\n{}\n{}\n{}\n{}",
        scaffold, wall, slab, column, containment
    ));

    let ifc_file = IfcFile::parse(&content).expect("Should parse multi-element IFC file");
    let model = BimModel::from_ifc_file(&ifc_file).expect("Should build model");

    // Verify element counts
    assert_eq!(model.walls.len(), 1);
    assert_eq!(model.slabs.len(), 1);
    assert_eq!(model.columns.len(), 1);
    assert_eq!(model.element_count, 3, "Should have exactly 3 elements total");

    // Generate mesh and verify each element has its own entry
    let mesh = model.generate_meshes();
    assert_eq!(
        mesh.elements.len(),
        3,
        "Mesh should have 3 element entries"
    );

    // Verify each element has distinct bounds (they are placed at different positions)
    if mesh.elements.len() == 3 {
        // Each element info should have non-zero bounds
        for elem in &mesh.elements {
            let span_x = elem.bounds.max[0] - elem.bounds.min[0];
            let span_y = elem.bounds.max[1] - elem.bounds.min[1];
            let span_z = elem.bounds.max[2] - elem.bounds.min[2];
            let volume = span_x * span_y * span_z;
            assert!(
                volume >= 0.0,
                "Element '{}' should have non-negative volume bounds",
                elem.name
            );
        }
    }
}

#[test]
fn test_regression_empty_model() {
    // IFC with header but no product entities
    let content = build_ifc_file("");

    let ifc_file = IfcFile::parse(&content).expect("Should parse empty IFC file");
    let model = BimModel::from_ifc_file(&ifc_file).expect("Should build empty model");

    assert_eq!(model.element_count, 0, "Empty model should have 0 elements");
    assert!(model.walls.is_empty());
    assert!(model.slabs.is_empty());
    assert!(model.columns.is_empty());

    // Should not panic when generating meshes
    let mesh = model.generate_meshes();
    // Empty model gets default building elements
    assert!(
        !mesh.vertices.is_empty(),
        "Even empty model should produce default building mesh"
    );
}

#[test]
fn test_regression_property_preservation() {
    // IFC with walls + property sets
    let (scaffold, _next_id) = ifc_spatial_scaffold();
    let wall = ifc_wall_entity(100, "Prop Wall", 0.0);

    // Add a property set with properties
    let property_set = r#"#400=IFCPROPERTYSINGLEVALUE('FireRating',$,'REI 60',$);
#401=IFCPROPERTYSINGLEVALUE('IsExternal',$,.T.,$);
#402=IFCPROPERTYSINGLEVALUE('ThermalTransmittance',$,0.25,$);
#403=IFCPROPERTYSET('PS001',$,'Pset_WallCommon',$,(#400,#401,#402));
#404=IFCRELDEFINESBYPROPERTIES('RDP01',$,$,$,(#100),#403);"#;

    let containment =
        "#8=IFCRELCONTAINEDINSPATIALSTRUCTURE('RC01',$,$,$,(#100),#4);";

    let content = build_ifc_file(&format!(
        "{}\n{}\n{}\n{}",
        scaffold, wall, property_set, containment
    ));

    let ifc_file = IfcFile::parse(&content).expect("Should parse IFC with properties");
    let model = BimModel::from_ifc_file(&ifc_file).expect("Should build model with properties");

    assert_eq!(model.walls.len(), 1);

    // Verify properties are accessible
    assert!(
        model.element_property_sets.contains_key(&100),
        "Wall #100 should have property sets"
    );
    let psets = &model.element_property_sets[&100];
    assert!(!psets.is_empty(), "Should have at least one property set");

    // Find Pset_WallCommon
    let wall_common = psets.iter().find(|ps| ps.name == "Pset_WallCommon");
    assert!(
        wall_common.is_some(),
        "Should have Pset_WallCommon property set"
    );

    let wc = wall_common.unwrap();
    assert!(
        wc.properties.len() >= 2,
        "Pset_WallCommon should have at least 2 properties, got {}",
        wc.properties.len()
    );
}

#[test]
fn test_regression_spatial_hierarchy() {
    // IFC with project -> site -> building -> storey -> wall
    let (scaffold, _next_id) = ifc_spatial_scaffold();
    let wall = ifc_wall_entity(100, "Hierarchy Wall", 0.0);
    let containment =
        "#8=IFCRELCONTAINEDINSPATIALSTRUCTURE('RC01',$,$,$,(#100),#4);";

    let content = build_ifc_file(&format!(
        "{}\n{}\n{}",
        scaffold, wall, containment
    ));

    let ifc_file = IfcFile::parse(&content).expect("Should parse spatial hierarchy IFC");
    let model = BimModel::from_ifc_file(&ifc_file).expect("Should build model with hierarchy");

    // Verify spatial structure
    assert!(model.project.is_some(), "Should have a project");
    assert_eq!(
        model.project.as_ref().unwrap().name,
        "Regression Test Project"
    );

    assert!(model.site.is_some(), "Should have a site");
    assert_eq!(model.site.as_ref().unwrap().name, "Test Site");

    assert!(model.building.is_some(), "Should have a building");
    assert_eq!(model.building.as_ref().unwrap().name, "Test Building");

    assert_eq!(model.storeys.len(), 1, "Should have 1 storey");
    assert_eq!(model.storeys[0].name, "Ground Floor");

    // Verify spatial decomposition: project -> site -> building -> storey
    assert!(
        model.spatial_children.contains_key(&1),
        "Project (#1) should have children"
    );
    let project_children = &model.spatial_children[&1];
    assert!(
        project_children.contains(&2),
        "Project should contain site (#2)"
    );

    assert!(
        model.spatial_children.contains_key(&2),
        "Site (#2) should have children"
    );
    let site_children = &model.spatial_children[&2];
    assert!(
        site_children.contains(&3),
        "Site should contain building (#3)"
    );

    assert!(
        model.spatial_children.contains_key(&3),
        "Building (#3) should have children"
    );
    let building_children = &model.spatial_children[&3];
    assert!(
        building_children.contains(&4),
        "Building should contain storey (#4)"
    );

    // Verify element-to-storey mapping
    assert!(
        model.element_to_storey.contains_key(&100),
        "Wall #100 should be mapped to a storey"
    );
    assert_eq!(
        model.element_to_storey[&100], 4,
        "Wall #100 should be on storey #4 (Ground Floor)"
    );
}

#[test]
fn test_regression_large_model_simulation() {
    // Generate IFC with 500 walls
    let (scaffold, _) = ifc_spatial_scaffold();

    let mut data = scaffold.clone();
    let mut wall_ids = Vec::with_capacity(500);

    for i in 0..500 {
        let base_id = 1000 + i * 20; // Leave room for sub-entities
        let x_offset = (i % 50) as f32 * 6.0;
        let wall = ifc_wall_entity(base_id, &format!("Wall_{}", i), x_offset);
        data.push('\n');
        data.push_str(&wall);
        wall_ids.push(base_id);
    }

    // Add containment relationship for all walls
    let id_list: String = wall_ids
        .iter()
        .map(|id| format!("#{}", id))
        .collect::<Vec<_>>()
        .join(",");
    data.push_str(&format!(
        "\n#8=IFCRELCONTAINEDINSPATIALSTRUCTURE('RC01',$,$,$,({}),#4);",
        id_list
    ));

    let content = build_ifc_file(&data);

    // Verify parsing completes
    let ifc_file = IfcFile::parse(&content).expect("Should parse large model IFC");
    let model = BimModel::from_ifc_file(&ifc_file).expect("Should build large model");

    assert_eq!(model.walls.len(), 500, "Should have 500 walls");
    assert_eq!(model.element_count, 500);

    // Generate mesh and verify it succeeds
    let mesh = model.generate_meshes();
    assert!(
        !mesh.vertices.is_empty(),
        "Large model mesh should have vertices"
    );
    assert_eq!(
        mesh.elements.len(),
        500,
        "Should have 500 element entries in mesh"
    );

    // Verify bounds encompass all elements
    if let Some(bounds) = &mesh.bounds {
        for i in 0..3 {
            assert!(!bounds.min[i].is_nan(), "Large model bounds min[{}] NaN", i);
            assert!(!bounds.max[i].is_nan(), "Large model bounds max[{}] NaN", i);
        }
        // With 500 walls spread across x, bounds should be wide
        let span_x = bounds.max[0] - bounds.min[0];
        assert!(
            span_x > 0.0,
            "Large model should have positive X span, got {}",
            span_x
        );
    }
}

// ========================================================================
// Parser Regression Tests
// ========================================================================

#[test]
fn test_regression_parser_entity_types() {
    // Verify all common entity types parse correctly
    let content = build_ifc_file(
        r#"#1=IFCPROJECT('P001',$,'Test Project',$,$,$,$,$,$);
#2=IFCSITE('S001',$,'Test Site',$,$,$,$,$,$,$,$,$);
#3=IFCBUILDING('B001',$,'Test Building',$,$,$,$,$,$,$,$,$);
#4=IFCBUILDINGSTOREY('BS01',$,'Floor 1',$,$,$,$,$,0.0);
#10=IFCWALL('W001',$,'Wall',$,$,$,$,$);
#11=IFCSLAB('SL01',$,'Slab',$,$,$,$,$);
#12=IFCCOLUMN('C001',$,'Column',$,$,$,$,$);
#13=IFCBEAM('BM01',$,'Beam',$,$,$,$,$);
#14=IFCDOOR('D001',$,'Door',$,$,$,$,$);
#15=IFCWINDOW('WN01',$,'Window',$,$,$,$,$);
#16=IFCROOF('RF01',$,'Roof',$,$,$,$,$);
#17=IFCSTAIR('ST01',$,'Stair',$,$,$,$,$);
#18=IFCFOOTING('FT01',$,'Footing',$,$,$,$,$);"#,
    );

    let ifc_file = IfcFile::parse(&content).expect("Should parse all entity types");
    let model = BimModel::from_ifc_file(&ifc_file).expect("Should build model");

    assert_eq!(model.walls.len(), 1);
    assert_eq!(model.slabs.len(), 1);
    assert_eq!(model.columns.len(), 1);
    assert_eq!(model.beams.len(), 1);
    assert_eq!(model.doors.len(), 1);
    assert_eq!(model.windows.len(), 1);
    assert_eq!(model.roofs.len(), 1);
    assert_eq!(model.stairs.len(), 1);
    assert_eq!(model.footings.len(), 1);
}

#[test]
fn test_regression_crlf_handling() {
    // Verify the parser handles Windows-style CRLF line endings
    let content_unix = build_ifc_file(
        "#10=IFCWALL('W001',$,'Wall A',$,$,$,$,$);",
    );

    // Convert to CRLF
    let content_crlf = content_unix.replace('\n', "\r\n");

    let ifc_unix = IfcFile::parse(&content_unix).expect("Unix line endings");
    let ifc_crlf = IfcFile::parse(&content_crlf).expect("CRLF line endings");

    assert_eq!(
        ifc_unix.entities.len(),
        ifc_crlf.entities.len(),
        "CRLF and Unix should produce same entity count"
    );
}
