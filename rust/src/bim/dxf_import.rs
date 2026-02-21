//! DXF 2D Import
//!
//! Parses DXF (Drawing Exchange Format) files into 2D entities suitable for
//! overlay rendering in the BIM viewer (site plans, floor plans, etc.).
//!
//! DXF is a text-based format from AutoCAD using group-code / value pairs.
//! We support a practical subset of entities: LINE, CIRCLE, ARC, POLYLINE/
//! LWPOLYLINE, TEXT, and INSERT (block references).

use std::f32::consts::PI;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A parsed DXF drawing containing entities, layers, and bounding box.
#[derive(Debug, Clone)]
pub struct DxfDrawing {
    pub entities: Vec<DxfEntity>,
    pub layers: Vec<DxfLayer>,
    pub bounds_min: [f32; 2],
    pub bounds_max: [f32; 2],
}

/// A DXF layer with color and visibility.
#[derive(Debug, Clone)]
pub struct DxfLayer {
    pub name: String,
    pub color: u8,
    pub visible: bool,
}

/// A 2D DXF entity.
#[derive(Debug, Clone)]
pub enum DxfEntity {
    Line {
        layer: String,
        start: [f32; 2],
        end: [f32; 2],
        color: Option<u8>,
    },
    Circle {
        layer: String,
        center: [f32; 2],
        radius: f32,
        color: Option<u8>,
    },
    Arc {
        layer: String,
        center: [f32; 2],
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        color: Option<u8>,
    },
    Polyline {
        layer: String,
        points: Vec<[f32; 2]>,
        closed: bool,
        color: Option<u8>,
    },
    Text {
        layer: String,
        position: [f32; 2],
        height: f32,
        text: String,
        color: Option<u8>,
    },
    Insert {
        layer: String,
        position: [f32; 2],
        block_name: String,
        scale: [f32; 2],
        rotation: f32,
    },
}

// ---------------------------------------------------------------------------
// Group code / value pair
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct DxfPair {
    code: i32,
    value: String,
}

/// Parse raw DXF content into group-code / value pairs.
fn parse_pairs(content: &str) -> Vec<DxfPair> {
    let lines: Vec<&str> = content.lines().map(|l| l.trim()).collect();
    let mut pairs = Vec::new();
    let mut i = 0;
    while i + 1 < lines.len() {
        if let Ok(code) = lines[i].parse::<i32>() {
            pairs.push(DxfPair {
                code,
                value: lines[i + 1].to_string(),
            });
            i += 2;
        } else {
            // Skip malformed lines
            i += 1;
        }
    }
    pairs
}

// ---------------------------------------------------------------------------
// Section extraction helpers
// ---------------------------------------------------------------------------

