//! Bounding Volume Hierarchy for O(log n) ray picking,
//! spatial hash for broad-phase queries, occlusion culling,
//! and distance-based LOD classification.

use super::camera::ray_aabb_intersect;
use glam::Vec3;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Helper: AABB utilities
// ---------------------------------------------------------------------------

/// Compute the union of two AABBs.
fn aabb_union(
    a_min: [f32; 3],
    a_max: [f32; 3],
    b_min: [f32; 3],
    b_max: [f32; 3],
) -> ([f32; 3], [f32; 3]) {
    let mut out_min = [0.0f32; 3];
    let mut out_max = [0.0f32; 3];
    for i in 0..3 {
        out_min[i] = a_min[i].min(b_min[i]);
        out_max[i] = a_max[i].max(b_max[i]);
    }
    (out_min, out_max)
}

/// Surface area of an AABB (used for SAH-like cost heuristics).
fn aabb_surface_area(min: [f32; 3], max: [f32; 3]) -> f32 {
    let dx = (max[0] - min[0]).max(0.0);
    let dy = (max[1] - min[1]).max(0.0);
    let dz = (max[2] - min[2]).max(0.0);
    2.0 * (dx * dy + dy * dz + dz * dx)
}

/// Compute the center of an AABB.
fn aabb_center(min: [f32; 3], max: [f32; 3]) -> [f32; 3] {
    [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ]
}

/// Squared distance from a point to the nearest point on an AABB.
fn point_aabb_distance_sq(point: [f32; 3], min: [f32; 3], max: [f32; 3]) -> f32 {
    let mut dist_sq = 0.0f32;
    for i in 0..3 {
        if point[i] < min[i] {
            let d = min[i] - point[i];
            dist_sq += d * d;
        } else if point[i] > max[i] {
            let d = point[i] - max[i];
            dist_sq += d * d;
        }
    }
    dist_sq
}

