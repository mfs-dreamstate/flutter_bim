use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

use flutter_rust_bridge::frb;

use crate::bim::{ElementInfo, ModelMesh, ModelRegistry};
use crate::renderer::{BvhNode, ElementDrawRange, InstanceData, Renderer};

/// Color-by visualization mode
#[frb(ignore)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ColorMode {
    #[default]
    Normal,
    ByType,
    ByStorey,
    ByMaterial,
    ByProperty,
    Grayscale,
}

/// Consolidated application state — replaces 10 separate global statics.
#[frb(ignore)]
pub(crate) struct AppState {
    pub renderer: Option<Renderer>,
    pub registry: ModelRegistry,
    pub visibility: HashSet<String>,
    pub hidden_elements: HashSet<i32>,
    pub isolated_storey: Option<String>,
    pub color_mode: ColorMode,
    pub selected_element: Option<i32>,
    pub selected_elements: HashSet<i32>,
    pub selection_sets: HashMap<String, Vec<i32>>,
    pub color_property_name: Option<String>,
    pub pick_accelerator: PickAccelerator,
    pub pick_bvh_dirty: bool,
    pub grid_visible: bool,
    pub measurement_points: Vec<super::measurement::MeasurementPoint>,
    pub measurement_type: Option<super::measurement::MeasurementType>,
    pub section_plane: Option<super::section::SectionPlane>,
    pub view_mode: super::overlay::ViewMode,
    pub viewpoints: Vec<super::camera::ViewpointData>,
}

impl AppState {
    fn new() -> Self {
        Self {
            renderer: None,
            registry: ModelRegistry::new(),
            visibility: HashSet::new(),
            hidden_elements: HashSet::new(),
            isolated_storey: None,
            color_mode: ColorMode::Normal,
            selected_element: None,
            selected_elements: HashSet::new(),
            selection_sets: HashMap::new(),
            color_property_name: None,
            pick_accelerator: PickAccelerator::empty(),
            pick_bvh_dirty: false,
            grid_visible: true,
            measurement_points: Vec::new(),
            measurement_type: None,
            section_plane: None,
            view_mode: super::overlay::ViewMode::ThreeD,
            viewpoints: Vec::new(),
        }
    }

    /// Get a mutable reference to the renderer, or error if not initialized.
    pub fn renderer(&mut self) -> Result<&mut Renderer, String> {
        self.renderer
            .as_mut()
            .ok_or_else(|| "Renderer not initialized".to_string())
    }

    /// Rebuild the BVH pick accelerator from all visible model elements.
    pub fn rebuild_pick_bvh(&mut self) {
        // Pre-allocate with total visible element count
        let total: usize = self.registry.iter_visible().map(|(_, m)| m.elements().len()).sum();
        let mut all_elements = Vec::with_capacity(total);
        for (_id, reg_model) in self.registry.iter_visible() {
            all_elements.extend_from_slice(reg_model.elements());
        }
        self.pick_accelerator = PickAccelerator::build(all_elements);
        self.pick_bvh_dirty = false;
    }

    /// Mark the BVH as needing a rebuild (lazy — actual rebuild on next pick).
    pub fn mark_bvh_dirty(&mut self) {
        self.pick_bvh_dirty = true;
    }

    /// Rebuild BVH only if dirty (lazy evaluation for pick_element).
    pub fn ensure_bvh_fresh(&mut self) {
        if self.pick_bvh_dirty {
            self.rebuild_pick_bvh();
        }
    }
}

static APP: LazyLock<Mutex<AppState>> = LazyLock::new(|| Mutex::new(AppState::new()));

/// Lock state and run a closure. Single lock for all state access.
pub(crate) fn with_state<T>(f: impl FnOnce(&mut AppState) -> T) -> T {
    let mut state = APP.lock().unwrap();
    f(&mut state)
}

/// BVH accelerator for O(log n) ray picking.
#[frb(ignore)]
pub(crate) struct PickAccelerator {
    pub bvh: Option<Box<BvhNode>>,
    pub elements: Vec<ElementInfo>,
}

impl PickAccelerator {
    pub fn empty() -> Self {
        Self {
            bvh: None,
            elements: Vec::new(),
        }
    }

    pub fn build(elements: Vec<ElementInfo>) -> Self {
        let mut bvh_input: Vec<(usize, [f32; 3], [f32; 3])> = elements
            .iter()
            .enumerate()
            .map(|(i, e)| (i, e.bounds.min, e.bounds.max))
            .collect();
        let bvh = BvhNode::build(&mut bvh_input);
        Self { bvh, elements }
    }
}

/// Build ElementDrawRange list from a ModelMesh for non-instanced rendering.
///
/// When `hidden_types` is `None`, includes all elements (unfiltered).
/// When `Some`, skips hidden types.
pub(crate) fn draw_ranges_from_mesh(
    mesh: &ModelMesh,
    hidden_types: Option<&HashSet<String>>,
) -> Vec<ElementDrawRange> {
    let mut result = Vec::with_capacity(mesh.elements.len());
    for e in &mesh.elements {
        if let Some(ht) = hidden_types {
            if ht.contains(&e.element_type) {
                continue;
            }
        }
        result.push(ElementDrawRange {
            index_start: e.triangle_start * 3, // triangle index -> vertex index
            index_count: e.triangle_count * 3,
            aabb_min: e.bounds.min,
            aabb_max: e.bounds.max,
        });
    }
    result
}

/// Build InstanceData from a ModelMesh (kept for backward compatibility).
///
/// When `hidden_types` is `None`, includes all elements (unfiltered).
/// When `Some`, skips hidden types and highlights the selected element.
#[allow(dead_code)]
pub(crate) fn instances_from_mesh(
    mesh: &ModelMesh,
    hidden_types: Option<&HashSet<String>>,
    selected_id: Option<i32>,
) -> Vec<InstanceData> {
    let highlight_color = [0.2f32, 0.9, 0.9, 1.0];
    let mut result = Vec::with_capacity(mesh.elements.len());

    for (i, e) in mesh.elements.iter().enumerate() {
        if let Some(ht) = hidden_types {
            if ht.contains(&e.element_type) {
                continue;
            }
        }
        let center = [
            (e.bounds.min[0] + e.bounds.max[0]) * 0.5,
            (e.bounds.min[1] + e.bounds.max[1]) * 0.5,
            (e.bounds.min[2] + e.bounds.max[2]) * 0.5,
        ];
        let half_size = [
            (e.bounds.max[0] - e.bounds.min[0]) * 0.5,
            (e.bounds.max[1] - e.bounds.min[1]) * 0.5,
            (e.bounds.max[2] - e.bounds.min[2]) * 0.5,
        ];
        let c = i * 24 * 4;
        let color = if selected_id == Some(e.id) {
            highlight_color
        } else if c + 3 < mesh.colors.len() {
            [
                mesh.colors[c],
                mesh.colors[c + 1],
                mesh.colors[c + 2],
                mesh.colors[c + 3],
            ]
        } else {
            [0.7, 0.7, 0.7, 1.0]
        };
        result.push(InstanceData::new(center, half_size, color));
    }
    result
}