/// Find the range of pairs belonging to a named section (ENTITIES, TABLES, etc.).
fn find_section<'a>(pairs: &'a [DxfPair], section_name: &str) -> Option<&'a [DxfPair]> {
    let mut start = None;
    let mut i = 0;
    while i < pairs.len() {
        // Look for (0, "SECTION") followed by (2, section_name)
        if pairs[i].code == 0 && pairs[i].value == "SECTION" {
            if i + 1 < pairs.len() && pairs[i + 1].code == 2 && pairs[i + 1].value == section_name
            {
                start = Some(i + 2); // skip the SECTION and name pairs
            }
        }
        if pairs[i].code == 0 && pairs[i].value == "ENDSEC" {
            if let Some(s) = start {
                return Some(&pairs[s..i]);
            }
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// Layer parsing (from TABLES section)
// ---------------------------------------------------------------------------

fn parse_layers(pairs: &[DxfPair]) -> Vec<DxfLayer> {
    let tables = match find_section(pairs, "TABLES") {
        Some(s) => s,
        None => return vec![DxfLayer { name: "0".to_string(), color: 7, visible: true }],
    };

    let mut layers = Vec::new();
    let mut i = 0;
    while i < tables.len() {
        // Find LAYER entities inside the TABLE
        if tables[i].code == 0 && tables[i].value == "LAYER" {
            let mut name = String::new();
            let mut color: u8 = 7;
            let mut visible = true;
            let mut j = i + 1;
            while j < tables.len() && !(tables[j].code == 0) {
                match tables[j].code {
                    2 => name = tables[j].value.clone(),
                    62 => {
                        let c: i32 = tables[j].value.parse().unwrap_or(7);
                        if c < 0 {
                            // Negative color means layer is off/frozen
                            visible = false;
                            color = (-c) as u8;
                        } else {
                            color = c as u8;
                        }
                    }
                    70 => {
                        let flags: i32 = tables[j].value.parse().unwrap_or(0);
                        if flags & 1 != 0 {
                            visible = false; // frozen
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            if !name.is_empty() {
                layers.push(DxfLayer { name, color, visible });
            }
            i = j;
        } else {
            i += 1;
        }
    }

    if layers.is_empty() {
        layers.push(DxfLayer { name: "0".to_string(), color: 7, visible: true });
    }
    layers
}

// ---------------------------------------------------------------------------
// Entity parsing
// ---------------------------------------------------------------------------

/// Split a slice of pairs at each entity boundary (code 0).
fn split_entities(pairs: &[DxfPair]) -> Vec<&[DxfPair]> {
    let mut entities = Vec::new();
    let mut start = 0;
    for (i, p) in pairs.iter().enumerate() {
        if p.code == 0 && i > 0 {
            entities.push(&pairs[start..i]);
            start = i;
        }
    }
    if start < pairs.len() {
        entities.push(&pairs[start..]);
    }
    entities
}

fn get_str(pairs: &[DxfPair], code: i32) -> String {
    pairs
        .iter()
        .find(|p| p.code == code)
        .map(|p| p.value.clone())
        .unwrap_or_default()
}

fn get_f32(pairs: &[DxfPair], code: i32) -> f32 {
    pairs
        .iter()
        .find(|p| p.code == code)
        .and_then(|p| p.value.parse::<f32>().ok())
        .unwrap_or(0.0)
}

fn get_optional_color(pairs: &[DxfPair]) -> Option<u8> {
    pairs
        .iter()
        .find(|p| p.code == 62)
        .and_then(|p| p.value.parse::<u8>().ok())
}

fn parse_entity(pairs: &[DxfPair]) -> Option<DxfEntity> {
    if pairs.is_empty() {
        return None;
    }
    let entity_type = if pairs[0].code == 0 {
        &pairs[0].value
    } else {
        return None;
    };

    let layer = get_str(pairs, 8);
    let color = get_optional_color(pairs);

    match entity_type.as_str() {
        "LINE" => {
            let start = [get_f32(pairs, 10), get_f32(pairs, 20)];
            let end = [get_f32(pairs, 11), get_f32(pairs, 21)];
            Some(DxfEntity::Line { layer, start, end, color })
        }
        "CIRCLE" => {
            let center = [get_f32(pairs, 10), get_f32(pairs, 20)];
            let radius = get_f32(pairs, 40);
            Some(DxfEntity::Circle { layer, center, radius, color })
        }
        "ARC" => {
            let center = [get_f32(pairs, 10), get_f32(pairs, 20)];
            let radius = get_f32(pairs, 40);
            let start_angle = get_f32(pairs, 50);
            let end_angle = get_f32(pairs, 51);
            Some(DxfEntity::Arc {
                layer,
                center,
                radius,
                start_angle,
                end_angle,
                color,
            })
        }
        "LWPOLYLINE" => {
            // LWPOLYLINE uses multiple group-10/20 pairs for vertices
            let mut points = Vec::new();
            let mut x = 0.0_f32;
            for p in pairs {
                match p.code {
                    10 => x = p.value.parse().unwrap_or(0.0),
                    20 => {
                        let y: f32 = p.value.parse().unwrap_or(0.0);
                        points.push([x, y]);
                    }
                    _ => {}
                }
            }
            // Check closed flag (group code 70, bit 1)
            let flags: i32 = pairs
                .iter()
                .find(|p| p.code == 70)
                .and_then(|p| p.value.parse().ok())
                .unwrap_or(0);
            let closed = flags & 1 != 0;
            Some(DxfEntity::Polyline { layer, points, closed, color })
        }
        "POLYLINE" => {
            // Old-style POLYLINE — vertices follow as separate VERTEX entities.
            // We collect points from VERTEX entries within the same entity group.
            // In our split, only the POLYLINE header is here; VERTEXes are separate.
            // For simplicity, we create an empty polyline and rely on the caller
            // to handle VERTEX merging (or users provide LWPOLYLINE).
            let flags: i32 = pairs
                .iter()
                .find(|p| p.code == 70)
                .and_then(|p| p.value.parse().ok())
                .unwrap_or(0);
            let closed = flags & 1 != 0;
            Some(DxfEntity::Polyline {
                layer,
                points: Vec::new(),
                closed,
                color,
            })
        }
        "TEXT" | "MTEXT" => {
            let position = [get_f32(pairs, 10), get_f32(pairs, 20)];
            let height = get_f32(pairs, 40);
            let text = if entity_type == "MTEXT" {
                get_str(pairs, 1)
            } else {
                get_str(pairs, 1)
            };
            Some(DxfEntity::Text { layer, position, height, text, color })
        }
        "INSERT" => {
            let position = [get_f32(pairs, 10), get_f32(pairs, 20)];
            let block_name = get_str(pairs, 2);
            let sx = {
                let v = get_f32(pairs, 41);
                if v == 0.0 { 1.0 } else { v }
            };
            let sy = {
                let v = get_f32(pairs, 42);
                if v == 0.0 { 1.0 } else { v }
            };
            let rotation = get_f32(pairs, 50);
            Some(DxfEntity::Insert {
                layer,
                position,
                block_name,
                scale: [sx, sy],
                rotation,
            })
        }
        _ => None, // Skip unsupported entity types
    }
}

/// Merge old-style POLYLINE + VERTEX sequences. After initial entity parsing,
/// VERTEX entities are separate. This pass absorbs them into the preceding POLYLINE.
fn merge_vertices(entities: &mut Vec<DxfEntity>, raw_groups: &[&[DxfPair]]) {
    // Build a mapping from raw_group index to entity index
    let mut group_to_entity: Vec<Option<usize>> = Vec::new();
    let mut eidx = 0;
    for group in raw_groups {
        if !group.is_empty() && group[0].code == 0 {
            match group[0].value.as_str() {
                "VERTEX" | "SEQEND" => {
                    group_to_entity.push(None);
                }
                _ => {
                    group_to_entity.push(Some(eidx));
                    eidx += 1;
                }
            }
        } else {
            group_to_entity.push(None);
        }
    }

    // Now collect VERTEX points into POLYLINE entities
    let mut current_polyline: Option<usize> = None;
    for (gi, group) in raw_groups.iter().enumerate() {
        if group.is_empty() || group[0].code != 0 {
            continue;
        }
        match group[0].value.as_str() {
            "POLYLINE" => {
                current_polyline = group_to_entity[gi];
            }
            "VERTEX" => {
                if let Some(pi) = current_polyline {
                    if pi < entities.len() {
                        let x = get_f32(group, 10);
                        let y = get_f32(group, 20);
                        if let DxfEntity::Polyline { ref mut points, .. } = entities[pi] {
                            points.push([x, y]);
                        }
                    }
                }
            }
            "SEQEND" => {
                current_polyline = None;
            }
            _ => {
                current_polyline = None;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Bounding box
// ---------------------------------------------------------------------------

fn update_bounds(min: &mut [f32; 2], max: &mut [f32; 2], point: [f32; 2]) {
    min[0] = min[0].min(point[0]);
    min[1] = min[1].min(point[1]);
    max[0] = max[0].max(point[0]);
    max[1] = max[1].max(point[1]);
}

fn compute_bounds(entities: &[DxfEntity]) -> ([f32; 2], [f32; 2]) {
    let mut min = [f32::MAX, f32::MAX];
    let mut max = [f32::MIN, f32::MIN];
    let mut has_points = false;

    for entity in entities {
        match entity {
            DxfEntity::Line { start, end, .. } => {
                update_bounds(&mut min, &mut max, *start);
                update_bounds(&mut min, &mut max, *end);
                has_points = true;
            }
            DxfEntity::Circle { center, radius, .. } => {
                update_bounds(&mut min, &mut max, [center[0] - radius, center[1] - radius]);
                update_bounds(&mut min, &mut max, [center[0] + radius, center[1] + radius]);
                has_points = true;
            }
            DxfEntity::Arc {
                center,
                radius,
                ..
            } => {
                // Conservative: use full circle bounds for simplicity
                update_bounds(&mut min, &mut max, [center[0] - radius, center[1] - radius]);
                update_bounds(&mut min, &mut max, [center[0] + radius, center[1] + radius]);
                has_points = true;
            }
            DxfEntity::Polyline { points, .. } => {
                for p in points {
                    update_bounds(&mut min, &mut max, *p);
                    has_points = true;
                }
            }
            DxfEntity::Text { position, .. } => {
                update_bounds(&mut min, &mut max, *position);
                has_points = true;
            }
            DxfEntity::Insert { position, .. } => {
                update_bounds(&mut min, &mut max, *position);
                has_points = true;
            }
        }
    }

    if !has_points {
        return ([0.0, 0.0], [0.0, 0.0]);
    }
    (min, max)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a DXF text file into a `DxfDrawing`.
pub fn parse_dxf(content: &str) -> Result<DxfDrawing, String> {
    if content.trim().is_empty() {
        return Err("Empty DXF content".to_string());
    }

    let pairs = parse_pairs(content);
    if pairs.is_empty() {
        return Err("No valid group-code/value pairs found in DXF".to_string());
    }

    // Parse layers
    let layers = parse_layers(&pairs);

    // Find ENTITIES section
    let entity_pairs = find_section(&pairs, "ENTITIES")
        .unwrap_or(&[]);

    // Split into per-entity groups and parse
    let groups = split_entities(entity_pairs);
    let mut entities: Vec<DxfEntity> = groups
        .iter()
        .filter_map(|g| parse_entity(g))
        .collect();

    // Merge old-style POLYLINE + VERTEX
    merge_vertices(&mut entities, &groups);

    // Compute bounding box
    let (bounds_min, bounds_max) = compute_bounds(&entities);

    Ok(DxfDrawing {
        entities,
        layers,
        bounds_min,
        bounds_max,
    })
}

/// Flatten all entities in a drawing into line segments for simple rendering.
///
/// Circles and arcs are tessellated with a fixed number of segments.
/// Polylines are decomposed into individual segments.
/// Text and Insert entities are skipped (they have no line geometry).
pub fn dxf_to_line_segments(drawing: &DxfDrawing) -> Vec<([f32; 2], [f32; 2])> {
    let mut segments = Vec::new();

    for entity in &drawing.entities {
        match entity {
            DxfEntity::Line { start, end, .. } => {
                segments.push((*start, *end));
            }
            DxfEntity::Circle { center, radius, .. } => {
                let n = 64;
                for i in 0..n {
                    let a0 = 2.0 * PI * (i as f32) / (n as f32);
                    let a1 = 2.0 * PI * ((i + 1) as f32) / (n as f32);
                    let p0 = [center[0] + radius * a0.cos(), center[1] + radius * a0.sin()];
                    let p1 = [center[0] + radius * a1.cos(), center[1] + radius * a1.sin()];
                    segments.push((p0, p1));
                }
            }
            DxfEntity::Arc {
                center,
                radius,
                start_angle,
                end_angle,
                ..
            } => {
                let n = 32;
                let sa = start_angle * PI / 180.0;
                let mut ea = end_angle * PI / 180.0;
                if ea <= sa {
                    ea += 2.0 * PI;
                }
                let step = (ea - sa) / (n as f32);
                for i in 0..n {
                    let a0 = sa + step * (i as f32);
                    let a1 = sa + step * ((i + 1) as f32);
                    let p0 = [center[0] + radius * a0.cos(), center[1] + radius * a0.sin()];
                    let p1 = [center[0] + radius * a1.cos(), center[1] + radius * a1.sin()];
                    segments.push((p0, p1));
                }
            }
            DxfEntity::Polyline { points, closed, .. } => {
                if points.len() >= 2 {
                    for i in 0..points.len() - 1 {
                        segments.push((points[i], points[i + 1]));
                    }
                    if *closed && points.len() >= 3 {
                        segments.push((*points.last().unwrap(), points[0]));
                    }
                }
            }
            // Text and Insert have no direct line geometry
            DxfEntity::Text { .. } | DxfEntity::Insert { .. } => {}
        }
    }

    segments
}

/// Convert an AutoCAD Color Index (ACI, 0-255) to an RGB triple in [0.0, 1.0].
///
/// The standard ACI palette defines 256 colors. We map the most common ones
/// explicitly and interpolate for the rest.
pub fn dxf_color_to_rgb(color_index: u8) -> [f32; 3] {
    match color_index {
        0 => [0.0, 0.0, 0.0],         // BYBLOCK — default to black
        1 => [1.0, 0.0, 0.0],         // Red
        2 => [1.0, 1.0, 0.0],         // Yellow
        3 => [0.0, 1.0, 0.0],         // Green
        4 => [0.0, 1.0, 1.0],         // Cyan
        5 => [0.0, 0.0, 1.0],         // Blue
        6 => [1.0, 0.0, 1.0],         // Magenta
        7 => [1.0, 1.0, 1.0],         // White / Black (context-dependent)
        8 => [0.5, 0.5, 0.5],         // Dark grey
        9 => [0.75, 0.75, 0.75],      // Light grey
        // Extended colors (10-249): derive from the ACI palette formula
        10 => [1.0, 0.0, 0.0],        // Red family
        11 => [1.0, 0.5, 0.5],
        12 => [0.65, 0.0, 0.0],
        13 => [0.65, 0.325, 0.325],
        14 => [0.5, 0.0, 0.0],
        15 => [0.5, 0.25, 0.25],
        20 => [1.0, 0.25, 0.0],       // Orange family
        30 => [1.0, 0.5, 0.0],
        40 => [1.0, 0.75, 0.0],
        50 => [1.0, 1.0, 0.0],        // Yellow
        60 => [0.75, 1.0, 0.0],
        70 => [0.5, 1.0, 0.0],
        80 => [0.25, 1.0, 0.0],
        90 => [0.0, 1.0, 0.0],        // Green
        100 => [0.0, 1.0, 0.25],
        110 => [0.0, 1.0, 0.5],
        120 => [0.0, 1.0, 0.75],
        130 => [0.0, 1.0, 1.0],       // Cyan
        140 => [0.0, 0.75, 1.0],
        150 => [0.0, 0.5, 1.0],
        160 => [0.0, 0.25, 1.0],
        170 => [0.0, 0.0, 1.0],       // Blue
        180 => [0.25, 0.0, 1.0],
        190 => [0.5, 0.0, 1.0],
        200 => [0.75, 0.0, 1.0],
        210 => [1.0, 0.0, 1.0],       // Magenta
        220 => [1.0, 0.0, 0.75],
        230 => [1.0, 0.0, 0.5],
        240 => [1.0, 0.0, 0.25],
        250 => [0.33, 0.33, 0.33],    // Greys
        251 => [0.46, 0.46, 0.46],
        252 => [0.6, 0.6, 0.6],
        253 => [0.73, 0.73, 0.73],
        254 => [0.87, 0.87, 0.87],
        255 => [1.0, 1.0, 1.0],       // White
        // For any other index, use a simple hue-based formula
        other => {
            if other >= 10 && other <= 249 {
                // Map 10-249 to hue wheel
                let t = (other as f32 - 10.0) / 240.0;
                let hue = t * 360.0;
                hue_to_rgb(hue)
            } else {
                [1.0, 1.0, 1.0]
            }
        }
    }
}

/// Simple HSV-to-RGB with S=1, V=1 for hue-based colors.
fn hue_to_rgb(hue: f32) -> [f32; 3] {
    let h = ((hue % 360.0) + 360.0) % 360.0;
    let c = 1.0_f32;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    [r, g, b]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal DXF with a single LINE entity.
    fn minimal_line_dxf() -> String {
        [
            "0", "SECTION",
            "2", "ENTITIES",
            "0", "LINE",
            "8", "0",
            "10", "0.0",
            "20", "0.0",
            "11", "10.0",
            "21", "5.0",
            "0", "ENDSEC",
            "0", "EOF",
        ]
        .join("\n")
    }

    /// DXF with a CIRCLE entity.
    fn circle_dxf() -> String {
        [
            "0", "SECTION",
            "2", "ENTITIES",
            "0", "CIRCLE",
            "8", "MyLayer",
            "62", "1",
            "10", "5.0",
            "20", "5.0",
            "40", "3.0",
            "0", "ENDSEC",
            "0", "EOF",
        ]
        .join("\n")
    }

    /// DXF with an LWPOLYLINE.
    fn polyline_dxf() -> String {
        [
            "0", "SECTION",
            "2", "ENTITIES",
            "0", "LWPOLYLINE",
            "8", "Walls",
            "70", "1",
            "90", "4",
            "10", "0.0",
            "20", "0.0",
            "10", "10.0",
            "20", "0.0",
            "10", "10.0",
            "20", "10.0",
            "10", "0.0",
            "20", "10.0",
            "0", "ENDSEC",
            "0", "EOF",
        ]
        .join("\n")
    }

    #[test]
    fn test_parse_dxf_line() {
        let drawing = parse_dxf(&minimal_line_dxf()).unwrap();
        assert_eq!(drawing.entities.len(), 1);

        match &drawing.entities[0] {
            DxfEntity::Line { layer, start, end, .. } => {
                assert_eq!(layer, "0");
                assert!((start[0] - 0.0).abs() < 0.001);
                assert!((start[1] - 0.0).abs() < 0.001);
                assert!((end[0] - 10.0).abs() < 0.001);
                assert!((end[1] - 5.0).abs() < 0.001);
            }
            other => panic!("Expected Line, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_dxf_circle() {
        let drawing = parse_dxf(&circle_dxf()).unwrap();
        assert_eq!(drawing.entities.len(), 1);

        match &drawing.entities[0] {
            DxfEntity::Circle { layer, center, radius, color } => {
                assert_eq!(layer, "MyLayer");
                assert!((center[0] - 5.0).abs() < 0.001);
                assert!((center[1] - 5.0).abs() < 0.001);
                assert!((radius - 3.0).abs() < 0.001);
                assert_eq!(*color, Some(1));
            }
            other => panic!("Expected Circle, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_dxf_polyline() {
        let drawing = parse_dxf(&polyline_dxf()).unwrap();
        assert_eq!(drawing.entities.len(), 1);

        match &drawing.entities[0] {
            DxfEntity::Polyline { layer, points, closed, .. } => {
                assert_eq!(layer, "Walls");
                assert_eq!(points.len(), 4);
                assert!(*closed);
                assert!((points[0][0] - 0.0).abs() < 0.001);
                assert!((points[1][0] - 10.0).abs() < 0.001);
                assert!((points[2][1] - 10.0).abs() < 0.001);
            }
            other => panic!("Expected Polyline, got {:?}", other),
        }
    }

    #[test]
    fn test_dxf_color_to_rgb_standard() {
        // Red
        let red = dxf_color_to_rgb(1);
        assert!((red[0] - 1.0).abs() < 0.001);
        assert!((red[1] - 0.0).abs() < 0.001);
        assert!((red[2] - 0.0).abs() < 0.001);

        // Green
        let green = dxf_color_to_rgb(3);
        assert!((green[0] - 0.0).abs() < 0.001);
        assert!((green[1] - 1.0).abs() < 0.001);
        assert!((green[2] - 0.0).abs() < 0.001);

        // Blue
        let blue = dxf_color_to_rgb(5);
        assert!((blue[0] - 0.0).abs() < 0.001);
        assert!((blue[1] - 0.0).abs() < 0.001);
        assert!((blue[2] - 1.0).abs() < 0.001);

        // White
        let white = dxf_color_to_rgb(7);
        assert!((white[0] - 1.0).abs() < 0.001);
        assert!((white[1] - 1.0).abs() < 0.001);
        assert!((white[2] - 1.0).abs() < 0.001);

        // BYBLOCK (0) should be black
        let byblock = dxf_color_to_rgb(0);
        assert!((byblock[0] - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_dxf_bounds_computation() {
        let drawing = parse_dxf(&minimal_line_dxf()).unwrap();
        assert!((drawing.bounds_min[0] - 0.0).abs() < 0.001);
        assert!((drawing.bounds_min[1] - 0.0).abs() < 0.001);
        assert!((drawing.bounds_max[0] - 10.0).abs() < 0.001);
        assert!((drawing.bounds_max[1] - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_dxf_bounds_circle() {
        let drawing = parse_dxf(&circle_dxf()).unwrap();
        // Circle at (5,5) r=3 => bounds (2,2) to (8,8)
        assert!((drawing.bounds_min[0] - 2.0).abs() < 0.001);
        assert!((drawing.bounds_min[1] - 2.0).abs() < 0.001);
        assert!((drawing.bounds_max[0] - 8.0).abs() < 0.001);
        assert!((drawing.bounds_max[1] - 8.0).abs() < 0.001);
    }

    #[test]
    fn test_dxf_to_line_segments_line() {
        let drawing = parse_dxf(&minimal_line_dxf()).unwrap();
        let segs = dxf_to_line_segments(&drawing);
        assert_eq!(segs.len(), 1);
        assert!((segs[0].0[0] - 0.0).abs() < 0.001);
        assert!((segs[0].1[0] - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_dxf_to_line_segments_circle() {
        let drawing = parse_dxf(&circle_dxf()).unwrap();
        let segs = dxf_to_line_segments(&drawing);
        // Circle tessellated to 64 segments
        assert_eq!(segs.len(), 64);
    }

    #[test]
    fn test_dxf_to_line_segments_polyline_closed() {
        let drawing = parse_dxf(&polyline_dxf()).unwrap();
        let segs = dxf_to_line_segments(&drawing);
        // 4 points, closed => 4 segments (3 + 1 closing)
        assert_eq!(segs.len(), 4);
    }

    #[test]
    fn test_parse_dxf_empty_fails() {
        assert!(parse_dxf("").is_err());
    }

    #[test]
    fn test_parse_dxf_arc() {
        let dxf = [
            "0", "SECTION",
            "2", "ENTITIES",
            "0", "ARC",
            "8", "0",
            "10", "0.0",
            "20", "0.0",
            "40", "5.0",
            "50", "0.0",
            "51", "90.0",
            "0", "ENDSEC",
            "0", "EOF",
        ]
        .join("\n");

        let drawing = parse_dxf(&dxf).unwrap();
        assert_eq!(drawing.entities.len(), 1);
        match &drawing.entities[0] {
            DxfEntity::Arc { center, radius, start_angle, end_angle, .. } => {
                assert!((center[0]).abs() < 0.001);
                assert!((center[1]).abs() < 0.001);
                assert!((radius - 5.0).abs() < 0.001);
                assert!((start_angle - 0.0).abs() < 0.001);
                assert!((end_angle - 90.0).abs() < 0.001);
            }
            other => panic!("Expected Arc, got {:?}", other),
        }

        // Line segments from the arc
        let segs = dxf_to_line_segments(&drawing);
        assert_eq!(segs.len(), 32); // 32 segments for arc tessellation
    }

    #[test]
    fn test_parse_dxf_text() {
        let dxf = [
            "0", "SECTION",
            "2", "ENTITIES",
            "0", "TEXT",
            "8", "Annotations",
            "10", "1.0",
            "20", "2.0",
            "40", "0.5",
            "1", "Hello World",
            "0", "ENDSEC",
            "0", "EOF",
        ]
        .join("\n");

        let drawing = parse_dxf(&dxf).unwrap();
        assert_eq!(drawing.entities.len(), 1);
        match &drawing.entities[0] {
            DxfEntity::Text { layer, position, height, text, .. } => {
                assert_eq!(layer, "Annotations");
                assert!((position[0] - 1.0).abs() < 0.001);
                assert!((position[1] - 2.0).abs() < 0.001);
                assert!((height - 0.5).abs() < 0.001);
                assert_eq!(text, "Hello World");
            }
            other => panic!("Expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_dxf_multiple_entities() {
        let dxf = [
            "0", "SECTION",
            "2", "ENTITIES",
            "0", "LINE",
            "8", "0",
            "10", "0.0",
            "20", "0.0",
            "11", "5.0",
            "21", "5.0",
            "0", "CIRCLE",
            "8", "0",
            "10", "10.0",
            "20", "10.0",
            "40", "2.0",
            "0", "ENDSEC",
            "0", "EOF",
        ]
        .join("\n");

        let drawing = parse_dxf(&dxf).unwrap();
        assert_eq!(drawing.entities.len(), 2);
        assert!(matches!(&drawing.entities[0], DxfEntity::Line { .. }));
        assert!(matches!(&drawing.entities[1], DxfEntity::Circle { .. }));

        // Bounds should encompass both entities
        assert!((drawing.bounds_min[0] - 0.0).abs() < 0.001);
        assert!((drawing.bounds_min[1] - 0.0).abs() < 0.001);
        assert!((drawing.bounds_max[0] - 12.0).abs() < 0.001); // circle at 10 + r=2
        assert!((drawing.bounds_max[1] - 12.0).abs() < 0.001);
    }

    #[test]
    fn test_dxf_layer_parsing() {
        let dxf = [
            "0", "SECTION",
            "2", "TABLES",
            "0", "TABLE",
            "2", "LAYER",
            "0", "LAYER",
            "2", "Walls",
            "62", "1",
            "70", "0",
            "0", "LAYER",
            "2", "Hidden",
            "62", "-3",
            "70", "1",
            "0", "ENDTAB",
            "0", "ENDSEC",
            "0", "SECTION",
            "2", "ENTITIES",
            "0", "LINE",
            "8", "Walls",
            "10", "0.0",
            "20", "0.0",
            "11", "1.0",
            "21", "1.0",
            "0", "ENDSEC",
            "0", "EOF",
        ]
        .join("\n");

        let drawing = parse_dxf(&dxf).unwrap();
        assert_eq!(drawing.layers.len(), 2);
        assert_eq!(drawing.layers[0].name, "Walls");
        assert_eq!(drawing.layers[0].color, 1);
        assert!(drawing.layers[0].visible);

        assert_eq!(drawing.layers[1].name, "Hidden");
        assert_eq!(drawing.layers[1].color, 3); // abs(-3)
        assert!(!drawing.layers[1].visible);
    }
}
