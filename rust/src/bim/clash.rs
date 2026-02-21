//! Clash Detection Engine
//!
//! AABB broad-phase + triangle-level narrow-phase intersection testing.

use crate::bim::geometry::BoundingBox;
use crate::bim::model::ElementInfo;

// ============================================================================
// Data Structures
// ============================================================================

/// The type/severity of a detected clash.
#[derive(Debug, Clone, PartialEq)]
pub enum ClashType {
    /// Geometric intersection (penetration).
    Hard,
    /// Within soft tolerance but not intersecting.
    Soft,
    /// Within clearance tolerance.
    Clearance,
}

impl ClashType {
    /// Convert to integer code for FFI (0=Hard, 1=Soft, 2=Clearance).
    pub fn as_i32(&self) -> i32 {
        match self {
            ClashType::Hard => 0,
            ClashType::Soft => 1,
            ClashType::Clearance => 2,
        }
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            ClashType::Hard => "Hard",
            ClashType::Soft => "Soft",
            ClashType::Clearance => "Clearance",
        }
    }
}

/// A single detected clash between two elements.
#[derive(Debug, Clone)]
pub struct ClashResult {
    /// Sequential clash ID.
    pub id: usize,
    /// Element A identifier.
    pub element_a_id: i32,
    /// Element A display name.
    pub element_a_name: String,
    /// Element A IFC type.
    pub element_a_type: String,
    /// Element B identifier.
    pub element_b_id: i32,
    /// Element B display name.
    pub element_b_name: String,
    /// Element B IFC type.
    pub element_b_type: String,
    /// Classification of the clash.
    pub clash_type: ClashType,
    /// Penetration depth (negative) or clearance distance (positive).
    pub distance: f32,
    /// Approximate point of clash (midpoint of overlapping region).
    pub clash_point: [f32; 3],
    /// Clash group (e.g., "Structural vs MEP").
    pub group_name: Option<String>,
}

/// Settings that control clash detection behaviour.
#[derive(Debug, Clone)]
pub struct ClashSettings {
    /// Any overlap deeper than this is a hard clash (default 0.0).
    pub hard_tolerance: f32,
    /// Overlap/separation within this is a soft clash (default 0.01 = 1 cm).
    pub soft_tolerance: f32,
    /// Separation within this is a clearance clash (default 0.05 = 5 cm).
    pub clearance_tolerance: f32,
    /// When false, skip pairs where both elements share the same type.
    pub check_same_type: bool,
    /// If non-empty, only check pairs matching one of these type pairs.
    pub type_pairs: Vec<(String, String)>,
}

impl Default for ClashSettings {
    fn default() -> Self {
        Self {
            hard_tolerance: 0.0,
            soft_tolerance: 0.01,
            clearance_tolerance: 0.05,
            check_same_type: false,
            type_pairs: Vec::new(),
        }
    }
}

/// Aggregated clash detection report.
#[derive(Debug, Clone)]
pub struct ClashReport {
    /// All detected clashes.
    pub clashes: Vec<ClashResult>,
    /// Number of hard clashes.
    pub hard_count: usize,
    /// Number of soft clashes.
    pub soft_count: usize,
    /// Number of clearance clashes.
    pub clearance_count: usize,
    /// Total elements considered.
    pub elements_checked: usize,
    /// Total element pairs tested.
    pub pairs_checked: usize,
}

// ============================================================================
// AABB Utilities
// ============================================================================

/// Check whether two axis-aligned bounding boxes overlap.
pub fn aabb_overlap(a: &BoundingBox, b: &BoundingBox) -> bool {
    a.min[0] <= b.max[0]
        && a.max[0] >= b.min[0]
        && a.min[1] <= b.max[1]
        && a.max[1] >= b.min[1]
        && a.min[2] <= b.max[2]
        && a.max[2] >= b.min[2]
}

/// Signed distance between two AABBs.
///
/// * Negative values indicate overlap depth.
/// * Positive values indicate separation distance.
pub fn aabb_distance(a: &BoundingBox, b: &BoundingBox) -> f32 {
    if aabb_overlap(a, b) {
        // Overlapping: compute minimum penetration depth (negative).
        let overlap_x = (a.max[0].min(b.max[0]) - a.min[0].max(b.min[0])).max(0.0);
        let overlap_y = (a.max[1].min(b.max[1]) - a.min[1].max(b.min[1])).max(0.0);
        let overlap_z = (a.max[2].min(b.max[2]) - a.min[2].max(b.min[2])).max(0.0);
        -overlap_x.min(overlap_y).min(overlap_z)
    } else {
        // Separated: compute minimum separation on any axis.
        let gap_x = if a.max[0] < b.min[0] {
            b.min[0] - a.max[0]
        } else if b.max[0] < a.min[0] {
            a.min[0] - b.max[0]
        } else {
            0.0
        };
        let gap_y = if a.max[1] < b.min[1] {
            b.min[1] - a.max[1]
        } else if b.max[1] < a.min[1] {
            a.min[1] - b.max[1]
        } else {
            0.0
        };
        let gap_z = if a.max[2] < b.min[2] {
            b.min[2] - a.max[2]
        } else if b.max[2] < a.min[2] {
            a.min[2] - b.max[2]
        } else {
            0.0
        };
        // The true gap is the maximum single-axis gap (all must be closed to overlap).
        gap_x.max(gap_y).max(gap_z)
    }
}

