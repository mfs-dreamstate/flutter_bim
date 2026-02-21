//! Point cloud loading and rendering support (LAS/LAZ files).
//!
//! Provides parsing of LAS and LAZ point cloud files, spatial filtering,
//! subsampling, colorization by various modes, and GPU-ready vertex data
//! generation.

#![allow(dead_code)]

use las::Read as _;
use serde::{Deserialize, Serialize};
use std::io::{BufReader, Cursor};

/// A single point in a point cloud.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CloudPoint {
    /// XYZ position in world coordinates.
    pub position: [f32; 3],
    /// RGBA color (0..255 per channel).
    pub color: [u8; 4],
    /// Normalized intensity value in the range 0.0 to 1.0.
    pub intensity: f32,
    /// LAS classification code (e.g., 2 = Ground, 6 = Building).
    pub classification: u8,
}

/// Point cloud metadata extracted from a LAS/LAZ file header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointCloudInfo {
    /// Total number of points in the file.
    pub point_count: u64,
    /// Minimum XYZ bounds of the point cloud.
    pub bounds_min: [f64; 3],
    /// Maximum XYZ bounds of the point cloud.
    pub bounds_max: [f64; 3],
    /// Whether the point format includes RGB color data.
    pub has_color: bool,
    /// Whether the point cloud contains intensity values.
    pub has_intensity: bool,
    /// Coordinate reference system identifier, if available.
    pub coordinate_system: Option<String>,
    /// LAS file version string (e.g., "1.2", "1.4").
    pub file_version: String,
}

/// A loaded point cloud containing all points and associated metadata.
#[derive(Debug, Clone)]
pub struct PointCloud {
    /// Metadata about the point cloud.
    pub info: PointCloudInfo,
    /// All loaded points.
    pub points: Vec<CloudPoint>,
}

/// Level-of-detail settings for point cloud rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointCloudLodSettings {
    /// Maximum number of points to send to the GPU.
    pub max_points_rendered: usize,
    /// Distance within which full point density is maintained.
    pub near_distance: f32,
    /// Maximum render distance; points beyond this are culled.
    pub far_distance: f32,
    /// Screen-space point size in pixels.
    pub point_size: f32,
    /// At far distances, keep every Nth point (1 = no decimation).
    pub decimation_factor: usize,
}

impl Default for PointCloudLodSettings {
    fn default() -> Self {
        Self {
            max_points_rendered: 2_000_000,
            near_distance: 50.0,
            far_distance: 500.0,
            point_size: 2.0,
            decimation_factor: 4,
        }
    }
}

/// Color mode for point cloud rendering.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PointColorMode {
    /// Use the original RGB color stored in each point.
    Rgb,
    /// Render as grayscale based on the intensity value.
    Intensity,
    /// Color by LAS classification code using standard colors.
    Classification,
    /// Color by height (Z coordinate) using a gradient.
    Height,
    /// Apply a single uniform RGBA color to all points.
    Single([f32; 4]),
}

// ---------------------------------------------------------------------------
// LAS classification color lookup
// ---------------------------------------------------------------------------