/// Check if two AABBs overlap.
fn aabb_overlap(a_min: [f32; 3], a_max: [f32; 3], b_min: [f32; 3], b_max: [f32; 3]) -> bool {
    for i in 0..3 {
        if a_max[i] < b_min[i] || b_max[i] < a_min[i] {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Task 1: BVH Node with incremental updates
// ---------------------------------------------------------------------------

/// Internal result type for the recursive remove operation.
enum RemoveResult {
    /// Element was not found in this subtree.
    NotFound,
    /// Element was found and the current node should be replaced by this node.
    Collapse(Box<BvhNode>),
    /// Element was found and removed; the subtree was patched in place.
    RemovedInPlace,
}

/// BVH node -- either a leaf (single element) or internal (two children).
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

    // -- Accessor helpers ---------------------------------------------------

    /// Return the AABB of this node.
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        match self {
            BvhNode::Leaf { min, max, .. } => (*min, *max),
            BvhNode::Internal { min, max, .. } => (*min, *max),
        }
    }

    // -- Incremental update methods -----------------------------------------

    /// Insert a new element into the BVH without full rebuild.
    /// Finds the best leaf to split and creates a new internal node.
    pub fn insert(&mut self, element_index: usize, min: [f32; 3], max: [f32; 3]) {
        // Strategy: find the leaf whose AABB would grow the least (by surface
        // area increase) when combined with the new element. Then split that
        // leaf into an internal node containing the old leaf and a new leaf.
        self.insert_recursive(element_index, min, max);
    }

    /// Recursive helper: walks the tree picking the cheaper child to descend
    /// into, and splits a leaf when it reaches one.
    fn insert_recursive(&mut self, element_index: usize, new_min: [f32; 3], new_max: [f32; 3]) {
        match self {
            BvhNode::Leaf {
                min,
                max,
                element_index: existing_idx,
            } => {
                // Replace this leaf with an internal node containing both the
                // old element and the new element.
                let old_leaf = BvhNode::Leaf {
                    min: *min,
                    max: *max,
                    element_index: *existing_idx,
                };
                let new_leaf = BvhNode::Leaf {
                    min: new_min,
                    max: new_max,
                    element_index,
                };
                let (combined_min, combined_max) = aabb_union(*min, *max, new_min, new_max);
                *self = BvhNode::Internal {
                    min: combined_min,
                    max: combined_max,
                    left: Box::new(old_leaf),
                    right: Box::new(new_leaf),
                };
            }
            BvhNode::Internal {
                min,
                max,
                left,
                right,
            } => {
                // Expand this node's AABB to include the new element.
                for i in 0..3 {
                    min[i] = min[i].min(new_min[i]);
                    max[i] = max[i].max(new_max[i]);
                }

                // Choose the child that results in the least surface area
                // increase.
                let (l_min, l_max) = left.bounds();
                let (r_min, r_max) = right.bounds();

                let (merged_l_min, merged_l_max) = aabb_union(l_min, l_max, new_min, new_max);
                let (merged_r_min, merged_r_max) = aabb_union(r_min, r_max, new_min, new_max);

                let cost_left =
                    aabb_surface_area(merged_l_min, merged_l_max) - aabb_surface_area(l_min, l_max);
                let cost_right = aabb_surface_area(merged_r_min, merged_r_max)
                    - aabb_surface_area(r_min, r_max);

                if cost_left <= cost_right {
                    left.insert_recursive(element_index, new_min, new_max);
                } else {
                    right.insert_recursive(element_index, new_min, new_max);
                }
            }
        }
    }

    /// Remove an element from the BVH by index.
    /// Returns `true` if the element was found and removed.
    ///
    /// Because removing an element can collapse a subtree, this method
    /// returns the replacement subtree via an internal helper and then
    /// patches `self` in place.
    pub fn remove(&mut self, element_index: usize) -> bool {
        // Handle the degenerate case where *this* node is the target leaf.
        if let BvhNode::Leaf {
            element_index: idx, ..
        } = self
        {
            if *idx == element_index {
                // Cannot truly remove the root leaf in-place (the caller
                // would need to discard the whole tree). We set it to an
                // empty sentinel with element_index = usize::MAX.
                *self = BvhNode::Leaf {
                    min: [0.0; 3],
                    max: [0.0; 3],
                    element_index: usize::MAX,
                };
                return true;
            }
            return false;
        }

        // For internal nodes use the recursive helper.
        match self.remove_recursive(element_index) {
            RemoveResult::NotFound => false,
            RemoveResult::Collapse(replacement) => {
                *self = *replacement;
                true
            }
            RemoveResult::RemovedInPlace => true,
        }
    }

    /// Recursive remove helper.
    ///
    /// Returns `Collapse(replacement)` when the *current* node should be
    /// replaced by `replacement` in its parent (e.g. when a direct child
    /// was the removed leaf and this internal node collapses to its sibling).
    ///
    /// Returns `RemovedInPlace` when the element was found deeper in the
    /// tree and the subtree was patched without needing to collapse this node.
    ///
    /// Returns `NotFound` when the element does not exist in this subtree.
    fn remove_recursive(&mut self, element_index: usize) -> RemoveResult {
        match self {
            BvhNode::Leaf { .. } => {
                // Leaves are handled by the parent's internal-node branch.
                RemoveResult::NotFound
            }
            BvhNode::Internal {
                min: _,
                max: _,
                left,
                right,
            } => {
                // Check if left child is the target leaf.
                if let BvhNode::Leaf {
                    element_index: idx, ..
                } = left.as_ref()
                {
                    if *idx == element_index {
                        // Remove left child -> collapse to right child.
                        let dummy = BvhNode::Leaf {
                            min: [0.0; 3],
                            max: [0.0; 3],
                            element_index: usize::MAX,
                        };
                        let right_owned =
                            std::mem::replace(right, Box::new(dummy));
                        return RemoveResult::Collapse(right_owned);
                    }
                }

                // Check if right child is the target leaf.
                if let BvhNode::Leaf {
                    element_index: idx, ..
                } = right.as_ref()
                {
                    if *idx == element_index {
                        let dummy = BvhNode::Leaf {
                            min: [0.0; 3],
                            max: [0.0; 3],
                            element_index: usize::MAX,
                        };
                        let left_owned =
                            std::mem::replace(left, Box::new(dummy));
                        return RemoveResult::Collapse(left_owned);
                    }
                }

                // Recurse into left subtree.
                match left.remove_recursive(element_index) {
                    RemoveResult::Collapse(replacement) => {
                        *left = replacement;
                        self.refit_local();
                        return RemoveResult::RemovedInPlace;
                    }
                    RemoveResult::RemovedInPlace => {
                        self.refit_local();
                        return RemoveResult::RemovedInPlace;
                    }
                    RemoveResult::NotFound => {}
                }

                // Recurse into right subtree.
                match right.remove_recursive(element_index) {
                    RemoveResult::Collapse(replacement) => {
                        *right = replacement;
                        self.refit_local();
                        return RemoveResult::RemovedInPlace;
                    }
                    RemoveResult::RemovedInPlace => {
                        self.refit_local();
                        return RemoveResult::RemovedInPlace;
                    }
                    RemoveResult::NotFound => {}
                }

                RemoveResult::NotFound
            }
        }
    }

    /// Update an element's bounding box (remove + re-insert).
    /// Returns `true` if the element was found and updated.
    pub fn update(
        &mut self,
        element_index: usize,
        new_min: [f32; 3],
        new_max: [f32; 3],
    ) -> bool {
        if self.remove(element_index) {
            self.insert(element_index, new_min, new_max);
            true
        } else {
            false
        }
    }

    /// Refit: update internal node bounds to tightly contain children.
    /// Call after multiple inserts/removes for better tree quality.
    pub fn refit(&mut self) {
        match self {
            BvhNode::Leaf { .. } => { /* nothing to do */ }
            BvhNode::Internal {
                min,
                max,
                left,
                right,
            } => {
                left.refit();
                right.refit();

                let (l_min, l_max) = left.bounds();
                let (r_min, r_max) = right.bounds();
                let (new_min, new_max) = aabb_union(l_min, l_max, r_min, r_max);
                *min = new_min;
                *max = new_max;
            }
        }
    }

    /// Refit just this node (not recursive into children).
    fn refit_local(&mut self) {
        if let BvhNode::Internal {
            min,
            max,
            left,
            right,
        } = self
        {
            let (l_min, l_max) = left.bounds();
            let (r_min, r_max) = right.bounds();
            let (new_min, new_max) = aabb_union(l_min, l_max, r_min, r_max);
            *min = new_min;
            *max = new_max;
        }
    }

    /// Count the number of leaf nodes.
    pub fn leaf_count(&self) -> usize {
        match self {
            BvhNode::Leaf { .. } => 1,
            BvhNode::Internal { left, right, .. } => left.leaf_count() + right.leaf_count(),
        }
    }

    /// Count the total number of nodes (internal + leaf).
    pub fn node_count(&self) -> usize {
        match self {
            BvhNode::Leaf { .. } => 1,
            BvhNode::Internal { left, right, .. } => {
                1 + left.node_count() + right.node_count()
            }
        }
    }

    /// Get the depth of the tree.
    pub fn depth(&self) -> usize {
        match self {
            BvhNode::Leaf { .. } => 1,
            BvhNode::Internal { left, right, .. } => {
                1 + left.depth().max(right.depth())
            }
        }
    }

    /// Check if the tree contains an element with the given index.
    pub fn contains(&self, element_index: usize) -> bool {
        match self {
            BvhNode::Leaf {
                element_index: idx, ..
            } => *idx == element_index,
            BvhNode::Internal { left, right, .. } => {
                left.contains(element_index) || right.contains(element_index)
            }
        }
    }

    /// Frustum cull: collect all element indices whose AABB overlaps the query AABB.
    pub fn query_aabb(&self, query_min: [f32; 3], query_max: [f32; 3]) -> Vec<usize> {
        let mut results = Vec::new();
        self.query_aabb_recursive(query_min, query_max, &mut results);
        results
    }

    fn query_aabb_recursive(
        &self,
        query_min: [f32; 3],
        query_max: [f32; 3],
        results: &mut Vec<usize>,
    ) {
        let (node_min, node_max) = self.bounds();
        if !aabb_overlap(node_min, node_max, query_min, query_max) {
            return;
        }
        match self {
            BvhNode::Leaf { element_index, .. } => {
                results.push(*element_index);
            }
            BvhNode::Internal { left, right, .. } => {
                left.query_aabb_recursive(query_min, query_max, results);
                right.query_aabb_recursive(query_min, query_max, results);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Task 2: Spatial Hash Grid
// ---------------------------------------------------------------------------

/// Uniform grid spatial hash for fast neighbor/range queries.
pub struct SpatialHash {
    cell_size: f32,
    /// cell coordinate -> element indices in that cell
    cells: HashMap<(i32, i32, i32), Vec<usize>>,
    /// element index -> list of cells it occupies
    element_cells: HashMap<usize, Vec<(i32, i32, i32)>>,
    /// element index -> its AABB (for sphere/nearest queries)
    element_bounds: HashMap<usize, ([f32; 3], [f32; 3])>,
}

impl SpatialHash {
    /// Create an empty spatial hash with the given cell size.
    pub fn new(cell_size: f32) -> Self {
        assert!(cell_size > 0.0, "cell_size must be positive");
        Self {
            cell_size,
            cells: HashMap::new(),
            element_cells: HashMap::new(),
            element_bounds: HashMap::new(),
        }
    }

    /// Build from a list of (index, aabb_min, aabb_max).
    pub fn build(elements: &[(usize, [f32; 3], [f32; 3])], cell_size: f32) -> Self {
        let mut hash = Self::new(cell_size);
        for &(idx, min, max) in elements {
            hash.insert(idx, min, max);
        }
        hash
    }

    /// Auto-determine optimal cell size based on average element size.
    /// Uses 2x the average element diagonal as the cell size, which is a
    /// common heuristic that balances cell occupancy vs. number of cells.
    pub fn build_auto(elements: &[(usize, [f32; 3], [f32; 3])]) -> Self {
        if elements.is_empty() {
            return Self::new(1.0);
        }

        let mut total_diag = 0.0f64;
        for &(_, min, max) in elements {
            let dx = (max[0] - min[0]) as f64;
            let dy = (max[1] - min[1]) as f64;
            let dz = (max[2] - min[2]) as f64;
            total_diag += (dx * dx + dy * dy + dz * dz).sqrt();
        }
        let avg_diag = total_diag / elements.len() as f64;
        let cell_size = (avg_diag * 2.0).max(0.001) as f32;

        Self::build(elements, cell_size)
    }

    /// Convert a world-space coordinate to a cell coordinate.
    fn cell_coord(&self, x: f32, y: f32, z: f32) -> (i32, i32, i32) {
        (
            (x / self.cell_size).floor() as i32,
            (y / self.cell_size).floor() as i32,
            (z / self.cell_size).floor() as i32,
        )
    }

    /// Insert an element into the spatial hash.
    pub fn insert(&mut self, element_index: usize, min: [f32; 3], max: [f32; 3]) {
        let (cx0, cy0, cz0) = self.cell_coord(min[0], min[1], min[2]);
        let (cx1, cy1, cz1) = self.cell_coord(max[0], max[1], max[2]);

        let mut occupied_cells = Vec::new();
        for cx in cx0..=cx1 {
            for cy in cy0..=cy1 {
                for cz in cz0..=cz1 {
                    let key = (cx, cy, cz);
                    self.cells
                        .entry(key)
                        .or_insert_with(Vec::new)
                        .push(element_index);
                    occupied_cells.push(key);
                }
            }
        }

        self.element_cells.insert(element_index, occupied_cells);
        self.element_bounds.insert(element_index, (min, max));
    }

    /// Remove an element from the spatial hash.
    pub fn remove(&mut self, element_index: usize) {
        if let Some(cells) = self.element_cells.remove(&element_index) {
            for key in cells {
                if let Some(cell_elements) = self.cells.get_mut(&key) {
                    cell_elements.retain(|&idx| idx != element_index);
                    if cell_elements.is_empty() {
                        self.cells.remove(&key);
                    }
                }
            }
        }
        self.element_bounds.remove(&element_index);
    }

    /// Find all elements whose AABBs overlap with the given query AABB.
    pub fn query_aabb(&self, min: [f32; 3], max: [f32; 3]) -> Vec<usize> {
        let (cx0, cy0, cz0) = self.cell_coord(min[0], min[1], min[2]);
        let (cx1, cy1, cz1) = self.cell_coord(max[0], max[1], max[2]);

        let mut seen = HashMap::new();
        let mut results = Vec::new();

        for cx in cx0..=cx1 {
            for cy in cy0..=cy1 {
                for cz in cz0..=cz1 {
                    if let Some(cell_elements) = self.cells.get(&(cx, cy, cz)) {
                        for &idx in cell_elements {
                            if seen.contains_key(&idx) {
                                continue;
                            }
                            seen.insert(idx, ());
                            // Check actual AABB overlap.
                            if let Some(&(e_min, e_max)) = self.element_bounds.get(&idx) {
                                if aabb_overlap(e_min, e_max, min, max) {
                                    results.push(idx);
                                }
                            }
                        }
                    }
                }
            }
        }

        results
    }

    /// Find all elements within a radius of a point.
    pub fn query_sphere(&self, center: [f32; 3], radius: f32) -> Vec<usize> {
        // Build an AABB around the sphere and query cells.
        let sphere_min = [
            center[0] - radius,
            center[1] - radius,
            center[2] - radius,
        ];
        let sphere_max = [
            center[0] + radius,
            center[1] + radius,
            center[2] + radius,
        ];

        let (cx0, cy0, cz0) = self.cell_coord(sphere_min[0], sphere_min[1], sphere_min[2]);
        let (cx1, cy1, cz1) = self.cell_coord(sphere_max[0], sphere_max[1], sphere_max[2]);

        let radius_sq = radius * radius;
        let mut seen = HashMap::new();
        let mut results = Vec::new();

        for cx in cx0..=cx1 {
            for cy in cy0..=cy1 {
                for cz in cz0..=cz1 {
                    if let Some(cell_elements) = self.cells.get(&(cx, cy, cz)) {
                        for &idx in cell_elements {
                            if seen.contains_key(&idx) {
                                continue;
                            }
                            seen.insert(idx, ());
                            if let Some(&(e_min, e_max)) = self.element_bounds.get(&idx) {
                                // Check if the closest point on the AABB to
                                // the sphere center is within radius.
                                let dist_sq = point_aabb_distance_sq(center, e_min, e_max);
                                if dist_sq <= radius_sq {
                                    results.push(idx);
                                }
                            }
                        }
                    }
                }
            }
        }

        results
    }

    /// Find the K nearest elements to a point (approximate).
    ///
    /// Uses an expanding search: starts with the cell containing the point
    /// and expands outward until at least `k` candidates are found, then
    /// returns the closest `k` by distance to the element AABB center.
    pub fn query_nearest(&self, point: [f32; 3], k: usize) -> Vec<usize> {
        if k == 0 || self.element_bounds.is_empty() {
            return Vec::new();
        }

        let (px, py, pz) = self.cell_coord(point[0], point[1], point[2]);

        let mut candidates: HashMap<usize, f32> = HashMap::new();
        let mut shell = 0i32;

        // Expand shells until we have enough candidates.
        loop {
            let found_any = self.collect_shell(px, py, pz, shell, point, &mut candidates);

            if candidates.len() >= k && shell > 0 {
                // We have enough candidates and have searched at least one
                // extra shell beyond the first hit.
                break;
            }

            if !found_any && shell > 0 && candidates.len() >= k {
                break;
            }

            shell += 1;

            // Safety: stop if we've searched an unreasonably large area.
            if shell > 1000 {
                break;
            }

            // If we have candidates and the minimum possible distance for
            // elements in the next shell is greater than the k-th closest
            // distance we have, we can stop.
            if candidates.len() >= k {
                let mut dists: Vec<f32> = candidates.values().cloned().collect();
                dists.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let kth_dist = dists[k - 1];
                let min_shell_dist = (shell as f32 - 1.0) * self.cell_size;
                if min_shell_dist * min_shell_dist > kth_dist {
                    break;
                }
            }
        }

        // Sort by distance and take the closest k.
        let mut result: Vec<(usize, f32)> = candidates.into_iter().collect();
        result.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        result.truncate(k);
        result.into_iter().map(|(idx, _)| idx).collect()
    }

    /// Collect all elements in cells at the given shell distance from (px,py,pz).
    fn collect_shell(
        &self,
        px: i32,
        py: i32,
        pz: i32,
        shell: i32,
        point: [f32; 3],
        candidates: &mut HashMap<usize, f32>,
    ) -> bool {
        let mut found = false;

        for cx in (px - shell)..=(px + shell) {
            for cy in (py - shell)..=(py + shell) {
                for cz in (pz - shell)..=(pz + shell) {
                    // Only process cells on the surface of the shell cube.
                    if shell > 0 {
                        let on_surface = cx == px - shell
                            || cx == px + shell
                            || cy == py - shell
                            || cy == py + shell
                            || cz == pz - shell
                            || cz == pz + shell;
                        if !on_surface {
                            continue;
                        }
                    }

                    if let Some(cell_elements) = self.cells.get(&(cx, cy, cz)) {
                        for &idx in cell_elements {
                            if candidates.contains_key(&idx) {
                                continue;
                            }
                            if let Some(&(e_min, e_max)) = self.element_bounds.get(&idx) {
                                let dist_sq = point_aabb_distance_sq(point, e_min, e_max);
                                candidates.insert(idx, dist_sq);
                                found = true;
                            }
                        }
                    }
                }
            }
        }

        found
    }

    /// Get the number of elements in the hash.
    pub fn len(&self) -> usize {
        self.element_cells.len()
    }

    /// Check if the hash is empty.
    pub fn is_empty(&self) -> bool {
        self.element_cells.is_empty()
    }

    /// Get the number of occupied cells.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Average number of element entries per occupied cell.
    pub fn avg_elements_per_cell(&self) -> f32 {
        if self.cells.is_empty() {
            return 0.0;
        }
        let total: usize = self.cells.values().map(|v| v.len()).sum();
        total as f32 / self.cells.len() as f32
    }
}

// ---------------------------------------------------------------------------
// Task 3: Hierarchical Z-Buffer for Software Occlusion Culling
// ---------------------------------------------------------------------------

/// Hierarchical Z-buffer for software occlusion culling.
/// Uses a low-resolution depth buffer to quickly reject hidden elements.
pub struct HierarchicalZBuffer {
    width: usize,
    height: usize,
    /// Finest level depth values (row-major, width * height).
    depth: Vec<f32>,
    /// Coarser mip levels, each half the resolution of the previous.
    mip_levels: Vec<Vec<f32>>,
}

impl HierarchicalZBuffer {
    /// Create a new HZ-buffer with the given resolution.
    pub fn new(width: usize, height: usize) -> Self {
        assert!(width > 0 && height > 0, "dimensions must be positive");
        let depth = vec![f32::MAX; width * height];
        Self {
            width,
            height,
            depth,
            mip_levels: Vec::new(),
        }
    }

    /// Clear all depth values to far plane (f32::MAX).
    pub fn clear(&mut self) {
        for d in self.depth.iter_mut() {
            *d = f32::MAX;
        }
        for level in self.mip_levels.iter_mut() {
            for d in level.iter_mut() {
                *d = f32::MAX;
            }
        }
    }

    /// Rasterize an AABB to the depth buffer (conservative).
    /// `screen_min` and `screen_max` are in pixel coordinates [0..width, 0..height].
    /// `depth` is the near depth of the AABB (smaller = closer to camera).
    pub fn rasterize_aabb(&mut self, screen_min: [f32; 2], screen_max: [f32; 2], depth_val: f32) {
        let x0 = (screen_min[0].floor() as isize).max(0) as usize;
        let y0 = (screen_min[1].floor() as isize).max(0) as usize;
        let x1 = ((screen_max[0].ceil() as isize).max(0) as usize).min(self.width);
        let y1 = ((screen_max[1].ceil() as isize).max(0) as usize).min(self.height);

        for y in y0..y1 {
            for x in x0..x1 {
                let idx = y * self.width + x;
                if depth_val < self.depth[idx] {
                    self.depth[idx] = depth_val;
                }
            }
        }
    }

    /// Test if an AABB is occluded (fully behind existing geometry).
    /// Uses the hierarchical structure for fast rejection when mip levels
    /// are available; falls back to finest-level scan otherwise.
    ///
    /// `depth_val` is the *far* depth of the test AABB (the farthest
    /// point from the camera). If the entire screen rect has closer depth
    /// already written, the AABB is occluded.
    pub fn is_occluded(
        &self,
        screen_min: [f32; 2],
        screen_max: [f32; 2],
        depth_val: f32,
    ) -> bool {
        // First try hierarchical test with mip levels (coarse to fine).
        if !self.mip_levels.is_empty() {
            return self.is_occluded_hierarchical(screen_min, screen_max, depth_val);
        }

        // Fallback: check finest level directly.
        self.is_occluded_finest(screen_min, screen_max, depth_val)
    }

    /// Check occlusion against the finest depth buffer.
    fn is_occluded_finest(
        &self,
        screen_min: [f32; 2],
        screen_max: [f32; 2],
        depth_val: f32,
    ) -> bool {
        let x0 = (screen_min[0].floor() as isize).max(0) as usize;
        let y0 = (screen_min[1].floor() as isize).max(0) as usize;
        let x1 = ((screen_max[0].ceil() as isize).max(0) as usize).min(self.width);
        let y1 = ((screen_max[1].ceil() as isize).max(0) as usize).min(self.height);

        if x0 >= x1 || y0 >= y1 {
            // Zero-area region: consider it occluded (invisible).
            return true;
        }

        for y in y0..y1 {
            for x in x0..x1 {
                let idx = y * self.width + x;
                // If any pixel in the region has no closer geometry, the AABB
                // is potentially visible.
                if self.depth[idx] >= depth_val {
                    return false;
                }
            }
        }

        true
    }

    /// Hierarchical occlusion test using mip chain.
    fn is_occluded_hierarchical(
        &self,
        screen_min: [f32; 2],
        screen_max: [f32; 2],
        depth_val: f32,
    ) -> bool {
        // Choose the coarsest mip level where the AABB covers a small number
        // of pixels (ideally 1-4).
        let rect_w = screen_max[0] - screen_min[0];
        let rect_h = screen_max[1] - screen_min[1];

        // Find the mip level where the rect covers at most ~4 pixels.
        let mut level = self.mip_levels.len();
        let mut mip_w = self.width;
        let mut mip_h = self.height;

        for i in 0..self.mip_levels.len() {
            mip_w = (mip_w + 1) / 2;
            mip_h = (mip_h + 1) / 2;
            let scale_x = mip_w as f32 / self.width as f32;
            let scale_y = mip_h as f32 / self.height as f32;
            let px_w = rect_w * scale_x;
            let px_h = rect_h * scale_y;
            if px_w <= 2.0 && px_h <= 2.0 {
                level = i;
                break;
            }
        }

        if level >= self.mip_levels.len() {
            // Rect is too large even at the coarsest mip; fall back to finest.
            return self.is_occluded_finest(screen_min, screen_max, depth_val);
        }

        // Compute the pixel coordinates in the chosen mip level.
        let mut lw = self.width;
        let mut lh = self.height;
        for _ in 0..=level {
            lw = (lw + 1) / 2;
            lh = (lh + 1) / 2;
        }
        let scale_x = lw as f32 / self.width as f32;
        let scale_y = lh as f32 / self.height as f32;

        let mx0 = (screen_min[0] * scale_x).floor() as isize;
        let my0 = (screen_min[1] * scale_y).floor() as isize;
        let mx1 = (screen_max[0] * scale_x).ceil() as isize;
        let my1 = (screen_max[1] * scale_y).ceil() as isize;

        let mx0 = mx0.max(0) as usize;
        let my0 = my0.max(0) as usize;
        let mx1 = (mx1.max(0) as usize).min(lw);
        let my1 = (my1.max(0) as usize).min(lh);

        let mip = &self.mip_levels[level];

        for y in my0..my1 {
            for x in mx0..mx1 {
                let idx = y * lw + x;
                if idx < mip.len() && mip[idx] >= depth_val {
                    return false; // Potentially visible.
                }
            }
        }

        true
    }

    /// Build mip levels from the finest depth buffer.
    /// Each mip level is half the resolution, storing the *maximum* depth
    /// in each 2x2 block (conservative for occlusion testing: if the max
    /// depth in a block is closer than the test depth, the whole block
    /// is definitely occluding).
    pub fn build_mip_chain(&mut self) {
        self.mip_levels.clear();

        let mut prev_w = self.width;
        let mut prev_h = self.height;
        let mut prev_data = &self.depth;

        loop {
            let new_w = (prev_w + 1) / 2;
            let new_h = (prev_h + 1) / 2;
            if new_w == 0 || new_h == 0 {
                break;
            }

            let mut mip = vec![f32::MAX; new_w * new_h];

            for my in 0..new_h {
                for mx in 0..new_w {
                    // The 2x2 block of source pixels.
                    let sx = mx * 2;
                    let sy = my * 2;

                    // Use max depth (conservative: if the max in the block
                    // is closer than the test object's far depth, the object
                    // is definitely occluded).
                    let mut max_d = f32::MIN;

                    for dy in 0..2 {
                        for dx in 0..2 {
                            let px = sx + dx;
                            let py = sy + dy;
                            if px < prev_w && py < prev_h {
                                let d = prev_data[py * prev_w + px];
                                if d > max_d {
                                    max_d = d;
                                }
                            }
                        }
                    }

                    mip[my * new_w + mx] = max_d;
                }
            }

            self.mip_levels.push(mip);

            if new_w <= 1 && new_h <= 1 {
                break;
            }

            prev_w = new_w;
            prev_h = new_h;
            // For subsequent levels we need to own the data; we'll re-borrow
            // from mip_levels.
            // Break and re-process from stored levels.
            break;
        }

        // Continue building from the stored mip levels.
        loop {
            let last_level = self.mip_levels.last().unwrap();
            let last_w = {
                let mut w = self.width;
                for _ in 0..self.mip_levels.len() {
                    w = (w + 1) / 2;
                }
                w
            };
            let last_h = {
                let mut h = self.height;
                for _ in 0..self.mip_levels.len() {
                    h = (h + 1) / 2;
                }
                h
            };

            if last_w <= 1 && last_h <= 1 {
                break;
            }

            let new_w = (last_w + 1) / 2;
            let new_h = (last_h + 1) / 2;
            if new_w == 0 || new_h == 0 {
                break;
            }

            let prev = last_level.clone();

            let mut mip = vec![f32::MAX; new_w * new_h];
            for my in 0..new_h {
                for mx in 0..new_w {
                    let sx = mx * 2;
                    let sy = my * 2;
                    let mut max_d = f32::MIN;
                    for dy in 0..2 {
                        for dx in 0..2 {
                            let px = sx + dx;
                            let py = sy + dy;
                            if px < last_w && py < last_h {
                                let d = prev[py * last_w + px];
                                if d > max_d {
                                    max_d = d;
                                }
                            }
                        }
                    }
                    mip[my * new_w + mx] = max_d;
                }
            }

            self.mip_levels.push(mip);

            if new_w <= 1 && new_h <= 1 {
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Task 4: Distance-Based LOD Support
// ---------------------------------------------------------------------------

/// LOD level for an element based on distance from camera.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodLevel {
    /// < 50m: full detail
    Full,
    /// 50-200m: simplified geometry
    Medium,
    /// 200-500m: very simplified geometry
    Low,
    /// > 500m: just show bounding box
    BoundingBox,
    /// Beyond far plane or occluded
    Hidden,
}

impl LodLevel {
    /// Determine LOD level from distance to camera (in meters).
    pub fn from_distance(distance: f32) -> Self {
        let thresholds = Self::thresholds();
        if distance < thresholds[0] {
            LodLevel::Full
        } else if distance < thresholds[1] {
            LodLevel::Medium
        } else if distance < thresholds[2] {
            LodLevel::Low
        } else if distance < thresholds[3] {
            LodLevel::BoundingBox
        } else {
            LodLevel::Hidden
        }
    }

    /// Get the LOD distance thresholds: [Full, Medium, Low, BoundingBox].
    /// Distances beyond the last threshold are Hidden.
    pub fn thresholds() -> [f32; 4] {
        [50.0, 200.0, 500.0, 1000.0]
    }
}

/// Classify elements by LOD level based on camera position.
///
/// For each element, computes the distance from the camera to the closest
/// point on the element's AABB and maps it to a `LodLevel`.
pub fn classify_elements_by_lod(
    elements: &[(usize, [f32; 3], [f32; 3])],
    camera_pos: [f32; 3],
) -> HashMap<LodLevel, Vec<usize>> {
    let mut result: HashMap<LodLevel, Vec<usize>> = HashMap::new();

    for &(idx, min, max) in elements {
        let dist_sq = point_aabb_distance_sq(camera_pos, min, max);
        let dist = dist_sq.sqrt();
        let lod = LodLevel::from_distance(dist);
        result.entry(lod).or_insert_with(Vec::new).push(idx);
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a small AABB centered at (cx, cy, cz) with half-extent `h`.
    fn make_aabb(cx: f32, cy: f32, cz: f32, h: f32) -> ([f32; 3], [f32; 3]) {
        ([cx - h, cy - h, cz - h], [cx + h, cy + h, cz + h])
    }

    /// Helper: build a set of test elements.
    fn test_elements() -> Vec<(usize, [f32; 3], [f32; 3])> {
        vec![
            {
                let (min, max) = make_aabb(0.0, 0.0, 0.0, 1.0);
                (0, min, max)
            },
            {
                let (min, max) = make_aabb(5.0, 0.0, 0.0, 1.0);
                (1, min, max)
            },
            {
                let (min, max) = make_aabb(10.0, 0.0, 0.0, 1.0);
                (2, min, max)
            },
            {
                let (min, max) = make_aabb(0.0, 5.0, 0.0, 1.0);
                (3, min, max)
            },
            {
                let (min, max) = make_aabb(0.0, 10.0, 0.0, 1.0);
                (4, min, max)
            },
        ]
    }

    // -- BVH insert/remove tests --

    #[test]
    fn test_bvh_insert() {
        let mut elems = test_elements();
        let mut root = *BvhNode::build(&mut elems).unwrap();
        assert_eq!(root.leaf_count(), 5);

        // Insert a new element.
        let (min, max) = make_aabb(20.0, 0.0, 0.0, 1.0);
        root.insert(99, min, max);

        assert_eq!(root.leaf_count(), 6);
        assert!(root.contains(99));
    }

    #[test]
    fn test_bvh_remove() {
        let mut elems = test_elements();
        let mut root = *BvhNode::build(&mut elems).unwrap();
        assert_eq!(root.leaf_count(), 5);
        assert!(root.contains(2));

        let removed = root.remove(2);
        assert!(removed);
        assert!(!root.contains(2));
        assert_eq!(root.leaf_count(), 4);

        // Removing a non-existent element returns false.
        let removed2 = root.remove(999);
        assert!(!removed2);
    }

    #[test]
    fn test_bvh_update() {
        let mut elems = test_elements();
        let mut root = *BvhNode::build(&mut elems).unwrap();

        let (new_min, new_max) = make_aabb(100.0, 100.0, 100.0, 2.0);
        let updated = root.update(1, new_min, new_max);
        assert!(updated);
        assert!(root.contains(1));

        // The tree should still have 5 leaves.
        assert_eq!(root.leaf_count(), 5);
    }

    #[test]
    fn test_bvh_refit() {
        let mut elems = test_elements();
        let mut root = *BvhNode::build(&mut elems).unwrap();

        // Insert an element far away.
        let (min, max) = make_aabb(1000.0, 1000.0, 1000.0, 1.0);
        root.insert(50, min, max);

        root.refit();

        // After refit, the root bounds should tightly contain all elements.
        let (root_min, root_max) = root.bounds();
        assert!(root_max[0] >= 1001.0);
        assert!(root_max[1] >= 1001.0);
        assert!(root_max[2] >= 1001.0);
        assert!(root_min[0] <= -1.0);
        assert!(root_min[1] <= -1.0);
        assert!(root_min[2] <= -1.0);
    }

    #[test]
    fn test_bvh_leaf_and_node_count() {
        let mut elems = test_elements();
        let root = *BvhNode::build(&mut elems).unwrap();

        assert_eq!(root.leaf_count(), 5);
        // A binary tree with 5 leaves has 4 internal nodes = 9 total nodes.
        assert_eq!(root.node_count(), 9);
    }

    #[test]
    fn test_bvh_depth() {
        // Single element.
        let mut elems = vec![(0, [0.0; 3], [1.0; 3])];
        let root = *BvhNode::build(&mut elems).unwrap();
        assert_eq!(root.depth(), 1);

        // Two elements.
        let mut elems2 = vec![(0, [0.0; 3], [1.0; 3]), (1, [2.0; 3], [3.0; 3])];
        let root2 = *BvhNode::build(&mut elems2).unwrap();
        assert_eq!(root2.depth(), 2);
    }

    #[test]
    fn test_bvh_contains() {
        let mut elems = test_elements();
        let root = *BvhNode::build(&mut elems).unwrap();

        assert!(root.contains(0));
        assert!(root.contains(1));
        assert!(root.contains(4));
        assert!(!root.contains(99));
    }

    // -- Spatial hash tests --

    #[test]
    fn test_spatial_hash_build_and_query() {
        let elems = test_elements();
        let hash = SpatialHash::build(&elems, 3.0);

        assert_eq!(hash.len(), 5);
        assert!(!hash.is_empty());

        // Query an AABB around the origin: should find element 0.
        let results = hash.query_aabb([-0.5, -0.5, -0.5], [0.5, 0.5, 0.5]);
        assert!(results.contains(&0));
        assert!(!results.contains(&1));

        // Query a larger AABB: should find elements 0 and 1.
        let results2 = hash.query_aabb([-2.0, -2.0, -2.0], [6.0, 2.0, 2.0]);
        assert!(results2.contains(&0));
        assert!(results2.contains(&1));
    }

    #[test]
    fn test_spatial_hash_insert_remove() {
        let mut hash = SpatialHash::new(5.0);
        assert!(hash.is_empty());

        let (min, max) = make_aabb(0.0, 0.0, 0.0, 1.0);
        hash.insert(10, min, max);
        assert_eq!(hash.len(), 1);

        let (min2, max2) = make_aabb(3.0, 0.0, 0.0, 1.0);
        hash.insert(20, min2, max2);
        assert_eq!(hash.len(), 2);

        hash.remove(10);
        assert_eq!(hash.len(), 1);

        let results = hash.query_aabb([-2.0, -2.0, -2.0], [2.0, 2.0, 2.0]);
        assert!(!results.contains(&10));
    }

    #[test]
    fn test_spatial_hash_sphere_query() {
        let elems = test_elements();
        let hash = SpatialHash::build(&elems, 3.0);

        // Sphere at origin with radius 2: should find element 0.
        let results = hash.query_sphere([0.0, 0.0, 0.0], 2.0);
        assert!(results.contains(&0));

        // Sphere at origin with radius 6: should find elements 0, 1, 3.
        let results2 = hash.query_sphere([0.0, 0.0, 0.0], 6.0);
        assert!(results2.contains(&0));
        assert!(results2.contains(&1));
        assert!(results2.contains(&3));
    }

    #[test]
    fn test_spatial_hash_auto_cell_size() {
        let elems = test_elements();
        let hash = SpatialHash::build_auto(&elems);

        assert_eq!(hash.len(), 5);
        assert!(hash.cell_size > 0.0);
        assert!(hash.cell_count() > 0);

        // Should still find element 0 near the origin.
        let results = hash.query_aabb([-0.5, -0.5, -0.5], [0.5, 0.5, 0.5]);
        assert!(results.contains(&0));
    }

    #[test]
    fn test_spatial_hash_statistics() {
        let elems = test_elements();
        let hash = SpatialHash::build(&elems, 3.0);

        assert!(hash.cell_count() > 0);
        assert!(hash.avg_elements_per_cell() > 0.0);
    }

    // -- HZ-buffer tests --

    #[test]
    fn test_hz_buffer_clear_rasterize_occlude() {
        let mut hz = HierarchicalZBuffer::new(64, 64);

        // Initially everything is at far plane; nothing should be occluded
        // because depth_val (far depth of test object) < f32::MAX is never
        // true -- wait, actually is_occluded checks if existing depth is
        // *closer* than the test depth. With everything at MAX, nothing
        // is occluded (the existing depth is *not* closer).
        //
        // Actually with our logic: self.depth[idx] >= depth_val returns
        // false immediately because MAX >= 5.0 is true. Wait no:
        // is_occluded returns false if depth[idx] >= depth_val (meaning
        // the pixel has no closer geometry). So a cleared buffer means
        // nothing occludes anything. Let me verify:
        assert!(!hz.is_occluded([10.0, 10.0], [20.0, 20.0], 5.0));

        // Rasterize an occluder at depth 2.0 covering [10..30, 10..30].
        hz.rasterize_aabb([10.0, 10.0], [30.0, 30.0], 2.0);

        // A test object at depth 5.0 in the same region should be occluded
        // (it's behind the occluder).
        assert!(hz.is_occluded([12.0, 12.0], [28.0, 28.0], 5.0));

        // A test object at depth 1.0 (in front of occluder) should NOT be
        // occluded.
        assert!(!hz.is_occluded([12.0, 12.0], [28.0, 28.0], 1.0));

        // A test object partially outside the rasterized region should NOT
        // be occluded (pixels at [31..40] still have MAX depth).
        assert!(!hz.is_occluded([10.0, 10.0], [40.0, 40.0], 5.0));
    }

    #[test]
    fn test_hz_buffer_mip_chain() {
        let mut hz = HierarchicalZBuffer::new(16, 16);

        // Rasterize a full-screen occluder.
        hz.rasterize_aabb([0.0, 0.0], [16.0, 16.0], 3.0);
        hz.build_mip_chain();

        assert!(!hz.mip_levels.is_empty());

        // Should still report occlusion correctly with mip levels.
        assert!(hz.is_occluded([2.0, 2.0], [14.0, 14.0], 5.0));
        assert!(!hz.is_occluded([2.0, 2.0], [14.0, 14.0], 1.0));
    }

    // -- LOD classification tests --

    #[test]
    fn test_lod_from_distance() {
        assert_eq!(LodLevel::from_distance(10.0), LodLevel::Full);
        assert_eq!(LodLevel::from_distance(49.9), LodLevel::Full);
        assert_eq!(LodLevel::from_distance(50.0), LodLevel::Medium);
        assert_eq!(LodLevel::from_distance(199.9), LodLevel::Medium);
        assert_eq!(LodLevel::from_distance(200.0), LodLevel::Low);
        assert_eq!(LodLevel::from_distance(500.0), LodLevel::BoundingBox);
        assert_eq!(LodLevel::from_distance(1000.0), LodLevel::Hidden);
        assert_eq!(LodLevel::from_distance(5000.0), LodLevel::Hidden);
    }

    #[test]
    fn test_classify_elements_by_lod() {
        // Elements at various distances from origin camera.
        let elements = vec![
            (0, [-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]),       // dist ~0 -> Full
            (1, [49.0, 0.0, 0.0], [51.0, 1.0, 1.0]),         // dist ~49 -> Full (closest point)
            (2, [100.0, 0.0, 0.0], [102.0, 1.0, 1.0]),       // dist ~100 -> Medium
            (3, [300.0, 0.0, 0.0], [302.0, 1.0, 1.0]),       // dist ~300 -> Low
            (4, [600.0, 0.0, 0.0], [602.0, 1.0, 1.0]),       // dist ~600 -> BoundingBox
            (5, [1500.0, 0.0, 0.0], [1502.0, 1.0, 1.0]),     // dist ~1500 -> Hidden
        ];

        let camera_pos = [0.0, 0.0, 0.0];
        let result = classify_elements_by_lod(&elements, camera_pos);

        assert!(result.get(&LodLevel::Full).unwrap().contains(&0));
        assert!(result.get(&LodLevel::Full).unwrap().contains(&1));
        assert!(result.get(&LodLevel::Medium).unwrap().contains(&2));
        assert!(result.get(&LodLevel::Low).unwrap().contains(&3));
        assert!(result.get(&LodLevel::BoundingBox).unwrap().contains(&4));
        assert!(result.get(&LodLevel::Hidden).unwrap().contains(&5));
    }

    #[test]
    fn test_spatial_hash_nearest_query() {
        let elems = test_elements();
        let hash = SpatialHash::build(&elems, 3.0);

        // Nearest to origin should be element 0.
        let nearest = hash.query_nearest([0.0, 0.0, 0.0], 1);
        assert_eq!(nearest.len(), 1);
        assert_eq!(nearest[0], 0);

        // Nearest 3 to origin.
        let nearest3 = hash.query_nearest([0.0, 0.0, 0.0], 3);
        assert_eq!(nearest3.len(), 3);
        // Element 0 should be first (distance 0).
        assert_eq!(nearest3[0], 0);
    }

    #[test]
    fn test_bvh_insert_multiple_and_remove() {
        // Start with a single element and build up incrementally.
        let mut root = BvhNode::Leaf {
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
            element_index: 0,
        };

        root.insert(1, [5.0, 0.0, 0.0], [6.0, 1.0, 1.0]);
        root.insert(2, [10.0, 0.0, 0.0], [11.0, 1.0, 1.0]);
        root.insert(3, [15.0, 0.0, 0.0], [16.0, 1.0, 1.0]);

        assert_eq!(root.leaf_count(), 4);
        assert!(root.contains(0));
        assert!(root.contains(1));
        assert!(root.contains(2));
        assert!(root.contains(3));

        // Remove from the middle.
        assert!(root.remove(2));
        assert!(!root.contains(2));
        assert_eq!(root.leaf_count(), 3);

        // Refit and verify bounds.
        root.refit();
        let (rmin, rmax) = root.bounds();
        assert!(rmin[0] <= 0.0);
        assert!(rmax[0] >= 16.0);
    }
}
