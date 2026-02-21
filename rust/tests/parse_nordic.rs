//! Integration test: parse NordicLCA structural + electrical IFC files

use rust::bim::{IfcFile, BimModel};

#[test]
fn test_parse_struc_nordic() {
    let path = "../example/assets/nordic/STRUC_NordicLCA_Housing_Concrete_BuildingPermit.ifc";
    let content = std::fs::read_to_string(path).expect("read STRUC file");
    let ifc = IfcFile::parse(&content).expect("parse STRUC IFC");
    println!("STRUC entities: {}", ifc.entity_count());
    let model = BimModel::from_ifc_file(&ifc).expect("build STRUC model");
    println!("STRUC element_count: {}", model.element_count);
    let mesh = model.generate_meshes();
    println!("STRUC mesh: {} verts, {} indices, {} elements",
        mesh.vertices.len()/3, mesh.indices.len(), mesh.elements.len());

    // Phase 2: Verify property sets were parsed
    println!("STRUC elements with properties: {}", model.element_property_sets.len());
    println!("STRUC elements with storey mapping: {}", model.element_to_storey.len());
    println!("STRUC spatial children entries: {}", model.spatial_children.len());
    println!("STRUC storeys: {}", model.storeys.len());

    // Print first few property sets for inspection
    let mut count = 0;
    for (elem_id, psets) in &model.element_property_sets {
        if count >= 3 { break; }
        println!("  Element #{}: {} property sets", elem_id, psets.len());
        for ps in psets {
            println!("    [{}]: {} properties", ps.name, ps.properties.len());
            for (k, v) in ps.properties.iter().take(3) {
                println!("      {} = {}", k, v);
            }
        }
        count += 1;
    }

    // Phase 2.5: Verify material and type object associations
    println!("STRUC elements with materials: {}", model.element_materials.len());
    println!("STRUC elements with type objects: {}", model.element_type_objects.len());
    let mut mat_count = 0;
    for (elem_id, mat) in &model.element_materials {
        if mat_count >= 5 { break; }
        println!("  Element #{}: material='{}' category={:?} layers={}",
            elem_id, mat.name, mat.category, mat.layers.len());
        for layer in &mat.layers {
            println!("    layer: '{}' thickness={:?}", layer.material_name, layer.thickness);
        }
        mat_count += 1;
    }

    // Print a few type objects
    let mut type_count = 0;
    for (elem_id, type_obj) in &model.element_type_objects {
        if type_count >= 5 { break; }
        println!("  Element #{}: type='{}' ifc_type='{}' psets={}",
            elem_id, type_obj.type_name, type_obj.ifc_type, type_obj.property_sets.len());
        for ps in &type_obj.property_sets {
            println!("    [{}]: {} properties", ps.name, ps.properties.len());
        }
        type_count += 1;
    }
}

#[test]
fn test_parse_ele_nordic() {
    let path = "../example/assets/nordic/ELE_NordicLCA_Housing_Concrete_BuildingPermit.ifc";
    let content = std::fs::read_to_string(path).expect("read ELE file");
    let ifc = IfcFile::parse(&content).expect("parse ELE IFC");
    println!("ELE entities: {}", ifc.entity_count());
    let model = BimModel::from_ifc_file(&ifc).expect("build ELE model");
    println!("ELE element_count: {}", model.element_count);
    let mesh = model.generate_meshes();
    println!("ELE mesh: {} verts, {} indices, {} elements",
        mesh.vertices.len()/3, mesh.indices.len(), mesh.elements.len());
}
