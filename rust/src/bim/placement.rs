//! Coordinate Transform Resolution
//!
//! Resolves IFCLOCALPLACEMENT chains into `glam::Mat4` world matrices.
//! Handles IFCAXIS2PLACEMENT3D, IFCAXIS2PLACEMENT2D, IFCCARTESIANTRANSFORMATIONOPERATOR3D.

use std::collections::HashMap;

use glam::{Mat4, Vec3, Vec4};

use super::entities::{EntityId, IfcEntity, IfcValue};

/// Cache for resolved placements to avoid redundant recursion.
pub struct PlacementCache {
    cache: HashMap<EntityId, Mat4>,
}

impl PlacementCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }
}

/// Read an IFCCARTESIANPOINT entity and return as Vec3.
/// IFCCARTESIANPOINT(Coordinates: LIST OF REAL)
pub fn read_cartesian_point(id: EntityId, entities: &HashMap<EntityId, IfcEntity>) -> Option<Vec3> {
    let entity = entities.get(&id)?;
    if !entity.entity_type.eq_ignore_ascii_case("IFCCARTESIANPOINT") {
        return None;
    }
    let coords = entity.get_list(0)?;
    let x = value_as_f32(coords.get(0)?)?;
    let y = value_as_f32(coords.get(1)?)?;
    let z = coords.get(2).and_then(|v| value_as_f32(v)).unwrap_or(0.0);
    Some(Vec3::new(x, y, z))
}

/// Read an IFCDIRECTION entity and return as Vec3 (normalized).
/// IFCDIRECTION(DirectionRatios: LIST OF REAL)
pub fn read_direction(id: EntityId, entities: &HashMap<EntityId, IfcEntity>) -> Option<Vec3> {
    let entity = entities.get(&id)?;
    if !entity.entity_type.eq_ignore_ascii_case("IFCDIRECTION") {
        return None;
    }
    let ratios = entity.get_list(0)?;
    let x = value_as_f32(ratios.get(0)?)?;
    let y = value_as_f32(ratios.get(1)?)?;
    let z = ratios.get(2).and_then(|v| value_as_f32(v)).unwrap_or(0.0);
    let v = Vec3::new(x, y, z);
    if v.length_squared() < 1e-12 {
        return None;
    }
    Some(v.normalize())
}

/// Resolve IFCAXIS2PLACEMENT3D into a 4x4 transformation matrix.
/// IFCAXIS2PLACEMENT3D(Location, Axis, RefDirection)
///   Location: IFCCARTESIANPOINT
///   Axis: IFCDIRECTION (optional, default Z)
///   RefDirection: IFCDIRECTION (optional, default X)
pub fn axis2_placement_3d_to_matrix(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
) -> Mat4 {
    let entity = match entities.get(&id) {
        Some(e) => e,
        None => return Mat4::IDENTITY,
    };

    // Location (attribute 0)
    let location = entity
        .get_entity_ref(0)
        .and_then(|ref_id| read_cartesian_point(ref_id, entities))
        .unwrap_or(Vec3::ZERO);

    // Axis (attribute 1) - Z axis direction
    let axis = entity
        .get_entity_ref(1)
        .and_then(|ref_id| read_direction(ref_id, entities))
        .unwrap_or(Vec3::Z);

    // RefDirection (attribute 2) - X axis direction
    let ref_dir = entity
        .get_entity_ref(2)
        .and_then(|ref_id| read_direction(ref_id, entities))
        .unwrap_or_else(|| {
            // Derive a default X direction perpendicular to axis
            default_ref_direction(axis)
        });

    // Build orthonormal basis: Z = axis, X = ref_dir adjusted, Y = Z x X
    let z_axis = axis.normalize();
    // Make ref_dir perpendicular to z_axis
    let x_axis = (ref_dir - z_axis * z_axis.dot(ref_dir)).normalize();
    let y_axis = z_axis.cross(x_axis);

    Mat4::from_cols(
        Vec4::new(x_axis.x, x_axis.y, x_axis.z, 0.0),
        Vec4::new(y_axis.x, y_axis.y, y_axis.z, 0.0),
        Vec4::new(z_axis.x, z_axis.y, z_axis.z, 0.0),
        Vec4::new(location.x, location.y, location.z, 1.0),
    )
}

