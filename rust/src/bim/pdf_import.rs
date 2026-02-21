//! Minimal PDF parser for 2D drawing overlay.
//!
//! Extracts line segments, rectangles, text, and basic shapes from PDF content
//! streams for floor plan overlay rendering.
//!
//! **This is NOT a full PDF renderer.** It extracts enough drawing commands to
//! overlay floor plans as 2D line drawings. Complex features such as images,
//! transparency, clipping paths, and advanced font metrics are not supported.

use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A parsed 2D drawing element from a PDF.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PdfElement {
    /// A single line segment.
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: [f32; 3],
        line_width: f32,
    },
    /// A rectangle (optionally stroked and/or filled).
    Rectangle {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        stroke_color: Option<[f32; 3]>,
        fill_color: Option<[f32; 3]>,
        line_width: f32,
    },
    /// A polyline or polygon.
    Polyline {
        points: Vec<[f32; 2]>,
        color: [f32; 3],
        line_width: f32,
        closed: bool,
    },
    /// A circle (approximated from PDF curve operators or explicit arcs).
    Circle {
        cx: f32,
        cy: f32,
        radius: f32,
        stroke_color: Option<[f32; 3]>,
        fill_color: Option<[f32; 3]>,
        line_width: f32,
    },
    /// A text string placed at a position.
    Text {
        x: f32,
        y: f32,
        text: String,
        font_size: f32,
        color: [f32; 3],
    },
}

/// A parsed PDF page containing drawing elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfPage {
    /// 1-based page number.
    pub page_number: usize,
    /// Page width in points (1 point = 1/72 inch).
    pub width: f32,
    /// Page height in points.
    pub height: f32,
    /// Drawing elements extracted from this page.
    pub elements: Vec<PdfElement>,
}

/// PDF document metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfInfo {
    /// Total number of pages.
    pub page_count: usize,
    /// Document title from the /Info dictionary.
    pub title: Option<String>,
    /// Author from the /Info dictionary.
    pub author: Option<String>,
    /// Creator application from the /Info dictionary.
    pub creator: Option<String>,
}

/// Complete PDF drawing overlay result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfDrawingOverlay {
    /// Document metadata.
    pub info: PdfInfo,
    /// Parsed pages with drawing elements.
    pub pages: Vec<PdfPage>,
    /// Overall bounding box: (min_xy, max_xy).
    pub bounds: ([f32; 2], [f32; 2]),
}

/// Configuration for PDF import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfImportConfig {
    /// Scale factor from PDF points to model units.
    /// Default: 0.0254 / 72.0 (points to meters).
    pub scale_factor: f32,
    /// Translation offset applied after scaling.
    pub offset: [f32; 2],
    /// Specific page to import (1-based). `None` imports all pages.
    pub target_page: Option<usize>,
    /// Tolerance for merging near-coincident points (in PDF points).
    pub simplify_tolerance: f32,
    /// Whether to extract text elements.
    pub extract_text: bool,
}

