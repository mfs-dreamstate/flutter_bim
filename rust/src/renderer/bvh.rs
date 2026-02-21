//! Bounding Volume Hierarchy for O(log n) ray picking.

use super::camera::ray_aabb_intersect;
use glam::Vec3;

/// BVH node — either a leaf (single element) or internal (two children).
pub enum BvhNode {
    Leaf {
        min: [f32; 3],
        max: [f32; 3],
        element_index: usize,
    },
    Internal {
        min: [f32; 3],
        max: [f32; 3],
        left: Box<BvhNode>,
        right: Box<BvhNode>,
    },
}

impl BvhNode {
    /// Build a BVH from a list of (index, aabb_min, aabb_max).
    /// Uses median-split along the longest axis for balanced trees.
    pub fn build(elements: &mut [(usize, [f32; 3], [f32; 3])]) -> Option<Box<BvhNode>> {
        if elements.is_empty() {
            return None;
        }
        if elements.len() == 1 {
            return Some(Box::new(BvhNode::Leaf {
                min: elements[0].1,
                max: elements[0].2,
                element_index: elements[0].0,
            }));
        }

        // Compute bounding box of all elements
        let mut total_min = [f32::MAX; 3];
        let mut total_max = [f32::MIN; 3];
        for (_, emin, emax) in elements.iter() {
            for i in 0..3 {
                total_min[i] = total_min[i].min(emin[i]);
                total_max[i] = total_max[i].max(emax[i]);
            }
        }

        // Find longest axis
        let extent = [
            total_max[0] - total_min[0],
            total_max[1] - total_min[1],
            total_max[2] - total_min[2],
        ];
        let axis = if extent[0] >= extent[1] && extent[0] >= extent[2] {
            0
        } else if extent[1] >= extent[2] {
            1
        } else {
            2
        };

        // Sort by center along longest axis
        elements.sort_unstable_by(|a, b| {
            let ca = (a.1[axis] + a.2[axis]) * 0.5;
            let cb = (b.1[axis] + b.2[axis]) * 0.5;
            ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Split at median
        let mid = elements.len() / 2;
        let (left_slice, right_slice) = elements.split_at_mut(mid);

        let left = Self::build(left_slice);
        let right = Self::build(right_slice);

        match (left, right) {
            (Some(l), Some(r)) => Some(Box::new(BvhNode::Internal {
                min: total_min,
                max: total_max,
                left: l,
                right: r,
            })),
            (Some(node), None) | (None, Some(node)) => Some(node),
            (None, None) => None,
        }
    }

    /// Query the BVH with a ray. Returns the index of the closest hit element.
    /// `closest_t` is updated to track the nearest intersection distance.
    pub fn ray_query(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        closest_t: &mut f32,
    ) -> Option<usize> {
        match self {
            BvhNode::Leaf {
                min,
                max,
                element_index,
            } => {
                if let Some(t) = ray_aabb_intersect(
                    ray_origin,
                    ray_dir,
                    Vec3::from_array(*min),
                    Vec3::from_array(*max),
                ) {
                    if t < *closest_t {
                        *closest_t = t;
                        return Some(*element_index);
                    }
                }
                None
            }
            BvhNode::Internal {
                min,
                max,
                left,
                right,
            } => {
                // Early rejection: skip subtree if ray misses parent AABB
                match ray_aabb_intersect(
                    ray_origin,
                    ray_dir,
                    Vec3::from_array(*min),
                    Vec3::from_array(*max),
                ) {
                    None => return None,
                    Some(t) if t > *closest_t => return None, // Already found something closer
                    _ => {}
                }

                let mut result = None;
                if let Some(idx) = left.ray_query(ray_origin, ray_dir, closest_t) {
                    result = Some(idx);
                }
                if let Some(idx) = right.ray_query(ray_origin, ray_dir, closest_t) {
                    result = Some(idx);
                }
                result
            }
        }
    }
}