/// Resolve IFCAXIS2PLACEMENT2D into a Vec3 offset (for profile positions).
/// IFCAXIS2PLACEMENT2D(Location, RefDirection)
pub fn axis2_placement_2d_offset(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
) -> Vec3 {
    let entity = match entities.get(&id) {
        Some(e) => e,
        None => return Vec3::ZERO,
    };

    entity
        .get_entity_ref(0)
        .and_then(|ref_id| read_cartesian_point(ref_id, entities))
        .unwrap_or(Vec3::ZERO)
}

/// Resolve an IFCLOCALPLACEMENT entity recursively (with caching).
/// IFCLOCALPLACEMENT(PlacementRelTo, RelativePlacement)
///   PlacementRelTo: optional IFCLOCALPLACEMENT (parent)
///   RelativePlacement: IFCAXIS2PLACEMENT3D
pub fn resolve_local_placement(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    cache: &mut PlacementCache,
) -> Mat4 {
    if let Some(&cached) = cache.cache.get(&id) {
        return cached;
    }

    let entity = match entities.get(&id) {
        Some(e) => e,
        None => return Mat4::IDENTITY,
    };

    if !entity.entity_type.eq_ignore_ascii_case("IFCLOCALPLACEMENT") {
        return Mat4::IDENTITY;
    }

    // Parent placement (attribute 0)
    let parent_matrix = match entity.get_entity_ref(0) {
        Some(parent_id) => resolve_local_placement(parent_id, entities, cache),
        None => Mat4::IDENTITY,
    };

    // Relative placement (attribute 1) - IFCAXIS2PLACEMENT3D
    let relative_matrix = match entity.get_entity_ref(1) {
        Some(rel_id) => axis2_placement_3d_to_matrix(rel_id, entities),
        None => Mat4::IDENTITY,
    };

    let result = parent_matrix * relative_matrix;
    cache.cache.insert(id, result);
    result
}

/// Resolve the world matrix for a product (IFCWALL, IFCSLAB, etc.).
/// Products have ObjectPlacement at attribute index 5.
pub fn resolve_product_placement(
    product_id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
    cache: &mut PlacementCache,
) -> Mat4 {
    let entity = match entities.get(&product_id) {
        Some(e) => e,
        None => return Mat4::IDENTITY,
    };

    // ObjectPlacement is at attribute index 5 for most IFC products
    match entity.get_entity_ref(5) {
        Some(placement_id) => resolve_local_placement(placement_id, entities, cache),
        None => Mat4::IDENTITY,
    }
}

/// Resolve IFCCARTESIANTRANSFORMATIONOPERATOR3D into a Mat4.
/// Used by IFCMAPPEDITEM for instance transforms.
/// IFCCARTESIANTRANSFORMATIONOPERATOR3D(Axis1, Axis2, LocalOrigin, Scale, Axis3)
pub fn cartesian_transform_operator_3d(
    id: EntityId,
    entities: &HashMap<EntityId, IfcEntity>,
) -> Mat4 {
    let entity = match entities.get(&id) {
        Some(e) => e,
        None => return Mat4::IDENTITY,
    };

    // LocalOrigin (attribute 2)
    let origin = entity
        .get_entity_ref(2)
        .and_then(|ref_id| read_cartesian_point(ref_id, entities))
        .unwrap_or(Vec3::ZERO);

    // Scale (attribute 3)
    let scale = entity.get_real(3).unwrap_or(1.0) as f32;

    // Axis1 (attribute 0) - X direction
    let axis1 = entity
        .get_entity_ref(0)
        .and_then(|ref_id| read_direction(ref_id, entities))
        .unwrap_or(Vec3::X);

    // Axis2 (attribute 1) - Y direction
    let axis2 = entity
        .get_entity_ref(1)
        .and_then(|ref_id| read_direction(ref_id, entities))
        .unwrap_or(Vec3::Y);

    // Axis3 (attribute 4) - Z direction
    let axis3 = entity
        .get_entity_ref(4)
        .and_then(|ref_id| read_direction(ref_id, entities))
        .unwrap_or(Vec3::Z);

    Mat4::from_cols(
        Vec4::new(axis1.x * scale, axis1.y * scale, axis1.z * scale, 0.0),
        Vec4::new(axis2.x * scale, axis2.y * scale, axis2.z * scale, 0.0),
        Vec4::new(axis3.x * scale, axis3.y * scale, axis3.z * scale, 0.0),
        Vec4::new(origin.x, origin.y, origin.z, 1.0),
    )
}