/// Check AABB overlap with an expansion tolerance applied to `b`.
pub fn aabb_overlap_expanded(a: &BoundingBox, b: &BoundingBox, tolerance: f32) -> bool {
    a.min[0] <= b.max[0] + tolerance
        && a.max[0] >= b.min[0] - tolerance
        && a.min[1] <= b.max[1] + tolerance
        && a.max[1] >= b.min[1] - tolerance
        && a.min[2] <= b.max[2] + tolerance
        && a.max[2] >= b.min[2] - tolerance
}

// ============================================================================
// Triangle-Triangle Intersection (Separating Axis Theorem)
// ============================================================================

/// Project a triangle onto an axis and return (min, max).
fn project_triangle(tri: &[[f32; 3]; 3], axis: [f32; 3]) -> (f32, f32) {
    let d0 = axis[0] * tri[0][0] + axis[1] * tri[0][1] + axis[2] * tri[0][2];
    let d1 = axis[0] * tri[1][0] + axis[1] * tri[1][1] + axis[2] * tri[1][2];
    let d2 = axis[0] * tri[2][0] + axis[1] * tri[2][1] + axis[2] * tri[2][2];
    (d0.min(d1).min(d2), d0.max(d1).max(d2))
}

/// Compute the cross product of two 3D vectors.
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Subtract two 3D vectors: a - b.
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Squared length of a vector.
fn len_sq(v: [f32; 3]) -> f32 {
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}

/// Moller's triangle-triangle intersection test using the Separating Axis Theorem.
///
/// Tests 13 potential separating axes:
/// - Normal of t1
/// - Normal of t2
/// - 9 cross products of edges (3 edges of t1 x 3 edges of t2)
pub fn triangle_triangle_intersect(t1: [[f32; 3]; 3], t2: [[f32; 3]; 3]) -> bool {
    // Edges of t1
    let e1 = [sub(t1[1], t1[0]), sub(t1[2], t1[1]), sub(t1[0], t1[2])];
    // Edges of t2
    let e2 = [sub(t2[1], t2[0]), sub(t2[2], t2[1]), sub(t2[0], t2[2])];

    // Normal of t1
    let n1 = cross(e1[0], e1[1]);
    // Normal of t2
    let n2 = cross(e2[0], e2[1]);

    // Helper: check a single separating axis.
    let separated = |axis: [f32; 3]| -> bool {
        // Skip degenerate (zero-length) axes.
        if len_sq(axis) < 1e-12 {
            return false; // not a valid separator
        }
        let (min1, max1) = project_triangle(&t1, axis);
        let (min2, max2) = project_triangle(&t2, axis);
        max1 < min2 || max2 < min1
    };

    // Test face normals
    if separated(n1) {
        return false;
    }
    if separated(n2) {
        return false;
    }

    // Test 9 edge-edge cross products
    for ea in &e1 {
        for eb in &e2 {
            if separated(cross(*ea, *eb)) {
                return false;
            }
        }
    }

    // No separating axis found => triangles intersect
    true
}

// ============================================================================
// Clash Detection
// ============================================================================

/// Determine the clash group name based on element types.
pub fn classify_clash_group(type_a: &str, type_b: &str) -> String {
    let structural_keywords = ["wall", "slab", "beam", "column", "footing", "foundation"];
    let mep_keywords = ["pipe", "duct", "terminal", "flowterminal"];

    let a_lower = type_a.to_lowercase();
    let b_lower = type_b.to_lowercase();

    let a_structural = structural_keywords.iter().any(|k| a_lower.contains(k));
    let b_structural = structural_keywords.iter().any(|k| b_lower.contains(k));
    let a_mep = mep_keywords.iter().any(|k| a_lower.contains(k));
    let b_mep = mep_keywords.iter().any(|k| b_lower.contains(k));

    if (a_structural && b_mep) || (a_mep && b_structural) {
        "Structural vs MEP".to_string()
    } else if a_structural && b_structural {
        "Structural vs Structural".to_string()
    } else if a_mep && b_mep {
        "MEP vs MEP".to_string()
    } else {
        "Generic".to_string()
    }
}