/// Returns the standard RGBA color for a given LAS classification code.
///
/// Uses the ASPRS standard classification colors:
/// - 0 = Never classified: gray
/// - 1 = Unassigned: light gray
/// - 2 = Ground: brown
/// - 3 = Low vegetation: light green
/// - 4 = Medium vegetation: green
/// - 5 = High vegetation: dark green
/// - 6 = Building: red
/// - 7 = Low point: magenta
/// - 8 = Reserved / Model key-point: yellow
/// - 9 = Water: blue
/// - All others: white
fn classification_color(code: u8) -> [f32; 4] {
    match code {
        0 => [0.5, 0.5, 0.5, 1.0],
        1 => [0.7, 0.7, 0.7, 1.0],
        2 => [0.6, 0.4, 0.2, 1.0],
        3 => [0.5, 0.8, 0.3, 1.0],
        4 => [0.3, 0.7, 0.2, 1.0],
        5 => [0.1, 0.5, 0.1, 1.0],
        6 => [0.8, 0.2, 0.2, 1.0],
        7 => [0.8, 0.2, 0.8, 1.0],
        8 => [0.8, 0.8, 0.2, 1.0],
        9 => [0.2, 0.3, 0.8, 1.0],
        _ => [1.0, 1.0, 1.0, 1.0],
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a LAS or LAZ file from raw bytes and load all points.
///
/// Reads the full point cloud including positions, colors, intensities, and
/// classification codes. Both uncompressed LAS and compressed LAZ formats
/// are supported (LAZ support depends on the `las` crate's `laz` feature).
///
/// # Arguments
/// * `data` - Raw bytes of a LAS or LAZ file.
///
/// # Returns
/// A `PointCloud` containing all points and metadata, or an error string.
pub fn parse_las_file(data: &[u8]) -> Result<PointCloud, String> {
    let cursor = Cursor::new(data);
    let reader_buf = BufReader::new(cursor);
    let mut reader =
        las::Reader::new(reader_buf).map_err(|e| format!("Failed to open LAS reader: {}", e))?;

    let header = reader.header().clone();
    let bounds = header.bounds();
    let version = header.version();
    let point_format = header.point_format();
    let has_color = point_format.has_color;
    // Intensity is always present in LAS point records.
    let has_intensity = true;

    let info = PointCloudInfo {
        point_count: header.number_of_points(),
        bounds_min: [bounds.min.x, bounds.min.y, bounds.min.z],
        bounds_max: [bounds.max.x, bounds.max.y, bounds.max.z],
        has_color,
        has_intensity,
        coordinate_system: None,
        file_version: format!("{}.{}", version.major, version.minor),
    };

    let mut points = Vec::with_capacity(header.number_of_points() as usize);

    for point_result in reader.points() {
        let pt = point_result.map_err(|e| format!("Error reading point: {}", e))?;

        let color = if let Some(c) = &pt.color {
            // LAS stores color as u16 (0..65535); scale to u8.
            [
                (c.red >> 8) as u8,
                (c.green >> 8) as u8,
                (c.blue >> 8) as u8,
                255u8,
            ]
        } else {
            [255, 255, 255, 255]
        };

        let intensity = pt.intensity as f32 / 65535.0;
        let classification_code: u8 = u8::from(pt.classification);

        points.push(CloudPoint {
            position: [pt.x as f32, pt.y as f32, pt.z as f32],
            color,
            intensity,
            classification: classification_code,
        });
    }

    Ok(PointCloud { info, points })
}

/// Quickly scan a LAS/LAZ file and return metadata without loading points.
///
/// Only the file header is read, making this much faster than `parse_las_file`
/// for large datasets when you only need point count, bounds, and format info.
///
/// # Arguments
/// * `data` - Raw bytes of a LAS or LAZ file.
///
/// # Returns
/// A `PointCloudInfo` with header-derived metadata, or an error string.
pub fn scan_las_info(data: &[u8]) -> Result<PointCloudInfo, String> {
    let cursor = Cursor::new(data);
    let reader_buf = BufReader::new(cursor);
    let reader =
        las::Reader::new(reader_buf).map_err(|e| format!("Failed to open LAS reader: {}", e))?;

    let header = reader.header();
    let bounds = header.bounds();
    let version = header.version();
    let point_format = header.point_format();

    Ok(PointCloudInfo {
        point_count: header.number_of_points(),
        bounds_min: [bounds.min.x, bounds.min.y, bounds.min.z],
        bounds_max: [bounds.max.x, bounds.max.y, bounds.max.z],
        has_color: point_format.has_color,
        has_intensity: true,
        coordinate_system: None,
        file_version: format!("{}.{}", version.major, version.minor),
    })
}

// ---------------------------------------------------------------------------
// Filtering and subsampling
// ---------------------------------------------------------------------------

/// Uniformly subsample a point cloud to at most `max_points` points.
///
/// Uses deterministic stride-based sampling (every Nth point) rather than
/// random sampling, ensuring reproducible results. If `max_points` is
/// greater than or equal to the number of points, all points are returned.
///
/// # Arguments
/// * `cloud` - The source point cloud.
/// * `max_points` - Maximum number of points to return.
///
/// # Returns
/// A `Vec<CloudPoint>` with at most `max_points` entries.
pub fn subsample_points(cloud: &PointCloud, max_points: usize) -> Vec<CloudPoint> {
    let count = cloud.points.len();
    if max_points == 0 {
        return Vec::new();
    }
    if count <= max_points {
        return cloud.points.clone();
    }

    // Stride-based deterministic subsampling.
    // We pick every Nth point where N = count / max_points (integer division),
    // iterating until we have enough.
    let stride = count / max_points;
    let stride = if stride == 0 { 1 } else { stride };

    let mut result = Vec::with_capacity(max_points);
    let mut idx = 0;
    while idx < count && result.len() < max_points {
        result.push(cloud.points[idx]);
        idx += stride;
    }

    result
}

/// Filter points to only those matching the given LAS classification codes.
///
/// # Arguments
/// * `cloud` - The source point cloud.
/// * `classes` - Slice of LAS classification codes to keep.
///
/// # Returns
/// A `Vec<CloudPoint>` containing only points whose classification is in `classes`.
pub fn filter_by_classification(cloud: &PointCloud, classes: &[u8]) -> Vec<CloudPoint> {
    cloud
        .points
        .iter()
        .filter(|p| classes.contains(&p.classification))
        .copied()
        .collect()
}

/// Filter points to those within an axis-aligned bounding box.
///
/// # Arguments
/// * `cloud` - The source point cloud.
/// * `min` - Minimum corner of the AABB (inclusive).
/// * `max` - Maximum corner of the AABB (inclusive).
///
/// # Returns
/// A `Vec<CloudPoint>` containing only points inside the bounding box.
pub fn filter_by_bounds(cloud: &PointCloud, min: [f32; 3], max: [f32; 3]) -> Vec<CloudPoint> {
    cloud
        .points
        .iter()
        .filter(|p| {
            p.position[0] >= min[0]
                && p.position[0] <= max[0]
                && p.position[1] >= min[1]
                && p.position[1] <= max[1]
                && p.position[2] >= min[2]
                && p.position[2] <= max[2]
        })
        .copied()
        .collect()
}

// ---------------------------------------------------------------------------
// Colorization
// ---------------------------------------------------------------------------

/// Generate RGBA colors for a slice of points based on the chosen color mode.
///
/// # Arguments
/// * `points` - The points to colorize.
/// * `mode` - How to derive color for each point.
/// * `z_min` - Minimum Z value for height-based coloring (used only with `Height` mode).
/// * `z_max` - Maximum Z value for height-based coloring (used only with `Height` mode).
///
/// # Returns
/// A `Vec<[f32; 4]>` with one RGBA entry per input point, values in 0.0..1.0.
pub fn colorize_points(
    points: &[CloudPoint],
    mode: PointColorMode,
    z_min: f32,
    z_max: f32,
) -> Vec<[f32; 4]> {
    points
        .iter()
        .map(|p| match mode {
            PointColorMode::Rgb => [
                p.color[0] as f32 / 255.0,
                p.color[1] as f32 / 255.0,
                p.color[2] as f32 / 255.0,
                p.color[3] as f32 / 255.0,
            ],
            PointColorMode::Intensity => {
                let v = p.intensity;
                [v, v, v, 1.0]
            }
            PointColorMode::Classification => classification_color(p.classification),
            PointColorMode::Height => {
                let range = z_max - z_min;
                let t = if range.abs() < f32::EPSILON {
                    0.5
                } else {
                    ((p.position[2] - z_min) / range).clamp(0.0, 1.0)
                };
                // Blue (low) -> Green (mid) -> Red (high) gradient.
                let r = (2.0 * t - 1.0).clamp(0.0, 1.0);
                let g = if t < 0.5 { 2.0 * t } else { 2.0 * (1.0 - t) };
                let b = (1.0 - 2.0 * t).clamp(0.0, 1.0);
                [r, g, b, 1.0]
            }
            PointColorMode::Single(rgba) => rgba,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// GPU data preparation
// ---------------------------------------------------------------------------

/// Convert points to flat position and color arrays suitable for GPU upload.
///
/// Positions are packed as interleaved `[x, y, z, x, y, z, ...]` floats.
/// Colors are packed as interleaved `[r, g, b, a, r, g, b, a, ...]` floats
/// in the 0.0..1.0 range, derived from the specified `PointColorMode`.
///
/// # Arguments
/// * `points` - The points to convert.
/// * `mode` - Color mode to use when generating the color array.
///
/// # Returns
/// A tuple of `(positions, colors)` where positions is `Vec<f32>` of length
/// `3 * N` and colors is `Vec<f32>` of length `4 * N`.
pub fn points_to_vertex_data(
    points: &[CloudPoint],
    mode: PointColorMode,
) -> (Vec<f32>, Vec<f32>) {
    let n = points.len();
    let mut positions = Vec::with_capacity(n * 3);
    let mut colors_out = Vec::with_capacity(n * 4);

    // Compute z bounds for height mode.
    let (z_min, z_max) = if matches!(mode, PointColorMode::Height) && !points.is_empty() {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for p in points {
            lo = lo.min(p.position[2]);
            hi = hi.max(p.position[2]);
        }
        (lo, hi)
    } else {
        (0.0, 1.0)
    };

    let rgba_values = colorize_points(points, mode, z_min, z_max);

    for (i, p) in points.iter().enumerate() {
        positions.push(p.position[0]);
        positions.push(p.position[1]);
        positions.push(p.position[2]);

        let c = &rgba_values[i];
        colors_out.push(c[0]);
        colors_out.push(c[1]);
        colors_out.push(c[2]);
        colors_out.push(c[3]);
    }

    (positions, colors_out)
}

// ---------------------------------------------------------------------------
// Bounds computation
// ---------------------------------------------------------------------------

/// Compute an axis-aligned bounding box from a set of points.
///
/// # Arguments
/// * `points` - The points to compute bounds for.
///
/// # Returns
/// A tuple `(min, max)` where each is an `[f32; 3]` representing the
/// minimum and maximum corners of the AABB. If `points` is empty, returns
/// `([0.0; 3], [0.0; 3])`.
pub fn compute_point_cloud_bounds(points: &[CloudPoint]) -> ([f32; 3], [f32; 3]) {
    if points.is_empty() {
        return ([0.0; 3], [0.0; 3]);
    }

    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];

    for p in points {
        for i in 0..3 {
            min[i] = min[i].min(p.position[i]);
            max[i] = max[i].max(p.position[i]);
        }
    }

    (min, max)
}

// ---------------------------------------------------------------------------
// Memory estimation
// ---------------------------------------------------------------------------

/// Estimate the memory required to hold `point_count` points in RAM.
///
/// Each `CloudPoint` occupies 24 bytes (12 for position, 4 for color,
/// 4 for intensity, 1 for classification, plus padding). This function
/// uses `std::mem::size_of::<CloudPoint>()` for an accurate estimate.
///
/// # Arguments
/// * `point_count` - Number of points.
///
/// # Returns
/// Estimated memory in bytes.
pub fn estimate_memory_bytes(point_count: u64) -> u64 {
    let per_point = std::mem::size_of::<CloudPoint>() as u64;
    point_count * per_point
}

// ---------------------------------------------------------------------------
// Merging
// ---------------------------------------------------------------------------

/// Merge multiple point clouds into a single combined point cloud.
///
/// All points are concatenated and the bounding box is recomputed to
/// encompass all input clouds. Metadata fields (`has_color`, `has_intensity`)
/// are set to `true` if **any** source cloud has them. The file version is
/// taken from the first cloud (or defaults to `"1.0"` if none are provided).
///
/// # Arguments
/// * `clouds` - Vector of point clouds to merge.
///
/// # Returns
/// A single `PointCloud` containing all points from all inputs.
pub fn merge_point_clouds(clouds: Vec<PointCloud>) -> PointCloud {
    if clouds.is_empty() {
        return PointCloud {
            info: PointCloudInfo {
                point_count: 0,
                bounds_min: [0.0; 3],
                bounds_max: [0.0; 3],
                has_color: false,
                has_intensity: false,
                coordinate_system: None,
                file_version: "1.0".to_string(),
            },
            points: Vec::new(),
        };
    }

    let total_points: usize = clouds.iter().map(|c| c.points.len()).sum();
    let mut all_points = Vec::with_capacity(total_points);

    let mut has_color = false;
    let mut has_intensity = false;
    let mut coordinate_system: Option<String> = None;
    let file_version = clouds[0].info.file_version.clone();

    for cloud in &clouds {
        all_points.extend_from_slice(&cloud.points);
        if cloud.info.has_color {
            has_color = true;
        }
        if cloud.info.has_intensity {
            has_intensity = true;
        }
        if coordinate_system.is_none() && cloud.info.coordinate_system.is_some() {
            coordinate_system = cloud.info.coordinate_system.clone();
        }
    }

    let (bmin, bmax) = compute_point_cloud_bounds(&all_points);

    PointCloud {
        info: PointCloudInfo {
            point_count: all_points.len() as u64,
            bounds_min: [bmin[0] as f64, bmin[1] as f64, bmin[2] as f64],
            bounds_max: [bmax[0] as f64, bmax[1] as f64, bmax[2] as f64],
            has_color,
            has_intensity,
            coordinate_system,
            file_version,
        },
        points: all_points,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test point with the given position, classification,
    /// color, and intensity.
    fn make_point(
        x: f32,
        y: f32,
        z: f32,
        classification: u8,
        color: [u8; 4],
        intensity: f32,
    ) -> CloudPoint {
        CloudPoint {
            position: [x, y, z],
            color,
            intensity,
            classification,
        }
    }

    /// Helper to create a simple test point cloud.
    fn make_test_cloud(points: Vec<CloudPoint>) -> PointCloud {
        let (bmin, bmax) = compute_point_cloud_bounds(&points);
        PointCloud {
            info: PointCloudInfo {
                point_count: points.len() as u64,
                bounds_min: [bmin[0] as f64, bmin[1] as f64, bmin[2] as f64],
                bounds_max: [bmax[0] as f64, bmax[1] as f64, bmax[2] as f64],
                has_color: true,
                has_intensity: true,
                coordinate_system: None,
                file_version: "1.4".to_string(),
            },
            points,
        }
    }

    #[test]
    fn test_point_cloud_info_default() {
        let settings = PointCloudLodSettings::default();
        assert_eq!(settings.max_points_rendered, 2_000_000);
        assert!(settings.near_distance > 0.0);
        assert!(settings.far_distance > settings.near_distance);
        assert!(settings.point_size > 0.0);
        assert!(settings.decimation_factor >= 1);
    }

    #[test]
    fn test_subsample_identity() {
        // When max_points >= point count, all points are returned.
        let points = vec![
            make_point(0.0, 0.0, 0.0, 2, [255, 0, 0, 255], 0.5),
            make_point(1.0, 0.0, 0.0, 2, [0, 255, 0, 255], 0.6),
            make_point(2.0, 0.0, 0.0, 2, [0, 0, 255, 255], 0.7),
        ];
        let cloud = make_test_cloud(points.clone());

        let result = subsample_points(&cloud, 10);
        assert_eq!(result.len(), 3);
        // Verify the points match the originals.
        for (i, p) in result.iter().enumerate() {
            assert_eq!(p.position, points[i].position);
        }
    }

    #[test]
    fn test_subsample_reduces() {
        // When max_points < count, result should have at most max_points entries.
        let points: Vec<CloudPoint> = (0..100)
            .map(|i| make_point(i as f32, 0.0, 0.0, 1, [255, 255, 255, 255], 0.5))
            .collect();
        let cloud = make_test_cloud(points);

        let result = subsample_points(&cloud, 10);
        assert!(result.len() <= 10);
        assert!(result.len() > 0);
    }

    #[test]
    fn test_filter_by_classification() {
        let points = vec![
            make_point(0.0, 0.0, 0.0, 2, [255, 0, 0, 255], 0.5),  // Ground
            make_point(1.0, 0.0, 0.0, 6, [0, 255, 0, 255], 0.6),  // Building
            make_point(2.0, 0.0, 0.0, 3, [0, 0, 255, 255], 0.7),  // Low vegetation
            make_point(3.0, 0.0, 0.0, 9, [128, 128, 0, 255], 0.8), // Water
            make_point(4.0, 0.0, 0.0, 2, [200, 100, 50, 255], 0.9), // Ground
        ];
        let cloud = make_test_cloud(points);

        // Filter for ground and building only.
        let filtered = filter_by_classification(&cloud, &[2, 6]);
        assert_eq!(filtered.len(), 3);
        assert!(filtered.iter().all(|p| p.classification == 2 || p.classification == 6));

        // Filter for water only.
        let water = filter_by_classification(&cloud, &[9]);
        assert_eq!(water.len(), 1);
        assert_eq!(water[0].classification, 9);

        // Filter for non-existent class.
        let empty = filter_by_classification(&cloud, &[99]);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_filter_by_bounds() {
        let points = vec![
            make_point(0.0, 0.0, 0.0, 2, [255, 0, 0, 255], 0.5),
            make_point(5.0, 5.0, 5.0, 2, [0, 255, 0, 255], 0.6),
            make_point(10.0, 10.0, 10.0, 2, [0, 0, 255, 255], 0.7),
            make_point(15.0, 15.0, 15.0, 2, [128, 128, 128, 255], 0.8),
        ];
        let cloud = make_test_cloud(points);

        // AABB that includes only the first two points.
        let result = filter_by_bounds(&cloud, [0.0, 0.0, 0.0], [6.0, 6.0, 6.0]);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].position, [0.0, 0.0, 0.0]);
        assert_eq!(result[1].position, [5.0, 5.0, 5.0]);

        // AABB that includes nothing.
        let empty = filter_by_bounds(&cloud, [100.0, 100.0, 100.0], [200.0, 200.0, 200.0]);
        assert!(empty.is_empty());

        // AABB that includes everything.
        let all = filter_by_bounds(&cloud, [-1.0, -1.0, -1.0], [20.0, 20.0, 20.0]);
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn test_colorize_rgb_mode() {
        let points = vec![
            make_point(0.0, 0.0, 0.0, 2, [255, 128, 0, 255], 0.5),
            make_point(1.0, 0.0, 0.0, 6, [0, 255, 64, 200], 0.6),
        ];

        let colors = colorize_points(&points, PointColorMode::Rgb, 0.0, 10.0);
        assert_eq!(colors.len(), 2);

        // First point: R=255/255=1.0, G=128/255~=0.502, B=0.0, A=1.0
        assert!((colors[0][0] - 1.0).abs() < 0.01);
        assert!((colors[0][1] - 128.0 / 255.0).abs() < 0.01);
        assert!((colors[0][2] - 0.0).abs() < 0.01);
        assert!((colors[0][3] - 1.0).abs() < 0.01);

        // Second point: R=0.0, G=1.0, B=64/255~=0.251, A=200/255~=0.784
        assert!((colors[1][0] - 0.0).abs() < 0.01);
        assert!((colors[1][1] - 1.0).abs() < 0.01);
        assert!((colors[1][2] - 64.0 / 255.0).abs() < 0.01);
        assert!((colors[1][3] - 200.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn test_colorize_height_mode() {
        let points = vec![
            make_point(0.0, 0.0, 0.0, 2, [255, 255, 255, 255], 0.5),   // z_min
            make_point(0.0, 0.0, 5.0, 2, [255, 255, 255, 255], 0.5),   // midpoint
            make_point(0.0, 0.0, 10.0, 2, [255, 255, 255, 255], 0.5),  // z_max
        ];

        let colors = colorize_points(&points, PointColorMode::Height, 0.0, 10.0);
        assert_eq!(colors.len(), 3);

        // At z_min (t=0): should be blue-ish (r=0, b=1)
        assert!(colors[0][0] < 0.1, "Low point should have low red");
        assert!(colors[0][2] > 0.9, "Low point should have high blue");

        // At z_max (t=1): should be red-ish (r=1, b=0)
        assert!(colors[2][0] > 0.9, "High point should have high red");
        assert!(colors[2][2] < 0.1, "High point should have low blue");

        // At midpoint (t=0.5): should be green-ish
        assert!(colors[1][1] > 0.8, "Mid point should have high green");

        // All alpha values should be 1.0.
        for c in &colors {
            assert!((c[3] - 1.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_colorize_classification_mode() {
        let points = vec![
            make_point(0.0, 0.0, 0.0, 2, [0, 0, 0, 255], 0.5), // Ground = brown
            make_point(0.0, 0.0, 0.0, 6, [0, 0, 0, 255], 0.5), // Building = red
            make_point(0.0, 0.0, 0.0, 9, [0, 0, 0, 255], 0.5), // Water = blue
        ];

        let colors = colorize_points(&points, PointColorMode::Classification, 0.0, 10.0);
        assert_eq!(colors.len(), 3);

        // Ground (class 2): brown [0.6, 0.4, 0.2, 1.0]
        assert!((colors[0][0] - 0.6).abs() < 0.01);
        assert!((colors[0][1] - 0.4).abs() < 0.01);
        assert!((colors[0][2] - 0.2).abs() < 0.01);

        // Building (class 6): red [0.8, 0.2, 0.2, 1.0]
        assert!((colors[1][0] - 0.8).abs() < 0.01);
        assert!((colors[1][1] - 0.2).abs() < 0.01);
        assert!((colors[1][2] - 0.2).abs() < 0.01);

        // Water (class 9): blue [0.2, 0.3, 0.8, 1.0]
        assert!((colors[2][0] - 0.2).abs() < 0.01);
        assert!((colors[2][1] - 0.3).abs() < 0.01);
        assert!((colors[2][2] - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_points_to_vertex_data() {
        let points = vec![
            make_point(1.0, 2.0, 3.0, 2, [255, 128, 64, 255], 0.5),
            make_point(4.0, 5.0, 6.0, 6, [0, 0, 0, 255], 0.8),
        ];

        let (positions, colors) = points_to_vertex_data(&points, PointColorMode::Rgb);

        // Positions: 2 points * 3 floats = 6.
        assert_eq!(positions.len(), 6);
        assert!((positions[0] - 1.0).abs() < f32::EPSILON);
        assert!((positions[1] - 2.0).abs() < f32::EPSILON);
        assert!((positions[2] - 3.0).abs() < f32::EPSILON);
        assert!((positions[3] - 4.0).abs() < f32::EPSILON);
        assert!((positions[4] - 5.0).abs() < f32::EPSILON);
        assert!((positions[5] - 6.0).abs() < f32::EPSILON);

        // Colors: 2 points * 4 floats = 8.
        assert_eq!(colors.len(), 8);
        // First point RGB: 255/255=1.0, 128/255~=0.502, 64/255~=0.251
        assert!((colors[0] - 1.0).abs() < 0.01);
        assert!((colors[1] - 128.0 / 255.0).abs() < 0.01);
        assert!((colors[2] - 64.0 / 255.0).abs() < 0.01);
        assert!((colors[3] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_bounds() {
        let points = vec![
            make_point(-10.0, 0.0, 5.0, 2, [0, 0, 0, 255], 0.5),
            make_point(20.0, -3.0, 100.0, 2, [0, 0, 0, 255], 0.5),
            make_point(5.0, 50.0, -20.0, 2, [0, 0, 0, 255], 0.5),
        ];

        let (min, max) = compute_point_cloud_bounds(&points);
        assert!((min[0] - (-10.0)).abs() < f32::EPSILON);
        assert!((min[1] - (-3.0)).abs() < f32::EPSILON);
        assert!((min[2] - (-20.0)).abs() < f32::EPSILON);
        assert!((max[0] - 20.0).abs() < f32::EPSILON);
        assert!((max[1] - 50.0).abs() < f32::EPSILON);
        assert!((max[2] - 100.0).abs() < f32::EPSILON);

        // Empty case.
        let (emin, emax) = compute_point_cloud_bounds(&[]);
        assert_eq!(emin, [0.0; 3]);
        assert_eq!(emax, [0.0; 3]);
    }

    #[test]
    fn test_merge_point_clouds() {
        let cloud_a = make_test_cloud(vec![
            make_point(0.0, 0.0, 0.0, 2, [255, 0, 0, 255], 0.5),
            make_point(1.0, 1.0, 1.0, 2, [0, 255, 0, 255], 0.6),
        ]);

        let cloud_b = make_test_cloud(vec![
            make_point(10.0, 10.0, 10.0, 6, [0, 0, 255, 255], 0.7),
        ]);

        let merged = merge_point_clouds(vec![cloud_a, cloud_b]);

        assert_eq!(merged.info.point_count, 3);
        assert_eq!(merged.points.len(), 3);

        // Bounds should encompass both clouds.
        assert!(merged.info.bounds_min[0] <= 0.0);
        assert!(merged.info.bounds_max[0] >= 10.0);
        assert!(merged.info.bounds_min[1] <= 0.0);
        assert!(merged.info.bounds_max[1] >= 10.0);

        // Empty merge.
        let empty = merge_point_clouds(vec![]);
        assert_eq!(empty.info.point_count, 0);
        assert!(empty.points.is_empty());
    }

    #[test]
    fn test_estimate_memory() {
        let size_of_point = std::mem::size_of::<CloudPoint>() as u64;

        assert_eq!(estimate_memory_bytes(0), 0);
        assert_eq!(estimate_memory_bytes(1), size_of_point);
        assert_eq!(estimate_memory_bytes(1000), 1000 * size_of_point);
        assert_eq!(estimate_memory_bytes(1_000_000), 1_000_000 * size_of_point);

        // Sanity: a CloudPoint should be around 24 bytes (12 pos + 4 color + 4 intensity + 1 class + padding).
        assert!(size_of_point >= 21, "CloudPoint should be at least 21 bytes");
        assert!(size_of_point <= 32, "CloudPoint should be at most 32 bytes");
    }

    #[test]
    fn test_colorize_single_mode() {
        let points = vec![
            make_point(0.0, 0.0, 0.0, 2, [255, 0, 0, 255], 0.5),
            make_point(1.0, 1.0, 1.0, 6, [0, 255, 0, 255], 0.6),
        ];

        let uniform_color = [0.1, 0.2, 0.3, 0.4];
        let colors = colorize_points(&points, PointColorMode::Single(uniform_color), 0.0, 1.0);

        assert_eq!(colors.len(), 2);
        for c in &colors {
            assert!((c[0] - 0.1).abs() < f32::EPSILON);
            assert!((c[1] - 0.2).abs() < f32::EPSILON);
            assert!((c[2] - 0.3).abs() < f32::EPSILON);
            assert!((c[3] - 0.4).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_colorize_intensity_mode() {
        let points = vec![
            make_point(0.0, 0.0, 0.0, 0, [0, 0, 0, 255], 0.0),
            make_point(0.0, 0.0, 0.0, 0, [0, 0, 0, 255], 0.5),
            make_point(0.0, 0.0, 0.0, 0, [0, 0, 0, 255], 1.0),
        ];

        let colors = colorize_points(&points, PointColorMode::Intensity, 0.0, 1.0);
        assert_eq!(colors.len(), 3);

        // Zero intensity => black.
        assert!((colors[0][0] - 0.0).abs() < f32::EPSILON);
        assert!((colors[0][1] - 0.0).abs() < f32::EPSILON);
        assert!((colors[0][2] - 0.0).abs() < f32::EPSILON);
        assert!((colors[0][3] - 1.0).abs() < f32::EPSILON);

        // Mid intensity => gray.
        assert!((colors[1][0] - 0.5).abs() < f32::EPSILON);

        // Full intensity => white.
        assert!((colors[2][0] - 1.0).abs() < f32::EPSILON);
        assert!((colors[2][1] - 1.0).abs() < f32::EPSILON);
        assert!((colors[2][2] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_classification_color_coverage() {
        // Verify all standard codes return correct colors.
        let gray = classification_color(0);
        assert!((gray[0] - 0.5).abs() < f32::EPSILON);

        let brown = classification_color(2);
        assert!((brown[0] - 0.6).abs() < f32::EPSILON);
        assert!((brown[1] - 0.4).abs() < f32::EPSILON);

        let building = classification_color(6);
        assert!((building[0] - 0.8).abs() < f32::EPSILON);

        let water = classification_color(9);
        assert!((water[2] - 0.8).abs() < f32::EPSILON);

        // Unknown code => white.
        let unknown = classification_color(255);
        assert!((unknown[0] - 1.0).abs() < f32::EPSILON);
        assert!((unknown[1] - 1.0).abs() < f32::EPSILON);
        assert!((unknown[2] - 1.0).abs() < f32::EPSILON);
    }
}
