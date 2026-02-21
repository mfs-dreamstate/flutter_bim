//! Polygon Triangulation
//!
//! Converts N-gon polygons into triangles for BREP faces and extrusion caps.
//! Uses ear-clipping algorithm for general polygons.

use glam::Vec3;

/// Triangulate a polygon defined by 3D points and a face normal.
/// Returns index triples into the original `points` slice.
pub fn triangulate_polygon(points: &[Vec3], normal: Vec3) -> Vec<[usize; 3]> {
    let n = points.len();
    if n < 3 {
        return Vec::new();
    }
    if n == 3 {
        return vec![[0, 1, 2]];
    }
    if n == 4 {
        return triangulate_quad(points);
    }
    // General case: project to 2D and ear-clip
    ear_clip_3d(points, normal)
}

/// Triangulate a quad by splitting on the shorter diagonal.
fn triangulate_quad(points: &[Vec3]) -> Vec<[usize; 3]> {
    let d02 = (points[0] - points[2]).length_squared();
    let d13 = (points[1] - points[3]).length_squared();
    if d02 <= d13 {
        vec![[0, 1, 2], [0, 2, 3]]
    } else {
        vec![[0, 1, 3], [1, 2, 3]]
    }
}

/// Project 3D polygon to 2D along the dominant axis of the normal, then ear-clip.
fn ear_clip_3d(points: &[Vec3], normal: Vec3) -> Vec<[usize; 3]> {
    // Choose projection plane based on dominant normal axis
    let abs_n = normal.abs();
    let (u_axis, v_axis) = if abs_n.x >= abs_n.y && abs_n.x >= abs_n.z {
        // X dominant -> project onto YZ
        (1usize, 2usize)
    } else if abs_n.y >= abs_n.x && abs_n.y >= abs_n.z {
        // Y dominant -> project onto XZ
        (0, 2)
    } else {
        // Z dominant -> project onto XY
        (0, 1)
    };

    let polygon_2d: Vec<[f32; 2]> = points
        .iter()
        .map(|p| {
            let arr = p.to_array();
            [arr[u_axis], arr[v_axis]]
        })
        .collect();

    // Ensure the winding is counter-clockwise in the projected plane
    let area = signed_area_2d(&polygon_2d);
    let ccw = if area < 0.0 {
        // Clockwise — we need to reverse the index mapping
        true // we'll flip at the end
    } else {
        false
    };

    let mut indices: Vec<usize> = (0..points.len()).collect();
    if ccw && area < 0.0 {
        indices.reverse();
    }

    let result = ear_clip(&polygon_2d, &mut indices);

    // If ear-clipping produced no triangles (degenerate), fall back to fan
    if result.is_empty() && points.len() >= 3 {
        return fan_triangulate(points.len());
    }

    result
}

/// Signed area of a 2D polygon (positive = CCW, negative = CW).
fn signed_area_2d(polygon: &[[f32; 2]]) -> f32 {
    let n = polygon.len();
    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += polygon[i][0] * polygon[j][1];
        area -= polygon[j][0] * polygon[i][1];
    }
    area * 0.5
}

/// Ear-clipping triangulation on a 2D polygon.
/// `indices` maps polygon vertex positions to original 3D point indices.
/// O(n^2) — fine for typical BIM faces (3-20 vertices).
fn ear_clip(polygon: &[[f32; 2]], indices: &mut Vec<usize>) -> Vec<[usize; 3]> {
    let mut result = Vec::new();
    let mut remaining: Vec<usize> = (0..polygon.len()).collect();

    let mut max_iterations = polygon.len() * polygon.len();

    while remaining.len() > 3 && max_iterations > 0 {
        max_iterations -= 1;
        let n = remaining.len();
        let mut found_ear = false;

        for i in 0..n {
            let prev = remaining[(i + n - 1) % n];
            let curr = remaining[i];
            let next = remaining[(i + 1) % n];

            let a = polygon[prev];
            let b = polygon[curr];
            let c = polygon[next];

            // Check if this is a convex vertex (left turn)
            if cross_2d(a, b, c) <= 0.0 {
                continue;
            }

            // Check if any other vertex is inside this triangle
            let mut ear = true;
            for j in 0..n {
                let idx = remaining[j];
                if idx == prev || idx == curr || idx == next {
                    continue;
                }
                if point_in_triangle_2d(polygon[idx], a, b, c) {
                    ear = false;
                    break;
                }
            }

            if ear {
                result.push([indices[prev], indices[curr], indices[next]]);
                remaining.remove(i);
                found_ear = true;
                break;
            }
        }

        if !found_ear {
            // No ear found — degenerate polygon, break
            break;
        }
    }

    // Handle remaining triangle
    if remaining.len() == 3 {
        result.push([
            indices[remaining[0]],
            indices[remaining[1]],
            indices[remaining[2]],
        ]);
    }

    result
}

/// 2D cross product of vectors (b-a) x (c-a).
/// Positive = left turn (CCW), negative = right turn (CW).
fn cross_2d(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// Check if point p is inside triangle (a, b, c) using barycentric coordinates.
fn point_in_triangle_2d(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let d1 = cross_2d(a, b, p);
    let d2 = cross_2d(b, c, p);
    let d3 = cross_2d(c, a, p);

    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);

    !(has_neg && has_pos)
}

/// Fan triangulation as fallback for degenerate polygons.
fn fan_triangulate(n: usize) -> Vec<[usize; 3]> {
    let mut result = Vec::with_capacity(n - 2);
    for i in 1..n - 1 {
        result.push([0, i, i + 1]);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triangle_passthrough() {
        let pts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let tris = triangulate_polygon(&pts, Vec3::Z);
        assert_eq!(tris.len(), 1);
        assert_eq!(tris[0], [0, 1, 2]);
    }

    #[test]
    fn test_quad_triangulation() {
        let pts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let tris = triangulate_polygon(&pts, Vec3::Z);
        assert_eq!(tris.len(), 2);
    }

    #[test]
    fn test_convex_pentagon() {
        let pts: Vec<Vec3> = (0..5)
            .map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / 5.0;
                Vec3::new(angle.cos(), angle.sin(), 0.0)
            })
            .collect();
        let tris = triangulate_polygon(&pts, Vec3::Z);
        assert_eq!(tris.len(), 3); // 5 vertices -> 3 triangles
    }

    #[test]
    fn test_concave_l_shape() {
        // L-shaped polygon (concave)
        let pts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
        ];
        let tris = triangulate_polygon(&pts, Vec3::Z);
        assert_eq!(tris.len(), 4); // 6 vertices -> 4 triangles
    }

    #[test]
    fn test_degenerate_less_than_3() {
        let pts = vec![Vec3::ZERO, Vec3::X];
        let tris = triangulate_polygon(&pts, Vec3::Z);
        assert!(tris.is_empty());
    }
}