/// Compute the midpoint of the overlapping region between two AABBs.
fn overlap_midpoint(a: &BoundingBox, b: &BoundingBox) -> [f32; 3] {
    let ox_min = a.min[0].max(b.min[0]);
    let ox_max = a.max[0].min(b.max[0]);
    let oy_min = a.min[1].max(b.min[1]);
    let oy_max = a.max[1].min(b.max[1]);
    let oz_min = a.min[2].max(b.min[2]);
    let oz_max = a.max[2].min(b.max[2]);
    [
        (ox_min + ox_max) / 2.0,
        (oy_min + oy_max) / 2.0,
        (oz_min + oz_max) / 2.0,
    ]
}

/// Run clash detection on a list of elements.
///
/// Broad phase: AABB overlap with clearance tolerance expansion.
/// Classification: Hard / Soft / Clearance based on signed distance.
pub fn detect_clashes(elements: &[ElementInfo], settings: &ClashSettings) -> ClashReport {
    let mut clashes = Vec::new();
    let mut hard_count = 0usize;
    let mut soft_count = 0usize;
    let mut clearance_count = 0usize;
    let mut pairs_checked = 0usize;
    let mut clash_id = 0usize;

    let n = elements.len();
    for i in 0..n {
        for j in (i + 1)..n {
            let a = &elements[i];
            let b = &elements[j];

            // Skip self (shouldn't happen with i<j but be defensive)
            if a.id == b.id {
                continue;
            }

            // Skip same-type pairs if configured
            if !settings.check_same_type && a.element_type == b.element_type {
                continue;
            }

            // Skip if type_pairs filter is active and pair doesn't match
            if !settings.type_pairs.is_empty() {
                let matches = settings.type_pairs.iter().any(|(ta, tb)| {
                    (a.element_type.eq_ignore_ascii_case(ta)
                        && b.element_type.eq_ignore_ascii_case(tb))
                        || (a.element_type.eq_ignore_ascii_case(tb)
                            && b.element_type.eq_ignore_ascii_case(ta))
                });
                if !matches {
                    continue;
                }
            }

            pairs_checked += 1;

            // Broad phase: expanded AABB overlap
            if !aabb_overlap_expanded(&a.bounds, &b.bounds, settings.clearance_tolerance) {
                continue;
            }

            // Compute signed distance
            let dist = aabb_distance(&a.bounds, &b.bounds);

            // Classify clash type
            let clash_type = if dist < -settings.hard_tolerance {
                ClashType::Hard
            } else if dist < settings.soft_tolerance {
                ClashType::Soft
            } else {
                ClashType::Clearance
            };

            match clash_type {
                ClashType::Hard => hard_count += 1,
                ClashType::Soft => soft_count += 1,
                ClashType::Clearance => clearance_count += 1,
            }

            // Compute clash point (midpoint of overlapping / closest AABB region)
            let clash_point = if dist <= 0.0 {
                overlap_midpoint(&a.bounds, &b.bounds)
            } else {
                // For non-overlapping pairs, use midpoint between closest faces
                let ca = a.bounds.center();
                let cb = b.bounds.center();
                [
                    (ca[0] + cb[0]) / 2.0,
                    (ca[1] + cb[1]) / 2.0,
                    (ca[2] + cb[2]) / 2.0,
                ]
            };

            let group_name = Some(classify_clash_group(&a.element_type, &b.element_type));

            clashes.push(ClashResult {
                id: clash_id,
                element_a_id: a.id,
                element_a_name: a.name.clone(),
                element_a_type: a.element_type.clone(),
                element_b_id: b.id,
                element_b_name: b.name.clone(),
                element_b_type: b.element_type.clone(),
                clash_type,
                distance: dist,
                clash_point,
                group_name,
            });

            clash_id += 1;
        }
    }

    ClashReport {
        clashes,
        hard_count,
        soft_count,
        clearance_count,
        elements_checked: n,
        pairs_checked,
    }
}

// ============================================================================
// Export Utilities
// ============================================================================

/// Generate a CSV clash report.
pub fn export_clash_report_csv(report: &ClashReport) -> String {
    let mut csv =
        String::from("ID,Element A,Type A,Element B,Type B,Clash Type,Distance,X,Y,Z,Group\n");
    for c in &report.clashes {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{:.4},{:.4},{:.4},{:.4},{}\n",
            c.id,
            c.element_a_name,
            c.element_a_type,
            c.element_b_name,
            c.element_b_type,
            c.clash_type.label(),
            c.distance,
            c.clash_point[0],
            c.clash_point[1],
            c.clash_point[2],
            c.group_name.as_deref().unwrap_or(""),
        ));
    }
    csv
}