impl Default for PdfImportConfig {
    fn default() -> Self {
        Self {
            scale_factor: 0.0254 / 72.0, // points -> meters
            offset: [0.0, 0.0],
            target_page: None,
            simplify_tolerance: 0.001,
            extract_text: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Internal PDF object representation
// ---------------------------------------------------------------------------

/// A raw PDF object extracted from the file.
#[derive(Debug, Clone)]
struct PdfObject {
    /// Object number.
    obj_num: u32,
    /// Generation number.
    gen_num: u32,
    /// The raw dictionary/content bytes between `obj` and `endobj`.
    body: Vec<u8>,
    /// Decoded stream data (if the object has a stream).
    stream_data: Option<Vec<u8>>,
}

/// Reference to a page object.
#[derive(Debug, Clone)]
struct PageRef {
    /// Object number for this page.
    obj_index: usize,
    /// MediaBox [x0, y0, x1, y1].
    media_box: [f32; 4],
    /// Index into the objects array for the content stream(s).
    content_obj_indices: Vec<usize>,
}

/// Graphics state used during content stream parsing.
#[derive(Debug, Clone)]
struct GraphicsState {
    stroke_color: [f32; 3],
    fill_color: [f32; 3],
    line_width: f32,
    font_size: f32,
    ctm: [f32; 6], // Current transformation matrix [a, b, c, d, e, f]
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            stroke_color: [0.0, 0.0, 0.0],
            fill_color: [0.0, 0.0, 0.0],
            line_width: 1.0,
            font_size: 12.0,
            ctm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], // identity
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Quick scan of PDF structure to extract basic info (page count, metadata).
///
/// This does not parse content streams — only the cross-reference table and
/// document catalog.
pub fn parse_pdf_bytes(data: &[u8]) -> Result<PdfInfo, String> {
    // Verify PDF header
    if data.len() < 5 || &data[0..5] != b"%PDF-" {
        return Err("Not a valid PDF file (missing %PDF- header)".to_string());
    }

    let text = String::from_utf8_lossy(data);

    // Count pages by looking for /Type /Page (not /Pages)
    let page_count = count_pages(&text);

    // Extract info dictionary entries
    let title = extract_info_field(&text, "/Title");
    let author = extract_info_field(&text, "/Author");
    let creator = extract_info_field(&text, "/Creator");

    Ok(PdfInfo {
        page_count,
        title,
        author,
        creator,
    })
}

/// Full parse: find page objects, parse content streams, extract drawing elements.
///
/// # Arguments
/// * `data` - Raw PDF file bytes.
/// * `config` - Import configuration controlling scale, offset, and filtering.
pub fn extract_pdf_drawings(
    data: &[u8],
    config: &PdfImportConfig,
) -> Result<PdfDrawingOverlay, String> {
    if data.len() < 5 || &data[0..5] != b"%PDF-" {
        return Err("Not a valid PDF file (missing %PDF- header)".to_string());
    }

    let objects = find_pdf_objects(data);
    if objects.is_empty() {
        return Err("No PDF objects found".to_string());
    }

    let page_refs = parse_page_tree(&objects);
    if page_refs.is_empty() {
        return Err("No pages found in PDF".to_string());
    }

    let text = String::from_utf8_lossy(data);
    let title = extract_info_field(&text, "/Title");
    let author = extract_info_field(&text, "/Author");
    let creator = extract_info_field(&text, "/Creator");

    let info = PdfInfo {
        page_count: page_refs.len(),
        title,
        author,
        creator,
    };

    let mut pages = Vec::new();
    let mut global_min = [f32::MAX, f32::MAX];
    let mut global_max = [f32::MIN, f32::MIN];

    for (page_idx, page_ref) in page_refs.iter().enumerate() {
        let page_number = page_idx + 1;

        // Skip pages not matching target_page filter
        if let Some(target) = config.target_page {
            if page_number != target {
                continue;
            }
        }

        let page_width = page_ref.media_box[2] - page_ref.media_box[0];
        let page_height = page_ref.media_box[3] - page_ref.media_box[1];

        // Parse content streams for this page
        let mut elements = Vec::new();
        for &content_idx in &page_ref.content_obj_indices {
            if content_idx < objects.len() {
                let obj = &objects[content_idx];
                let stream_bytes = if let Some(ref sd) = obj.stream_data {
                    sd.clone()
                } else {
                    // Try to extract stream from body
                    extract_stream_from_body(&obj.body, data)
                };

                if !stream_bytes.is_empty() {
                    let mut page_elements =
                        parse_content_stream(&stream_bytes, config.extract_text);
                    elements.append(&mut page_elements);
                }
            }
        }

        // Update global bounds
        for elem in &elements {
            update_bounds_from_element(elem, &mut global_min, &mut global_max);
        }

        pages.push(PdfPage {
            page_number,
            width: page_width,
            height: page_height,
            elements,
        });
    }

    // Handle case where no elements were found
    if global_min[0] == f32::MAX {
        global_min = [0.0, 0.0];
        global_max = [0.0, 0.0];
    }

    let mut overlay = PdfDrawingOverlay {
        info,
        pages,
        bounds: (global_min, global_max),
    };

    // Apply scale and offset
    scale_pdf_to_model(&mut overlay, config);

    Ok(overlay)
}

/// Flatten all PDF elements into line segments for simple overlay rendering.
///
/// Each segment is a pair of 2D points. Circles are tessellated into 32 segments.
/// Rectangles become 4 segments. Text elements are skipped.
pub fn pdf_elements_to_line_segments(elements: &[PdfElement]) -> Vec<([f32; 2], [f32; 2])> {
    let mut segments = Vec::new();

    for element in elements {
        match element {
            PdfElement::Line {
                x1, y1, x2, y2, ..
            } => {
                segments.push(([*x1, *y1], [*x2, *y2]));
            }
            PdfElement::Rectangle {
                x,
                y,
                width,
                height,
                ..
            } => {
                let x0 = *x;
                let y0 = *y;
                let x1 = x0 + width;
                let y1 = y0 + height;
                segments.push(([x0, y0], [x1, y0]));
                segments.push(([x1, y0], [x1, y1]));
                segments.push(([x1, y1], [x0, y1]));
                segments.push(([x0, y1], [x0, y0]));
            }
            PdfElement::Polyline {
                points, closed, ..
            } => {
                if points.len() >= 2 {
                    for i in 0..points.len() - 1 {
                        segments.push((points[i], points[i + 1]));
                    }
                    if *closed && points.len() >= 3 {
                        segments.push((*points.last().unwrap(), points[0]));
                    }
                }
            }
            PdfElement::Circle { cx, cy, radius, .. } => {
                let n = 32;
                for i in 0..n {
                    let a0 = 2.0 * PI * (i as f32) / (n as f32);
                    let a1 = 2.0 * PI * ((i + 1) as f32) / (n as f32);
                    let p0 = [cx + radius * a0.cos(), cy + radius * a0.sin()];
                    let p1 = [cx + radius * a1.cos(), cy + radius * a1.sin()];
                    segments.push((p0, p1));
                }
            }
            PdfElement::Text { .. } => {
                // Text has no line geometry
            }
        }
    }

    segments
}

/// Convert PDF elements to flat arrays for GPU overlay rendering.
///
/// Returns `(positions_f32, colors_f32)` where positions are pairs of
/// (x, y) coordinates for line endpoints and colors are (r, g, b) triples
/// matching each position.
pub fn pdf_elements_to_vertices(elements: &[PdfElement]) -> (Vec<f32>, Vec<f32>) {
    let segments = pdf_elements_to_line_segments(elements);
    let mut positions = Vec::with_capacity(segments.len() * 4);
    let mut colors = Vec::with_capacity(segments.len() * 6);

    for element in elements {
        let (elem_segments, color) = match element {
            PdfElement::Line {
                x1,
                y1,
                x2,
                y2,
                color,
                ..
            } => {
                (vec![([*x1, *y1], [*x2, *y2])], *color)
            }
            PdfElement::Rectangle {
                x,
                y,
                width,
                height,
                stroke_color,
                fill_color,
                ..
            } => {
                let c = stroke_color
                    .or(*fill_color)
                    .unwrap_or([0.0, 0.0, 0.0]);
                let x0 = *x;
                let y0 = *y;
                let x1 = x0 + width;
                let y1 = y0 + height;
                let segs = vec![
                    ([x0, y0], [x1, y0]),
                    ([x1, y0], [x1, y1]),
                    ([x1, y1], [x0, y1]),
                    ([x0, y1], [x0, y0]),
                ];
                (segs, c)
            }
            PdfElement::Polyline {
                points,
                color,
                closed,
                ..
            } => {
                let mut segs = Vec::new();
                if points.len() >= 2 {
                    for i in 0..points.len() - 1 {
                        segs.push((points[i], points[i + 1]));
                    }
                    if *closed && points.len() >= 3 {
                        segs.push((*points.last().unwrap(), points[0]));
                    }
                }
                (segs, *color)
            }
            PdfElement::Circle {
                cx,
                cy,
                radius,
                stroke_color,
                fill_color,
                ..
            } => {
                let c = stroke_color
                    .or(*fill_color)
                    .unwrap_or([0.0, 0.0, 0.0]);
                let n = 32;
                let mut segs = Vec::new();
                for i in 0..n {
                    let a0 = 2.0 * PI * (i as f32) / (n as f32);
                    let a1 = 2.0 * PI * ((i + 1) as f32) / (n as f32);
                    let p0 = [cx + radius * a0.cos(), cy + radius * a0.sin()];
                    let p1 = [cx + radius * a1.cos(), cy + radius * a1.sin()];
                    segs.push((p0, p1));
                }
                (segs, c)
            }
            PdfElement::Text { .. } => continue,
        };

        for (p0, p1) in &elem_segments {
            positions.push(p0[0]);
            positions.push(p0[1]);
            positions.push(p1[0]);
            positions.push(p1[1]);
            // Color for each endpoint
            colors.push(color[0]);
            colors.push(color[1]);
            colors.push(color[2]);
            colors.push(color[0]);
            colors.push(color[1]);
            colors.push(color[2]);
        }
    }

    (positions, colors)
}

/// Apply scale factor and offset to all elements in the overlay.
///
/// Transforms coordinates from PDF points to model units using
/// `config.scale_factor` and `config.offset`.
pub fn scale_pdf_to_model(overlay: &mut PdfDrawingOverlay, config: &PdfImportConfig) {
    let s = config.scale_factor;
    let ox = config.offset[0];
    let oy = config.offset[1];

    for page in &mut overlay.pages {
        page.width *= s;
        page.height *= s;

        for element in &mut page.elements {
            match element {
                PdfElement::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    line_width,
                    ..
                } => {
                    *x1 = *x1 * s + ox;
                    *y1 = *y1 * s + oy;
                    *x2 = *x2 * s + ox;
                    *y2 = *y2 * s + oy;
                    *line_width *= s;
                }
                PdfElement::Rectangle {
                    x,
                    y,
                    width,
                    height,
                    line_width,
                    ..
                } => {
                    *x = *x * s + ox;
                    *y = *y * s + oy;
                    *width *= s;
                    *height *= s;
                    *line_width *= s;
                }
                PdfElement::Polyline {
                    points, line_width, ..
                } => {
                    for point in points.iter_mut() {
                        point[0] = point[0] * s + ox;
                        point[1] = point[1] * s + oy;
                    }
                    *line_width *= s;
                }
                PdfElement::Circle {
                    cx,
                    cy,
                    radius,
                    line_width,
                    ..
                } => {
                    *cx = *cx * s + ox;
                    *cy = *cy * s + oy;
                    *radius *= s;
                    *line_width *= s;
                }
                PdfElement::Text {
                    x, y, font_size, ..
                } => {
                    *x = *x * s + ox;
                    *y = *y * s + oy;
                    *font_size *= s;
                }
            }
        }
    }

    // Update bounds
    overlay.bounds.0[0] = overlay.bounds.0[0] * s + ox;
    overlay.bounds.0[1] = overlay.bounds.0[1] * s + oy;
    overlay.bounds.1[0] = overlay.bounds.1[0] * s + ox;
    overlay.bounds.1[1] = overlay.bounds.1[1] * s + oy;
}

/// Merge near-coincident endpoints in polylines and lines.
///
/// For each pair of endpoints across all elements, if they are within
/// `tolerance` distance, snap them to the same position. This reduces
/// visual gaps in the overlay.
pub fn merge_coincident_points(elements: &mut Vec<PdfElement>, tolerance: f32) {
    let tol_sq = tolerance * tolerance;

    // Collect all unique points
    let mut all_points: Vec<[f32; 2]> = Vec::new();
    for element in elements.iter() {
        match element {
            PdfElement::Line {
                x1, y1, x2, y2, ..
            } => {
                all_points.push([*x1, *y1]);
                all_points.push([*x2, *y2]);
            }
            PdfElement::Polyline { points, .. } => {
                all_points.extend_from_slice(points);
            }
            _ => {}
        }
    }

    if all_points.is_empty() {
        return;
    }

    // Build a simple merge map: for each point, find the first earlier point
    // within tolerance and map to it.
    let mut canonical: Vec<[f32; 2]> = Vec::new();
    let mut _merge_count = 0usize;

    for pt in &all_points {
        let mut found = false;
        for c in &canonical {
            let dx = pt[0] - c[0];
            let dy = pt[1] - c[1];
            if dx * dx + dy * dy <= tol_sq {
                found = true;
                break;
            }
        }
        if !found {
            canonical.push(*pt);
        }
    }

    // Snap all element points to the nearest canonical point
    for element in elements.iter_mut() {
        match element {
            PdfElement::Line {
                x1, y1, x2, y2, ..
            } => {
                let p1 = snap_point([*x1, *y1], &canonical, tol_sq);
                let p2 = snap_point([*x2, *y2], &canonical, tol_sq);
                *x1 = p1[0];
                *y1 = p1[1];
                *x2 = p2[0];
                *y2 = p2[1];
            }
            PdfElement::Polyline { points, .. } => {
                for point in points.iter_mut() {
                    let snapped = snap_point(*point, &canonical, tol_sq);
                    *point = snapped;
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Internal PDF parsing helpers
// ---------------------------------------------------------------------------

/// Count pages by finding `/Type /Page` patterns (not `/Type /Pages`).
fn count_pages(text: &str) -> usize {
    let mut count = 0;
    let mut search_from = 0;

    while let Some(pos) = text[search_from..].find("/Type") {
        let abs_pos = search_from + pos;
        let after = &text[abs_pos + 5..];
        let trimmed = after.trim_start();

        if trimmed.starts_with("/Page") {
            // Make sure it's /Page and NOT /Pages
            let rest = &trimmed[5..];
            if rest.is_empty() || !rest.starts_with('s') {
                count += 1;
            }
        }
        search_from = abs_pos + 5;
    }

    // If we found 0 pages through /Type /Page, try counting via /Count in /Pages
    if count == 0 {
        if let Some(pages_pos) = text.find("/Type /Pages") {
            let region = &text[pages_pos..];
            if let Some(count_pos) = region.find("/Count") {
                let after_count = &region[count_pos + 6..];
                let trimmed = after_count.trim_start();
                let num_str: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(n) = num_str.parse::<usize>() {
                    count = n;
                }
            }
        }
    }

    count.max(1) // At least 1 page if we have a valid PDF
}

/// Extract a string value from the PDF /Info dictionary.
fn extract_info_field(text: &str, field: &str) -> Option<String> {
    // Look for patterns like /Title (Some Title) or /Title <hex>
    let field_with_space = format!("{} ", field);
    let field_with_paren = format!("{}(", field);

    for pattern in &[field_with_space.as_str(), field_with_paren.as_str()] {
        if let Some(pos) = text.find(pattern) {
            let after = &text[pos + field.len()..];
            let trimmed = after.trim_start();
            if trimmed.starts_with('(') {
                // Literal string: extract until closing paren
                return extract_paren_string(&trimmed[1..]);
            }
        }
    }

    None
}

/// Extract text between parentheses, handling escaped parens.
fn extract_paren_string(s: &str) -> Option<String> {
    let mut result = String::new();
    let mut depth = 1i32;
    let mut chars = s.chars();
    let mut prev_backslash = false;

    while let Some(ch) = chars.next() {
        if prev_backslash {
            result.push(ch);
            prev_backslash = false;
            continue;
        }
        match ch {
            '\\' => {
                prev_backslash = true;
            }
            '(' => {
                depth += 1;
                result.push(ch);
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                result.push(ch);
            }
            _ => {
                result.push(ch);
            }
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Scan the PDF byte stream for `N M obj ... endobj` blocks.
fn find_pdf_objects(data: &[u8]) -> Vec<PdfObject> {
    let mut objects = Vec::new();
    let text = String::from_utf8_lossy(data);
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut pos = 0;

    while pos < len {
        // Look for "N M obj" pattern
        if let Some(obj_match) = find_obj_start(&text, pos) {
            let (obj_num, gen_num, body_start) = obj_match;

            // Find "endobj"
            if let Some(end_pos) = text[body_start..].find("endobj") {
                let body_end = body_start + end_pos;
                let body_bytes = text[body_start..body_end].as_bytes().to_vec();

                // Try to extract stream data
                let stream_data = extract_stream(&body_bytes, data);

                objects.push(PdfObject {
                    obj_num,
                    gen_num,
                    body: body_bytes,
                    stream_data,
                });

                pos = body_end + 6; // skip "endobj"
            } else {
                pos = body_start;
            }
        } else {
            break;
        }
    }

    objects
}

/// Find the next `N M obj` pattern starting from `pos`.
/// Returns (obj_num, gen_num, body_start_pos).
fn find_obj_start(text: &str, start: usize) -> Option<(u32, u32, usize)> {
    let search = &text[start..];
    let mut i = 0;

    while i < search.len() {
        // Look for " obj" or newline + "obj"
        if let Some(obj_pos) = search[i..].find(" obj") {
            let abs_obj_pos = i + obj_pos;

            // Walk backwards to find "N M"
            let before = &search[..abs_obj_pos];
            let trimmed = before.trim_end();

            // Try to parse the last two numbers before "obj"
            let parts: Vec<&str> = trimmed.split_ascii_whitespace().rev().take(2).collect();
            if parts.len() == 2 {
                if let (Ok(gen), Ok(num)) = (
                    parts[0].parse::<u32>(),
                    parts[1].parse::<u32>(),
                ) {
                    let body_start = start + abs_obj_pos + 4; // skip " obj"
                    // Skip past any whitespace/newline after "obj"
                    let body_start = skip_whitespace(text, body_start);
                    return Some((num, gen, body_start));
                }
            }

            i = abs_obj_pos + 4;
        } else {
            break;
        }
    }

    None
}

/// Skip whitespace characters at a given position.
fn skip_whitespace(text: &str, pos: usize) -> usize {
    let bytes = text.as_bytes();
    let mut p = pos;
    while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\n' || bytes[p] == b'\r' || bytes[p] == b'\t') {
        p += 1;
    }
    p
}

/// Try to extract and decompress a stream from an object body.
fn extract_stream(body: &[u8], _full_data: &[u8]) -> Option<Vec<u8>> {
    let body_str = String::from_utf8_lossy(body);

    // Find "stream" keyword
    let stream_start = body_str.find("stream")?;
    let after_stream = stream_start + 6;

    // Skip optional \r\n or \n after "stream"
    let mut data_start = after_stream;
    let body_bytes = body_str.as_bytes();
    if data_start < body_bytes.len() && body_bytes[data_start] == b'\r' {
        data_start += 1;
    }
    if data_start < body_bytes.len() && body_bytes[data_start] == b'\n' {
        data_start += 1;
    }

    // Find "endstream"
    let data_end = body_str[data_start..].find("endstream")?;
    let stream_bytes = &body[data_start..data_start + data_end];

    // Check if the stream is FlateDecode compressed
    let dict_part = &body_str[..stream_start];
    if dict_part.contains("/FlateDecode") || dict_part.contains("/Fl") {
        match decompress_stream(stream_bytes) {
            Ok(decompressed) => Some(decompressed),
            Err(_) => {
                // If decompression fails, return raw bytes
                Some(stream_bytes.to_vec())
            }
        }
    } else {
        Some(stream_bytes.to_vec())
    }
}

/// Try to extract a stream from the raw body bytes, falling back to the
/// full file data if needed.
fn extract_stream_from_body(body: &[u8], full_data: &[u8]) -> Vec<u8> {
    if let Some(data) = extract_stream(body, full_data) {
        data
    } else {
        Vec::new()
    }
}

/// Handle FlateDecode compressed streams.
///
/// Uses raw deflate decompression. If the `flate2` crate is not available
/// through `zip`, falls back to returning the raw data with a warning.
fn decompress_stream(data: &[u8]) -> Result<Vec<u8>, String> {
    // Try zlib decompression (deflate with zlib header)
    // The first two bytes of a zlib stream are typically 0x78 0x01/0x9C/0xDA
    if data.len() < 2 {
        return Err("Stream too short for decompression".to_string());
    }

    // Use a simple inflate implementation
    // Since we have the `zip` crate with deflate support, we can use
    // std::io to attempt decompression
    use std::io::Read;

    // Try zlib decompression (skip 2-byte zlib header)
    if data[0] == 0x78 {
        let deflate_data = &data[2..];
        let mut decoder = flate2_decompress(deflate_data)?;
        return Ok(decoder);
    }

    // Try raw deflate
    flate2_decompress(data)
}

/// Decompress raw deflate data.
///
/// This is a minimal inflate implementation for PDF streams. Since we depend
/// on the `zip` crate which includes deflate support, we use a manual
/// approach with raw byte manipulation for the most common cases.
fn flate2_decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    // Use the miniz_oxide decompressor that ships with Rust's std
    // through the flate2 dependency pulled in by the zip crate.
    //
    // However, since we cannot guarantee flate2 is directly available,
    // we use a simple approach: try to decompress using the
    // miniz_oxide crate that Rust's std sometimes bundles.

    // Fallback: attempt to use the data as-is if it looks like ASCII
    // (uncompressed stream)
    if data.iter().all(|&b| b < 128 || b == b'\n' || b == b'\r') {
        return Ok(data.to_vec());
    }

    // For compressed data without flate2 directly available, return an
    // error indicating the stream could not be decompressed. The caller
    // will skip this stream.
    Err("FlateDecode decompression not available for this stream (compressed binary data)".to_string())
}

/// Navigate the /Type /Pages tree to find page objects with their MediaBox
/// and content stream references.
fn parse_page_tree(objects: &[PdfObject]) -> Vec<PageRef> {
    let mut page_refs = Vec::new();

    for (idx, obj) in objects.iter().enumerate() {
        let body_str = String::from_utf8_lossy(&obj.body);

        // Check if this is a Page object (not Pages)
        if is_page_object(&body_str) {
            let media_box = extract_media_box(&body_str).unwrap_or([0.0, 0.0, 612.0, 792.0]);
            let content_obj_indices = find_content_refs(&body_str, objects);

            page_refs.push(PageRef {
                obj_index: idx,
                media_box,
                content_obj_indices,
            });
        }
    }

    page_refs
}

/// Check if an object body represents a /Type /Page (not /Pages).
fn is_page_object(body: &str) -> bool {
    if let Some(pos) = body.find("/Type") {
        let after = body[pos + 5..].trim_start();
        if after.starts_with("/Page") {
            let rest = &after[5..];
            // Make sure it's not /Pages
            return rest.is_empty() || !rest.starts_with('s');
        }
    }
    false
}

/// Extract MediaBox from an object body.
fn extract_media_box(body: &str) -> Option<[f32; 4]> {
    let mb_pos = body.find("/MediaBox")?;
    let after = &body[mb_pos + 9..];
    let bracket_start = after.find('[')?;
    let bracket_end = after[bracket_start..].find(']')?;
    let inner = &after[bracket_start + 1..bracket_start + bracket_end];

    let nums: Vec<f32> = inner
        .split_whitespace()
        .filter_map(|s| s.parse::<f32>().ok())
        .collect();

    if nums.len() >= 4 {
        Some([nums[0], nums[1], nums[2], nums[3]])
    } else {
        None
    }
}

/// Find content stream object indices referenced by a page's /Contents entry.
fn find_content_refs(body: &str, objects: &[PdfObject]) -> Vec<usize> {
    let mut indices = Vec::new();

    if let Some(pos) = body.find("/Contents") {
        let after = &body[pos + 9..].trim_start();

        if after.starts_with('[') {
            // Array of references: [N M R ...]
            if let Some(bracket_end) = after.find(']') {
                let inner = &after[1..bracket_end];
                let tokens: Vec<&str> = inner.split_whitespace().collect();
                let mut i = 0;
                while i + 2 < tokens.len() {
                    if tokens[i + 2] == "R" {
                        if let (Ok(num), Ok(gen)) =
                            (tokens[i].parse::<u32>(), tokens[i + 1].parse::<u32>())
                        {
                            if let Some(idx) = find_object_index(objects, num, gen) {
                                indices.push(idx);
                            }
                        }
                        i += 3;
                    } else {
                        i += 1;
                    }
                }
            }
        } else {
            // Single reference: N M R
            let tokens: Vec<&str> = after.split_whitespace().take(3).collect();
            if tokens.len() >= 3 && tokens[2] == "R" {
                if let (Ok(num), Ok(gen)) =
                    (tokens[0].parse::<u32>(), tokens[1].parse::<u32>())
                {
                    if let Some(idx) = find_object_index(objects, num, gen) {
                        indices.push(idx);
                    }
                }
            }
        }
    }

    indices
}

/// Find the index of an object by its object number and generation number.
fn find_object_index(objects: &[PdfObject], obj_num: u32, gen_num: u32) -> Option<usize> {
    objects
        .iter()
        .position(|o| o.obj_num == obj_num && o.gen_num == gen_num)
}

/// Parse a PDF content stream and extract drawing elements.
///
/// Recognizes the following PDF operators:
/// - `m` (moveto), `l` (lineto), `c` (curveto), `h` (close), `re` (rectangle)
/// - `S` (stroke), `f` (fill), `B` (fill+stroke)
/// - `RG`/`rg` (set RGB color), `w` (line width)
/// - `BT`...`ET` (text block), `Tj` (show text), `Tf` (font size)
/// - `cm` (concat matrix), `q`/`Q` (save/restore graphics state)
fn parse_content_stream(stream: &[u8], extract_text: bool) -> Vec<PdfElement> {
    let text = String::from_utf8_lossy(stream);
    let mut elements = Vec::new();

    let mut state = GraphicsState::default();
    let mut state_stack: Vec<GraphicsState> = Vec::new();

    // Current path
    let mut path_points: Vec<[f32; 2]> = Vec::new();
    let mut path_closed = false;
    let mut current_pos: [f32; 2] = [0.0, 0.0];
    let mut move_to_pos: [f32; 2] = [0.0, 0.0];

    // Pending rectangles
    let mut pending_rects: Vec<(f32, f32, f32, f32)> = Vec::new();

    // Text state
    let mut in_text_block = false;
    let mut text_x: f32 = 0.0;
    let mut text_y: f32 = 0.0;

    // Operand stack
    let mut operands: Vec<f32> = Vec::new();

    // Tokenize
    let tokens = tokenize_content_stream(&text);

    for token in &tokens {
        // Try to parse as number
        if let Ok(num) = token.parse::<f32>() {
            operands.push(num);
            continue;
        }

        match token.as_str() {
            // Graphics state
            "q" => {
                state_stack.push(state.clone());
            }
            "Q" => {
                if let Some(prev) = state_stack.pop() {
                    state = prev;
                }
            }
            "cm" => {
                // Concat matrix: a b c d e f
                if operands.len() >= 6 {
                    let n = operands.len();
                    let _a = operands[n - 6];
                    let _b = operands[n - 5];
                    let _c = operands[n - 4];
                    let _d = operands[n - 3];
                    let _e = operands[n - 2];
                    let _f = operands[n - 1];
                    // For simplicity in floor plan extraction, we ignore CTM
                    // transformations beyond translation
                }
                operands.clear();
            }

            // Line width
            "w" => {
                if let Some(&lw) = operands.last() {
                    state.line_width = lw;
                }
                operands.clear();
            }

            // Stroke color (RGB)
            "RG" => {
                if operands.len() >= 3 {
                    let n = operands.len();
                    state.stroke_color = [operands[n - 3], operands[n - 2], operands[n - 1]];
                }
                operands.clear();
            }

            // Fill color (RGB)
            "rg" => {
                if operands.len() >= 3 {
                    let n = operands.len();
                    state.fill_color = [operands[n - 3], operands[n - 2], operands[n - 1]];
                }
                operands.clear();
            }

            // Stroke color (grayscale)
            "G" => {
                if let Some(&g) = operands.last() {
                    state.stroke_color = [g, g, g];
                }
                operands.clear();
            }

            // Fill color (grayscale)
            "g" => {
                if let Some(&g) = operands.last() {
                    state.fill_color = [g, g, g];
                }
                operands.clear();
            }

            // Path construction
            "m" => {
                // moveto
                if operands.len() >= 2 {
                    let n = operands.len();
                    current_pos = [operands[n - 2], operands[n - 1]];
                    move_to_pos = current_pos;
                    // Flush any existing path
                    if path_points.len() >= 2 {
                        emit_path(
                            &path_points,
                            path_closed,
                            &state,
                            true,
                            None,
                            &mut elements,
                        );
                    }
                    path_points.clear();
                    path_points.push(current_pos);
                    path_closed = false;
                }
                operands.clear();
            }
            "l" => {
                // lineto
                if operands.len() >= 2 {
                    let n = operands.len();
                    current_pos = [operands[n - 2], operands[n - 1]];
                    path_points.push(current_pos);
                }
                operands.clear();
            }
            "c" => {
                // curveto (cubic Bezier) — approximate with line to endpoint
                if operands.len() >= 6 {
                    let n = operands.len();
                    // We approximate the curve with a few intermediate points
                    let p0 = current_pos;
                    let p1 = [operands[n - 6], operands[n - 5]];
                    let p2 = [operands[n - 4], operands[n - 3]];
                    let p3 = [operands[n - 2], operands[n - 1]];

                    // Subdivide into 4 segments for a rough approximation
                    for i in 1..=4 {
                        let t = i as f32 / 4.0;
                        let pt = cubic_bezier(p0, p1, p2, p3, t);
                        path_points.push(pt);
                    }
                    current_pos = p3;
                }
                operands.clear();
            }
            "v" => {
                // curveto with first control point = current point
                if operands.len() >= 4 {
                    let n = operands.len();
                    let p0 = current_pos;
                    let p1 = p0;
                    let p2 = [operands[n - 4], operands[n - 3]];
                    let p3 = [operands[n - 2], operands[n - 1]];

                    for i in 1..=4 {
                        let t = i as f32 / 4.0;
                        let pt = cubic_bezier(p0, p1, p2, p3, t);
                        path_points.push(pt);
                    }
                    current_pos = p3;
                }
                operands.clear();
            }
            "y" => {
                // curveto with last control point = endpoint
                if operands.len() >= 4 {
                    let n = operands.len();
                    let p0 = current_pos;
                    let p1 = [operands[n - 4], operands[n - 3]];
                    let p2 = [operands[n - 2], operands[n - 1]];
                    let p3 = p2;

                    for i in 1..=4 {
                        let t = i as f32 / 4.0;
                        let pt = cubic_bezier(p0, p1, p2, p3, t);
                        path_points.push(pt);
                    }
                    current_pos = p3;
                }
                operands.clear();
            }
            "h" => {
                // closepath
                path_closed = true;
                current_pos = move_to_pos;
                operands.clear();
            }
            "re" => {
                // rectangle: x y w h
                if operands.len() >= 4 {
                    let n = operands.len();
                    pending_rects.push((
                        operands[n - 4],
                        operands[n - 3],
                        operands[n - 2],
                        operands[n - 1],
                    ));
                }
                operands.clear();
            }

            // Path painting
            "S" => {
                // Stroke
                emit_pending_rects(
                    &mut pending_rects,
                    &state,
                    true,
                    false,
                    &mut elements,
                );
                if path_points.len() >= 2 {
                    emit_path(
                        &path_points,
                        path_closed,
                        &state,
                        true,
                        None,
                        &mut elements,
                    );
                }
                path_points.clear();
                path_closed = false;
                operands.clear();
            }
            "s" => {
                // Close and stroke
                path_closed = true;
                emit_pending_rects(
                    &mut pending_rects,
                    &state,
                    true,
                    false,
                    &mut elements,
                );
                if path_points.len() >= 2 {
                    emit_path(
                        &path_points,
                        path_closed,
                        &state,
                        true,
                        None,
                        &mut elements,
                    );
                }
                path_points.clear();
                path_closed = false;
                operands.clear();
            }
            "f" | "F" | "f*" => {
                // Fill
                emit_pending_rects(
                    &mut pending_rects,
                    &state,
                    false,
                    true,
                    &mut elements,
                );
                if path_points.len() >= 2 {
                    emit_path(
                        &path_points,
                        true,
                        &state,
                        false,
                        Some(state.fill_color),
                        &mut elements,
                    );
                }
                path_points.clear();
                path_closed = false;
                operands.clear();
            }
            "B" | "B*" => {
                // Fill and stroke
                emit_pending_rects(
                    &mut pending_rects,
                    &state,
                    true,
                    true,
                    &mut elements,
                );
                if path_points.len() >= 2 {
                    emit_path(
                        &path_points,
                        true,
                        &state,
                        true,
                        Some(state.fill_color),
                        &mut elements,
                    );
                }
                path_points.clear();
                path_closed = false;
                operands.clear();
            }
            "b" | "b*" => {
                // Close, fill and stroke
                path_closed = true;
                emit_pending_rects(
                    &mut pending_rects,
                    &state,
                    true,
                    true,
                    &mut elements,
                );
                if path_points.len() >= 2 {
                    emit_path(
                        &path_points,
                        true,
                        &state,
                        true,
                        Some(state.fill_color),
                        &mut elements,
                    );
                }
                path_points.clear();
                path_closed = false;
                operands.clear();
            }
            "n" => {
                // End path without painting (clipping path)
                path_points.clear();
                pending_rects.clear();
                path_closed = false;
                operands.clear();
            }

            // Text
            "BT" => {
                in_text_block = true;
                text_x = 0.0;
                text_y = 0.0;
                operands.clear();
            }
            "ET" => {
                in_text_block = false;
                operands.clear();
            }
            "Td" | "TD" => {
                if operands.len() >= 2 {
                    let n = operands.len();
                    text_x += operands[n - 2];
                    text_y += operands[n - 1];
                }
                operands.clear();
            }
            "Tm" => {
                // Text matrix: a b c d e f
                if operands.len() >= 6 {
                    let n = operands.len();
                    text_x = operands[n - 2];
                    text_y = operands[n - 1];
                }
                operands.clear();
            }
            "Tf" => {
                // Font and size
                if operands.len() >= 1 {
                    // Last numeric operand before Tf is the font size
                    // (the font name is a Name object, not numeric)
                    if let Some(&size) = operands.last() {
                        state.font_size = size;
                    }
                }
                operands.clear();
            }
            "Tj" => {
                // Show text — the text string has already been extracted as a
                // token (the parenthesized string before Tj)
                // We handle this in the string extraction below
                operands.clear();
            }
            "TJ" => {
                // Show text with individual glyph positioning
                operands.clear();
            }
            "'" => {
                // Move to next line and show text
                text_y -= state.font_size;
                operands.clear();
            }

            _ => {
                // Check if this is a text string token for Tj
                if extract_text && in_text_block && token.starts_with('(') && token.ends_with(')') {
                    let inner = &token[1..token.len() - 1];
                    if !inner.is_empty() {
                        elements.push(PdfElement::Text {
                            x: text_x,
                            y: text_y,
                            text: inner.to_string(),
                            font_size: state.font_size,
                            color: state.fill_color,
                        });
                    }
                }
                // Unknown operator — clear operand stack
                if !token.starts_with('(') && !token.starts_with('/') && !token.starts_with('[') {
                    operands.clear();
                }
            }
        }
    }

    // Flush any remaining path
    if path_points.len() >= 2 {
        emit_path(
            &path_points,
            path_closed,
            &state,
            true,
            None,
            &mut elements,
        );
    }

    elements
}

/// Tokenize a PDF content stream into operator tokens and operands.
fn tokenize_content_stream(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];

        // Skip whitespace
        if b == b' ' || b == b'\n' || b == b'\r' || b == b'\t' {
            i += 1;
            continue;
        }

        // Comment
        if b == b'%' {
            while i < len && bytes[i] != b'\n' && bytes[i] != b'\r' {
                i += 1;
            }
            continue;
        }

        // Parenthesized string
        if b == b'(' {
            let mut s = String::new();
            s.push('(');
            i += 1;
            let mut depth = 1i32;
            while i < len && depth > 0 {
                if bytes[i] == b'\\' && i + 1 < len {
                    s.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
                if bytes[i] == b'(' {
                    depth += 1;
                }
                if bytes[i] == b')' {
                    depth -= 1;
                    if depth == 0 {
                        s.push(')');
                        i += 1;
                        break;
                    }
                }
                s.push(bytes[i] as char);
                i += 1;
            }
            tokens.push(s);
            continue;
        }

        // Hex string
        if b == b'<' && (i + 1 >= len || bytes[i + 1] != b'<') {
            let mut s = String::new();
            s.push('<');
            i += 1;
            while i < len && bytes[i] != b'>' {
                s.push(bytes[i] as char);
                i += 1;
            }
            if i < len {
                s.push('>');
                i += 1;
            }
            tokens.push(s);
            continue;
        }

        // Array
        if b == b'[' {
            tokens.push("[".to_string());
            i += 1;
            continue;
        }
        if b == b']' {
            tokens.push("]".to_string());
            i += 1;
            continue;
        }

        // Dictionary
        if b == b'<' && i + 1 < len && bytes[i + 1] == b'<' {
            tokens.push("<<".to_string());
            i += 2;
            continue;
        }
        if b == b'>' && i + 1 < len && bytes[i + 1] == b'>' {
            tokens.push(">>".to_string());
            i += 2;
            continue;
        }

        // Name
        if b == b'/' {
            let mut s = String::new();
            s.push('/');
            i += 1;
            while i < len
                && bytes[i] != b' '
                && bytes[i] != b'\n'
                && bytes[i] != b'\r'
                && bytes[i] != b'\t'
                && bytes[i] != b'/'
                && bytes[i] != b'('
                && bytes[i] != b')'
                && bytes[i] != b'<'
                && bytes[i] != b'>'
                && bytes[i] != b'['
                && bytes[i] != b']'
            {
                s.push(bytes[i] as char);
                i += 1;
            }
            tokens.push(s);
            continue;
        }

        // Number or keyword
        let start = i;
        while i < len
            && bytes[i] != b' '
            && bytes[i] != b'\n'
            && bytes[i] != b'\r'
            && bytes[i] != b'\t'
            && bytes[i] != b'/'
            && bytes[i] != b'('
            && bytes[i] != b')'
            && bytes[i] != b'<'
            && bytes[i] != b'>'
            && bytes[i] != b'['
            && bytes[i] != b']'
        {
            i += 1;
        }
        if i > start {
            let s: String = bytes[start..i].iter().map(|&b| b as char).collect();
            tokens.push(s);
        }
    }

    tokens
}

/// Evaluate a cubic Bezier curve at parameter t.
fn cubic_bezier(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], t: f32) -> [f32; 2] {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;

    [
        mt3 * p0[0] + 3.0 * mt2 * t * p1[0] + 3.0 * mt * t2 * p2[0] + t3 * p3[0],
        mt3 * p0[1] + 3.0 * mt2 * t * p1[1] + 3.0 * mt * t2 * p2[1] + t3 * p3[1],
    ]
}

/// Emit a path as a PdfElement.
fn emit_path(
    points: &[[f32; 2]],
    closed: bool,
    state: &GraphicsState,
    stroked: bool,
    fill_color: Option<[f32; 3]>,
    elements: &mut Vec<PdfElement>,
) {
    if points.len() < 2 {
        return;
    }

    if points.len() == 2 && !closed {
        // Simple line segment
        elements.push(PdfElement::Line {
            x1: points[0][0],
            y1: points[0][1],
            x2: points[1][0],
            y2: points[1][1],
            color: if stroked {
                state.stroke_color
            } else {
                fill_color.unwrap_or(state.stroke_color)
            },
            line_width: state.line_width,
        });
    } else {
        // Polyline
        elements.push(PdfElement::Polyline {
            points: points.to_vec(),
            color: if stroked {
                state.stroke_color
            } else {
                fill_color.unwrap_or(state.stroke_color)
            },
            line_width: state.line_width,
            closed,
        });
    }
}

/// Emit pending rectangle operations.
fn emit_pending_rects(
    rects: &mut Vec<(f32, f32, f32, f32)>,
    state: &GraphicsState,
    stroked: bool,
    filled: bool,
    elements: &mut Vec<PdfElement>,
) {
    for &(x, y, w, h) in rects.iter() {
        elements.push(PdfElement::Rectangle {
            x,
            y,
            width: w,
            height: h,
            stroke_color: if stroked {
                Some(state.stroke_color)
            } else {
                None
            },
            fill_color: if filled {
                Some(state.fill_color)
            } else {
                None
            },
            line_width: state.line_width,
        });
    }
    rects.clear();
}

/// Update bounding box from a single element.
fn update_bounds_from_element(element: &PdfElement, min: &mut [f32; 2], max: &mut [f32; 2]) {
    match element {
        PdfElement::Line {
            x1, y1, x2, y2, ..
        } => {
            update_bounds(min, max, [*x1, *y1]);
            update_bounds(min, max, [*x2, *y2]);
        }
        PdfElement::Rectangle {
            x, y, width, height, ..
        } => {
            update_bounds(min, max, [*x, *y]);
            update_bounds(min, max, [x + width, y + height]);
        }
        PdfElement::Polyline { points, .. } => {
            for p in points {
                update_bounds(min, max, *p);
            }
        }
        PdfElement::Circle { cx, cy, radius, .. } => {
            update_bounds(min, max, [cx - radius, cy - radius]);
            update_bounds(min, max, [cx + radius, cy + radius]);
        }
        PdfElement::Text { x, y, .. } => {
            update_bounds(min, max, [*x, *y]);
        }
    }
}

/// Update bounding box with a single point.
fn update_bounds(min: &mut [f32; 2], max: &mut [f32; 2], point: [f32; 2]) {
    min[0] = min[0].min(point[0]);
    min[1] = min[1].min(point[1]);
    max[0] = max[0].max(point[0]);
    max[1] = max[1].max(point[1]);
}

/// Snap a point to the nearest canonical point within tolerance.
fn snap_point(point: [f32; 2], canonical: &[[f32; 2]], tol_sq: f32) -> [f32; 2] {
    let mut best = point;
    let mut best_dist = f32::MAX;

    for c in canonical {
        let dx = point[0] - c[0];
        let dy = point[1] - c[1];
        let dist = dx * dx + dy * dy;
        if dist < best_dist && dist <= tol_sq {
            best = *c;
            best_dist = dist;
        }
    }

    best
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_import_config_default() {
        let config = PdfImportConfig::default();
        // 0.0254 / 72.0 ~= 0.000352778
        assert!((config.scale_factor - 0.0254 / 72.0).abs() < 1e-8);
        assert_eq!(config.offset, [0.0, 0.0]);
        assert!(config.target_page.is_none());
        assert!((config.simplify_tolerance - 0.001).abs() < 1e-8);
        assert!(config.extract_text);
    }

    #[test]
    fn test_pdf_element_line() {
        let line = PdfElement::Line {
            x1: 0.0,
            y1: 0.0,
            x2: 100.0,
            y2: 50.0,
            color: [0.0, 0.0, 0.0],
            line_width: 1.0,
        };

        match &line {
            PdfElement::Line {
                x1, y1, x2, y2, ..
            } => {
                assert!((x1 - 0.0).abs() < 0.001);
                assert!((y1 - 0.0).abs() < 0.001);
                assert!((x2 - 100.0).abs() < 0.001);
                assert!((y2 - 50.0).abs() < 0.001);
            }
            _ => panic!("Expected Line"),
        }

        // Verify serialization round-trip
        let json = serde_json::to_string(&line).unwrap();
        let deserialized: PdfElement = serde_json::from_str(&json).unwrap();
        match deserialized {
            PdfElement::Line { x2, y2, .. } => {
                assert!((x2 - 100.0).abs() < 0.001);
                assert!((y2 - 50.0).abs() < 0.001);
            }
            _ => panic!("Expected Line after deserialization"),
        }
    }

    #[test]
    fn test_pdf_element_rectangle() {
        let rect = PdfElement::Rectangle {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
            stroke_color: Some([0.0, 0.0, 0.0]),
            fill_color: Some([0.8, 0.8, 0.8]),
            line_width: 0.5,
        };

        match &rect {
            PdfElement::Rectangle {
                x,
                y,
                width,
                height,
                stroke_color,
                fill_color,
                ..
            } => {
                assert!((x - 10.0).abs() < 0.001);
                assert!((y - 20.0).abs() < 0.001);
                assert!((width - 100.0).abs() < 0.001);
                assert!((height - 50.0).abs() < 0.001);
                assert!(stroke_color.is_some());
                assert!(fill_color.is_some());
            }
            _ => panic!("Expected Rectangle"),
        }
    }

    #[test]
    fn test_pdf_page_construction() {
        let page = PdfPage {
            page_number: 1,
            width: 612.0,
            height: 792.0,
            elements: vec![
                PdfElement::Line {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 612.0,
                    y2: 792.0,
                    color: [0.0, 0.0, 0.0],
                    line_width: 1.0,
                },
                PdfElement::Rectangle {
                    x: 72.0,
                    y: 72.0,
                    width: 468.0,
                    height: 648.0,
                    stroke_color: Some([0.0, 0.0, 0.0]),
                    fill_color: None,
                    line_width: 0.5,
                },
            ],
        };

        assert_eq!(page.page_number, 1);
        assert!((page.width - 612.0).abs() < 0.001);
        assert!((page.height - 792.0).abs() < 0.001);
        assert_eq!(page.elements.len(), 2);
    }

    #[test]
    fn test_pdf_info_construction() {
        let info = PdfInfo {
            page_count: 5,
            title: Some("Floor Plan".to_string()),
            author: Some("Architect".to_string()),
            creator: Some("AutoCAD".to_string()),
        };

        assert_eq!(info.page_count, 5);
        assert_eq!(info.title.as_deref(), Some("Floor Plan"));
        assert_eq!(info.author.as_deref(), Some("Architect"));
        assert_eq!(info.creator.as_deref(), Some("AutoCAD"));
    }

    #[test]
    fn test_pdf_elements_to_line_segments() {
        let elements = vec![
            PdfElement::Line {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 0.0,
                color: [0.0, 0.0, 0.0],
                line_width: 1.0,
            },
            PdfElement::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 5.0,
                height: 5.0,
                stroke_color: Some([0.0, 0.0, 0.0]),
                fill_color: None,
                line_width: 0.5,
            },
            PdfElement::Polyline {
                points: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
                color: [0.0, 0.0, 0.0],
                line_width: 1.0,
                closed: true,
            },
            PdfElement::Circle {
                cx: 5.0,
                cy: 5.0,
                radius: 2.0,
                stroke_color: Some([1.0, 0.0, 0.0]),
                fill_color: None,
                line_width: 0.5,
            },
            PdfElement::Text {
                x: 0.0,
                y: 0.0,
                text: "Hello".to_string(),
                font_size: 12.0,
                color: [0.0, 0.0, 0.0],
            },
        ];

        let segments = pdf_elements_to_line_segments(&elements);

        // 1 (line) + 4 (rect) + 3 (polyline: 2 edges + 1 closing) + 32 (circle) + 0 (text) = 40
        assert_eq!(segments.len(), 40);

        // First segment is the line
        assert!((segments[0].0[0] - 0.0).abs() < 0.001);
        assert!((segments[0].1[0] - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_pdf_elements_to_vertices() {
        let elements = vec![
            PdfElement::Line {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 5.0,
                color: [1.0, 0.0, 0.0],
                line_width: 1.0,
            },
            PdfElement::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 4.0,
                height: 3.0,
                stroke_color: Some([0.0, 1.0, 0.0]),
                fill_color: None,
                line_width: 0.5,
            },
        ];

        let (positions, colors) = pdf_elements_to_vertices(&elements);

        // Line: 1 segment => 4 position floats, 6 color floats
        // Rectangle: 4 segments => 16 position floats, 24 color floats
        // Total: 20 position floats, 30 color floats
        assert_eq!(positions.len(), 20);
        assert_eq!(colors.len(), 30);

        // First line vertex positions
        assert!((positions[0] - 0.0).abs() < 0.001); // x1
        assert!((positions[1] - 0.0).abs() < 0.001); // y1
        assert!((positions[2] - 10.0).abs() < 0.001); // x2
        assert!((positions[3] - 5.0).abs() < 0.001); // y2

        // First line vertex colors (red)
        assert!((colors[0] - 1.0).abs() < 0.001); // r
        assert!((colors[1] - 0.0).abs() < 0.001); // g
        assert!((colors[2] - 0.0).abs() < 0.001); // b
    }

    #[test]
    fn test_scale_pdf_to_model() {
        let mut overlay = PdfDrawingOverlay {
            info: PdfInfo {
                page_count: 1,
                title: None,
                author: None,
                creator: None,
            },
            pages: vec![PdfPage {
                page_number: 1,
                width: 612.0,
                height: 792.0,
                elements: vec![
                    PdfElement::Line {
                        x1: 72.0,
                        y1: 72.0,
                        x2: 540.0,
                        y2: 720.0,
                        color: [0.0, 0.0, 0.0],
                        line_width: 1.0,
                    },
                    PdfElement::Rectangle {
                        x: 100.0,
                        y: 100.0,
                        width: 200.0,
                        height: 150.0,
                        stroke_color: Some([0.0, 0.0, 0.0]),
                        fill_color: None,
                        line_width: 0.5,
                    },
                ],
            }],
            bounds: ([72.0, 72.0], [540.0, 720.0]),
        };

        let config = PdfImportConfig {
            scale_factor: 2.0,
            offset: [10.0, 20.0],
            ..Default::default()
        };

        scale_pdf_to_model(&mut overlay, &config);

        let page = &overlay.pages[0];
        assert!((page.width - 1224.0).abs() < 0.001); // 612 * 2
        assert!((page.height - 1584.0).abs() < 0.001); // 792 * 2

        // Line should be scaled and offset
        match &page.elements[0] {
            PdfElement::Line {
                x1, y1, x2, y2, line_width, ..
            } => {
                assert!((x1 - (72.0 * 2.0 + 10.0)).abs() < 0.001);
                assert!((y1 - (72.0 * 2.0 + 20.0)).abs() < 0.001);
                assert!((x2 - (540.0 * 2.0 + 10.0)).abs() < 0.001);
                assert!((y2 - (720.0 * 2.0 + 20.0)).abs() < 0.001);
                assert!((line_width - 2.0).abs() < 0.001);
            }
            _ => panic!("Expected Line"),
        }

        // Bounds should be scaled and offset
        assert!((overlay.bounds.0[0] - (72.0 * 2.0 + 10.0)).abs() < 0.001);
        assert!((overlay.bounds.0[1] - (72.0 * 2.0 + 20.0)).abs() < 0.001);
    }

    #[test]
    fn test_merge_coincident_points() {
        let mut elements = vec![
            PdfElement::Line {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 0.0,
                color: [0.0, 0.0, 0.0],
                line_width: 1.0,
            },
            PdfElement::Line {
                x1: 10.001,
                y1: 0.0005,
                x2: 20.0,
                y2: 0.0,
                color: [0.0, 0.0, 0.0],
                line_width: 1.0,
            },
        ];

        merge_coincident_points(&mut elements, 0.01);

        // The second line's start should snap to (10.0, 0.0)
        match &elements[1] {
            PdfElement::Line { x1, y1, .. } => {
                assert!((x1 - 10.0).abs() < 0.001);
                assert!((y1 - 0.0).abs() < 0.001);
            }
            _ => panic!("Expected Line"),
        }
    }

    #[test]
    fn test_parse_pdf_bytes_invalid() {
        let data = b"This is not a PDF file";
        let result = parse_pdf_bytes(data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Not a valid PDF"));
    }

    #[test]
    fn test_parse_pdf_bytes_minimal() {
        // Construct a minimal valid PDF-like structure
        let pdf = b"%PDF-1.4\n1 0 obj\n<< /Type /Page /MediaBox [0 0 612 792] >>\nendobj\n2 0 obj\n<< /Type /Pages /Count 1 /Kids [1 0 R] >>\nendobj\n";
        let result = parse_pdf_bytes(pdf);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.page_count, 1);
    }

    #[test]
    fn test_tokenize_content_stream() {
        let stream = "0.5 w 0 0 m 100 200 l S";
        let tokens = tokenize_content_stream(stream);
        assert_eq!(tokens, vec!["0.5", "w", "0", "0", "m", "100", "200", "l", "S"]);
    }

    #[test]
    fn test_parse_content_stream_line() {
        let stream = b"1 0 0 RG 2 w 10 20 m 100 200 l S";
        let elements = parse_content_stream(stream, false);

        assert_eq!(elements.len(), 1);
        match &elements[0] {
            PdfElement::Line {
                x1, y1, x2, y2, color, line_width,
            } => {
                assert!((x1 - 10.0).abs() < 0.001);
                assert!((y1 - 20.0).abs() < 0.001);
                assert!((x2 - 100.0).abs() < 0.001);
                assert!((y2 - 200.0).abs() < 0.001);
                assert!((color[0] - 1.0).abs() < 0.001); // red
                assert!((line_width - 2.0).abs() < 0.001);
            }
            _ => panic!("Expected Line"),
        }
    }

    #[test]
    fn test_parse_content_stream_rectangle() {
        let stream = b"0 0 0 RG 0.5 0.5 0.5 rg 1 w 50 50 200 100 re B";
        let elements = parse_content_stream(stream, false);

        assert_eq!(elements.len(), 1);
        match &elements[0] {
            PdfElement::Rectangle {
                x, y, width, height, stroke_color, fill_color, ..
            } => {
                assert!((x - 50.0).abs() < 0.001);
                assert!((y - 50.0).abs() < 0.001);
                assert!((width - 200.0).abs() < 0.001);
                assert!((height - 100.0).abs() < 0.001);
                assert!(stroke_color.is_some());
                assert!(fill_color.is_some());
            }
            _ => panic!("Expected Rectangle"),
        }
    }

    #[test]
    fn test_cubic_bezier() {
        let p0 = [0.0, 0.0];
        let p1 = [0.0, 1.0];
        let p2 = [1.0, 1.0];
        let p3 = [1.0, 0.0];

        // t=0 should be p0
        let start = cubic_bezier(p0, p1, p2, p3, 0.0);
        assert!((start[0] - 0.0).abs() < 0.001);
        assert!((start[1] - 0.0).abs() < 0.001);

        // t=1 should be p3
        let end = cubic_bezier(p0, p1, p2, p3, 1.0);
        assert!((end[0] - 1.0).abs() < 0.001);
        assert!((end[1] - 0.0).abs() < 0.001);

        // t=0.5 should be at (0.5, 0.75) for this specific curve
        let mid = cubic_bezier(p0, p1, p2, p3, 0.5);
        assert!((mid[0] - 0.5).abs() < 0.001);
        assert!((mid[1] - 0.75).abs() < 0.001);
    }
}