/// Derive a default reference direction perpendicular to the given axis.
fn default_ref_direction(axis: Vec3) -> Vec3 {
    // Pick the axis least aligned with `axis`
    let abs_axis = axis.abs();
    let candidate = if abs_axis.x <= abs_axis.y && abs_axis.x <= abs_axis.z {
        Vec3::X
    } else if abs_axis.y <= abs_axis.z {
        Vec3::Y
    } else {
        Vec3::Z
    };
    // Gram-Schmidt: remove component along axis
    let ref_dir = candidate - axis * axis.dot(candidate);
    if ref_dir.length_squared() < 1e-12 {
        Vec3::X
    } else {
        ref_dir.normalize()
    }
}

/// Extract a float from an IfcValue.
fn value_as_f32(v: &IfcValue) -> Option<f32> {
    match v {
        IfcValue::Real(r) => Some(*r as f32),
        IfcValue::Integer(i) => Some(*i as f32),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity(id: EntityId, entity_type: &str, attrs: Vec<IfcValue>) -> IfcEntity {
        IfcEntity {
            id,
            entity_type: entity_type.to_string(),
            attributes: attrs,
        }
    }

    #[test]
    fn test_read_cartesian_point() {
        let mut entities = HashMap::new();
        entities.insert(
            1,
            make_entity(1, "IFCCARTESIANPOINT", vec![
                IfcValue::List(vec![
                    IfcValue::Real(1.0),
                    IfcValue::Real(2.0),
                    IfcValue::Real(3.0),
                ]),
            ]),
        );
        let pt = read_cartesian_point(1, &entities).unwrap();
        assert!((pt.x - 1.0).abs() < 1e-6);
        assert!((pt.y - 2.0).abs() < 1e-6);
        assert!((pt.z - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_read_direction() {
        let mut entities = HashMap::new();
        entities.insert(
            1,
            make_entity(1, "IFCDIRECTION", vec![
                IfcValue::List(vec![
                    IfcValue::Real(0.0),
                    IfcValue::Real(0.0),
                    IfcValue::Real(1.0),
                ]),
            ]),
        );
        let dir = read_direction(1, &entities).unwrap();
        assert!((dir.z - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_identity_placement() {
        let entities = HashMap::new();
        let mut cache = PlacementCache::new();
        let mat = resolve_local_placement(999, &entities, &mut cache);
        assert_eq!(mat, Mat4::IDENTITY);
    }

    #[test]
    fn test_simple_placement_chain() {
        let mut entities = HashMap::new();

        // Point at origin
        entities.insert(
            10,
            make_entity(10, "IFCCARTESIANPOINT", vec![
                IfcValue::List(vec![
                    IfcValue::Real(0.0),
                    IfcValue::Real(0.0),
                    IfcValue::Real(0.0),
                ]),
            ]),
        );

        // Point at (5, 0, 0)
        entities.insert(
            11,
            make_entity(11, "IFCCARTESIANPOINT", vec![
                IfcValue::List(vec![
                    IfcValue::Real(5.0),
                    IfcValue::Real(0.0),
                    IfcValue::Real(0.0),
                ]),
            ]),
        );

        // Axis2Placement3D at origin
        entities.insert(
            20,
            make_entity(20, "IFCAXIS2PLACEMENT3D", vec![
                IfcValue::EntityRef(10),
                IfcValue::Null,
                IfcValue::Null,
            ]),
        );

        // Axis2Placement3D at (5,0,0)
        entities.insert(
            21,
            make_entity(21, "IFCAXIS2PLACEMENT3D", vec![
                IfcValue::EntityRef(11),
                IfcValue::Null,
                IfcValue::Null,
            ]),
        );

        // Parent placement
        entities.insert(
            30,
            make_entity(30, "IFCLOCALPLACEMENT", vec![
                IfcValue::Null,
                IfcValue::EntityRef(20),
            ]),
        );

        // Child placement relative to parent, offset by (5,0,0)
        entities.insert(
            31,
            make_entity(31, "IFCLOCALPLACEMENT", vec![
                IfcValue::EntityRef(30),
                IfcValue::EntityRef(21),
            ]),
        );

        let mut cache = PlacementCache::new();
        let mat = resolve_local_placement(31, &entities, &mut cache);

        // The resulting translation should be at (5, 0, 0)
        let pos = mat.col(3);
        assert!((pos.x - 5.0).abs() < 1e-4, "x={}", pos.x);
        assert!((pos.y - 0.0).abs() < 1e-4, "y={}", pos.y);
        assert!((pos.z - 0.0).abs() < 1e-4, "z={}", pos.z);
    }
}