/// Generate an HTML clash report with a summary table.
pub fn export_clash_report_html(report: &ClashReport) -> String {
    let mut html = String::from(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Clash Detection Report</title>
<style>
  body { font-family: Arial, sans-serif; margin: 20px; }
  h1 { color: #333; }
  .summary { margin-bottom: 20px; }
  .summary span { font-weight: bold; }
  table { border-collapse: collapse; width: 100%; }
  th, td { border: 1px solid #ccc; padding: 6px 10px; text-align: left; }
  th { background: #f0f0f0; }
  tr:nth-child(even) { background: #fafafa; }
  .hard { color: #d32f2f; font-weight: bold; }
  .soft { color: #f57c00; }
  .clearance { color: #1976d2; }
</style>
</head>
<body>
<h1>Clash Detection Report</h1>
<div class="summary">
  <p>Elements checked: <span>"#,
    );
    html.push_str(&report.elements_checked.to_string());
    html.push_str("</span> | Pairs checked: <span>");
    html.push_str(&report.pairs_checked.to_string());
    html.push_str("</span></p>\n  <p>Total clashes: <span>");
    html.push_str(&report.clashes.len().to_string());
    html.push_str("</span> (Hard: <span class=\"hard\">");
    html.push_str(&report.hard_count.to_string());
    html.push_str("</span> | Soft: <span class=\"soft\">");
    html.push_str(&report.soft_count.to_string());
    html.push_str("</span> | Clearance: <span class=\"clearance\">");
    html.push_str(&report.clearance_count.to_string());
    html.push_str(
        r#"</span>)</p>
</div>
<table>
<tr>
  <th>ID</th><th>Element A</th><th>Type A</th>
  <th>Element B</th><th>Type B</th>
  <th>Clash Type</th><th>Distance</th>
  <th>X</th><th>Y</th><th>Z</th><th>Group</th>
</tr>
"#,
    );

    for c in &report.clashes {
        let class = match c.clash_type {
            ClashType::Hard => "hard",
            ClashType::Soft => "soft",
            ClashType::Clearance => "clearance",
        };
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>\
             <td class=\"{}\">{}</td><td>{:.4}</td>\
             <td>{:.4}</td><td>{:.4}</td><td>{:.4}</td><td>{}</td></tr>\n",
            c.id,
            c.element_a_name,
            c.element_a_type,
            c.element_b_name,
            c.element_b_type,
            class,
            c.clash_type.label(),
            c.distance,
            c.clash_point[0],
            c.clash_point[1],
            c.clash_point[2],
            c.group_name.as_deref().unwrap_or(""),
        ));
    }

    html.push_str("</table>\n</body>\n</html>\n");
    html
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bbox(min: [f32; 3], max: [f32; 3]) -> BoundingBox {
        BoundingBox { min, max }
    }

    #[test]
    fn test_aabb_overlap() {
        let a = make_bbox([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        let b = make_bbox([1.0, 1.0, 1.0], [3.0, 3.0, 3.0]);
        assert!(aabb_overlap(&a, &b));
    }

    #[test]
    fn test_aabb_no_overlap() {
        let a = make_bbox([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let b = make_bbox([2.0, 2.0, 2.0], [3.0, 3.0, 3.0]);
        assert!(!aabb_overlap(&a, &b));
    }

    #[test]
    fn test_aabb_distance_positive() {
        let a = make_bbox([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let b = make_bbox([3.0, 0.0, 0.0], [4.0, 1.0, 1.0]);
        let dist = aabb_distance(&a, &b);
        assert!(dist > 0.0, "Expected positive distance, got {}", dist);
        assert!(
            (dist - 2.0).abs() < 1e-5,
            "Expected distance ~2.0, got {}",
            dist
        );
    }

    #[test]
    fn test_aabb_distance_negative() {
        let a = make_bbox([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        let b = make_bbox([1.0, 1.0, 1.0], [3.0, 3.0, 3.0]);
        let dist = aabb_distance(&a, &b);
        assert!(
            dist < 0.0,
            "Expected negative distance (overlap), got {}",
            dist
        );
        // Overlap on each axis is 1.0, minimum overlap = 1.0
        assert!(
            (dist - (-1.0)).abs() < 1e-5,
            "Expected distance ~-1.0, got {}",
            dist
        );
    }

    #[test]
    fn test_triangle_triangle_intersect() {
        // Two triangles that cross each other in the XY plane
        let t1 = [[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let t2 = [[0.0, 0.5, -1.0], [0.0, 0.5, 1.0], [0.0, -0.5, 0.0]];
        assert!(triangle_triangle_intersect(t1, t2));
    }

    #[test]
    fn test_triangle_triangle_no_intersect() {
        // Two triangles that are far apart
        let t1 = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let t2 = [[5.0, 5.0, 5.0], [6.0, 5.0, 5.0], [5.0, 6.0, 5.0]];
        assert!(!triangle_triangle_intersect(t1, t2));
    }
}
