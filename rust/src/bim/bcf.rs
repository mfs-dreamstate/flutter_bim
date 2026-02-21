//! BCF (BIM Collaboration Format) Support
//!
//! Handles BCF 2.1 (XML/ZIP) and BCF 3.0 (JSON) import/export.
//! BCF is a standardized format for BIM issue tracking and collaboration,
//! consisting of ZIP archives containing XML markup and viewpoint files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Status of a BCF topic (issue).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BcfTopicStatus {
    Open,
    InProgress,
    Closed,
    ReOpened,
}

impl std::fmt::Display for BcfTopicStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BcfTopicStatus::Open => write!(f, "Open"),
            BcfTopicStatus::InProgress => write!(f, "InProgress"),
            BcfTopicStatus::Closed => write!(f, "Closed"),
            BcfTopicStatus::ReOpened => write!(f, "ReOpened"),
        }
    }
}

impl std::str::FromStr for BcfTopicStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Open" => Ok(BcfTopicStatus::Open),
            "InProgress" | "In Progress" => Ok(BcfTopicStatus::InProgress),
            "Closed" => Ok(BcfTopicStatus::Closed),
            "ReOpened" | "Reopened" => Ok(BcfTopicStatus::ReOpened),
            _ => Err(format!("Unknown BCF topic status: '{}'", s)),
        }
    }
}

/// Priority of a BCF topic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BcfPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl std::fmt::Display for BcfPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BcfPriority::Low => write!(f, "Low"),
            BcfPriority::Normal => write!(f, "Normal"),
            BcfPriority::High => write!(f, "High"),
            BcfPriority::Critical => write!(f, "Critical"),
        }
    }
}

impl std::str::FromStr for BcfPriority {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Low" => Ok(BcfPriority::Low),
            "Normal" => Ok(BcfPriority::Normal),
            "High" => Ok(BcfPriority::High),
            "Critical" => Ok(BcfPriority::Critical),
            _ => Err(format!("Unknown BCF priority: '{}'", s)),
        }
    }
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A BCF project containing multiple topics (issues).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BcfProject {
    pub project_id: String,
    pub name: String,
    pub topics: Vec<BcfTopic>,
}

/// A single BCF topic (issue/clash/request).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BcfTopic {
    pub guid: String,
    pub title: String,
    pub description: String,
    pub status: BcfTopicStatus,
    pub priority: BcfPriority,
    pub topic_type: String,
    pub creation_date: String,
    pub creation_author: String,
    pub modified_date: Option<String>,
    pub assigned_to: Option<String>,
    pub comments: Vec<BcfComment>,
    pub viewpoints: Vec<BcfViewpoint>,
}

/// A comment on a BCF topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BcfComment {
    pub guid: String,
    pub date: String,
    pub author: String,
    pub text: String,
    pub viewpoint_guid: Option<String>,
}

/// A BCF viewpoint capturing camera state, selection, and visibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BcfViewpoint {
    pub guid: String,
    pub camera_position: [f32; 3],
    pub camera_direction: [f32; 3],
    pub camera_up: [f32; 3],
    pub field_of_view: f32,
    pub is_orthographic: bool,
    pub selected_components: Vec<String>,
    pub hidden_components: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_png: Option<Vec<u8>>,
}

// ---------------------------------------------------------------------------
// BCF 2.1 ZIP export
// ---------------------------------------------------------------------------

/// Serialize a `BcfProject` to BCF 2.1 ZIP bytes.
pub fn export_bcf_zip(project: &BcfProject) -> Result<Vec<u8>, String> {
    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = zip::ZipWriter::new(cursor);

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // --- bcf.version ---
    zip.start_file("bcf.version", options)
        .map_err(|e| format!("Failed to create bcf.version: {}", e))?;
    let version_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<Version VersionId="2.1" DetailedVersion="2.1"/>
"#;
    zip.write_all(version_xml)
        .map_err(|e| format!("Failed to write bcf.version: {}", e))?;

    // --- Topics ---
    for topic in &project.topics {
        let prefix = &topic.guid;

        // markup.bcf
        let markup = write_markup_xml(topic)?;
        let markup_path = format!("{}/markup.bcf", prefix);
        zip.start_file(&markup_path, options)
            .map_err(|e| format!("Failed to create {}: {}", markup_path, e))?;
        zip.write_all(&markup)
            .map_err(|e| format!("Failed to write {}: {}", markup_path, e))?;

        // viewpoints
        for (i, vp) in topic.viewpoints.iter().enumerate() {
            let vp_xml = write_viewpoint_xml(vp)?;
            let vp_filename = if i == 0 {
                "viewpoint.bcfv".to_string()
            } else {
                format!("viewpoint_{}.bcfv", i)
            };
            let vp_path = format!("{}/{}", prefix, vp_filename);
            zip.start_file(&vp_path, options)
                .map_err(|e| format!("Failed to create {}: {}", vp_path, e))?;
            zip.write_all(&vp_xml)
                .map_err(|e| format!("Failed to write {}: {}", vp_path, e))?;

            // snapshot.png
            if let Some(png_data) = &vp.snapshot_png {
                let snap_filename = if i == 0 {
                    "snapshot.png".to_string()
                } else {
                    format!("snapshot_{}.png", i)
                };
                let snap_path = format!("{}/{}", prefix, snap_filename);
                zip.start_file(&snap_path, options)
                    .map_err(|e| format!("Failed to create {}: {}", snap_path, e))?;
                zip.write_all(png_data)
                    .map_err(|e| format!("Failed to write {}: {}", snap_path, e))?;
            }
        }
    }

    let cursor = zip
        .finish()
        .map_err(|e| format!("Failed to finalize ZIP: {}", e))?;
    Ok(cursor.into_inner())
}

// ---------------------------------------------------------------------------
// BCF 2.1 ZIP import
// ---------------------------------------------------------------------------

/// Parse BCF 2.1 ZIP bytes into a `BcfProject`.
pub fn import_bcf_zip(data: &[u8]) -> Result<BcfProject, String> {
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| format!("Failed to open BCF ZIP: {}", e))?;

    // Discover topic GUIDs by looking for <guid>/markup.bcf entries
    let mut topic_guids: Vec<String> = Vec::new();
    let mut file_contents: HashMap<String, Vec<u8>> = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read ZIP entry {}: {}", i, e))?;
        let name = file.name().to_string();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|e| format!("Failed to read {}: {}", name, e))?;

        // Detect topic folders
        if name.ends_with("/markup.bcf") {
            let guid = name.trim_end_matches("/markup.bcf").to_string();
            if !guid.is_empty() {
                topic_guids.push(guid);
            }
        }

        file_contents.insert(name, contents);
    }

    // Parse each topic
    let mut topics = Vec::new();
    for guid in &topic_guids {
        let markup_key = format!("{}/markup.bcf", guid);
        let markup_data = file_contents
            .get(&markup_key)
            .ok_or_else(|| format!("Missing markup.bcf for topic {}", guid))?;

        let (mut topic, viewpoint_guids) = parse_markup_xml(markup_data)?;

        // Override the topic GUID with the folder name (canonical source)
        topic.guid = guid.clone();

        // Parse viewpoints referenced in markup
        for vp_guid in &viewpoint_guids {
            // Try standard names first, then numbered names
            let vp_key_standard = format!("{}/viewpoint.bcfv", guid);
            let vp_key_named = format!("{}/{}.bcfv", guid, vp_guid);

            let vp_data = file_contents
                .get(&vp_key_standard)
                .or_else(|| file_contents.get(&vp_key_named));

            if let Some(data) = vp_data {
                let mut vp = parse_viewpoint_xml(data)?;

                // Try to load corresponding snapshot
                let snap_key = format!("{}/snapshot.png", guid);
                if let Some(png_data) = file_contents.get(&snap_key) {
                    vp.snapshot_png = Some(png_data.clone());
                }

                topic.viewpoints.push(vp);
            }
        }

        topics.push(topic);
    }

    Ok(BcfProject {
        project_id: uuid::Uuid::new_v4().to_string(),
        name: "Imported BCF".to_string(),
        topics,
    })
}

// ---------------------------------------------------------------------------
// BCF 3.0 JSON export / import
// ---------------------------------------------------------------------------

/// Export a `BcfProject` as BCF 3.0 JSON.
pub fn export_bcf_json(project: &BcfProject) -> Result<String, String> {
    serde_json::to_string_pretty(project).map_err(|e| format!("JSON serialization failed: {}", e))
}

/// Import a `BcfProject` from BCF 3.0 JSON.
pub fn import_bcf_json(json: &str) -> Result<BcfProject, String> {
    serde_json::from_str(json).map_err(|e| format!("JSON deserialization failed: {}", e))
}

// ---------------------------------------------------------------------------
// XML helpers — write
// ---------------------------------------------------------------------------

/// Generate markup.bcf XML bytes for a single topic.
pub fn write_markup_xml(topic: &BcfTopic) -> Result<Vec<u8>, String> {
    use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
    use quick_xml::Writer;

    let mut buf = Vec::new();
    let mut writer = Writer::new(Cursor::new(&mut buf));

    // XML declaration
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(|e| format!("XML write error: {}", e))?;

    // <Markup>
    writer
        .write_event(Event::Start(BytesStart::new("Markup")))
        .map_err(|e| format!("XML write error: {}", e))?;

    // <Topic>
    {
        let mut topic_elem = BytesStart::new("Topic");
        topic_elem.push_attribute(("Guid", topic.guid.as_str()));
        topic_elem.push_attribute(("TopicType", topic.topic_type.as_str()));
        topic_elem.push_attribute(("TopicStatus", topic.status.to_string().as_str()));
        writer
            .write_event(Event::Start(topic_elem))
            .map_err(|e| format!("XML write error: {}", e))?;

        write_xml_text_element(&mut writer, "Title", &topic.title)?;
        write_xml_text_element(&mut writer, "Description", &topic.description)?;
        write_xml_text_element(&mut writer, "CreationDate", &topic.creation_date)?;
        write_xml_text_element(&mut writer, "CreationAuthor", &topic.creation_author)?;

        if let Some(ref modified) = topic.modified_date {
            write_xml_text_element(&mut writer, "ModifiedDate", modified)?;
        }

        write_xml_text_element(&mut writer, "Priority", &topic.priority.to_string())?;

        if let Some(ref assigned) = topic.assigned_to {
            write_xml_text_element(&mut writer, "AssignedTo", assigned)?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("Topic")))
            .map_err(|e| format!("XML write error: {}", e))?;
    }

    // <Comment> elements
    for comment in &topic.comments {
        let mut comment_elem = BytesStart::new("Comment");
        comment_elem.push_attribute(("Guid", comment.guid.as_str()));
        writer
            .write_event(Event::Start(comment_elem))
            .map_err(|e| format!("XML write error: {}", e))?;

        write_xml_text_element(&mut writer, "Date", &comment.date)?;
        write_xml_text_element(&mut writer, "Author", &comment.author)?;
        write_xml_text_element(&mut writer, "Comment", &comment.text)?;

        if let Some(ref vp_guid) = comment.viewpoint_guid {
            let mut vp_ref = BytesStart::new("Viewpoint");
            vp_ref.push_attribute(("Guid", vp_guid.as_str()));
            writer
                .write_event(Event::Empty(vp_ref))
                .map_err(|e| format!("XML write error: {}", e))?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("Comment")))
            .map_err(|e| format!("XML write error: {}", e))?;
    }

    // <Viewpoints> elements
    for (i, vp) in topic.viewpoints.iter().enumerate() {
        let mut vp_elem = BytesStart::new("Viewpoints");
        vp_elem.push_attribute(("Guid", vp.guid.as_str()));
        writer
            .write_event(Event::Start(vp_elem))
            .map_err(|e| format!("XML write error: {}", e))?;

        let vp_filename = if i == 0 {
            "viewpoint.bcfv".to_string()
        } else {
            format!("viewpoint_{}.bcfv", i)
        };
        write_xml_text_element(&mut writer, "Viewpoint", &vp_filename)?;

        if vp.snapshot_png.is_some() {
            let snap_filename = if i == 0 {
                "snapshot.png".to_string()
            } else {
                format!("snapshot_{}.png", i)
            };
            write_xml_text_element(&mut writer, "Snapshot", &snap_filename)?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("Viewpoints")))
            .map_err(|e| format!("XML write error: {}", e))?;
    }

    // </Markup>
    writer
        .write_event(Event::End(BytesEnd::new("Markup")))
        .map_err(|e| format!("XML write error: {}", e))?;

    Ok(buf)
}

/// Generate viewpoint.bcfv XML bytes for a single viewpoint.
pub fn write_viewpoint_xml(vp: &BcfViewpoint) -> Result<Vec<u8>, String> {
    use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
    use quick_xml::Writer;

    let mut buf = Vec::new();
    let mut writer = Writer::new(Cursor::new(&mut buf));

    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(|e| format!("XML write error: {}", e))?;

    let mut vi_elem = BytesStart::new("VisualizationInfo");
    vi_elem.push_attribute(("Guid", vp.guid.as_str()));
    writer
        .write_event(Event::Start(vi_elem))
        .map_err(|e| format!("XML write error: {}", e))?;

    // Camera
    if vp.is_orthographic {
        writer
            .write_event(Event::Start(BytesStart::new("OrthogonalCamera")))
            .map_err(|e| format!("XML write error: {}", e))?;
    } else {
        writer
            .write_event(Event::Start(BytesStart::new("PerspectiveCamera")))
            .map_err(|e| format!("XML write error: {}", e))?;
    }

    // CameraViewPoint
    write_xyz_element(&mut writer, "CameraViewPoint", vp.camera_position)?;
    // CameraDirection
    write_xyz_element(&mut writer, "CameraDirection", vp.camera_direction)?;
    // CameraUpVector
    write_xyz_element(&mut writer, "CameraUpVector", vp.camera_up)?;

    if vp.is_orthographic {
        write_xml_text_element(
            &mut writer,
            "ViewToWorldScale",
            &format!("{}", vp.field_of_view),
        )?;
        writer
            .write_event(Event::End(BytesEnd::new("OrthogonalCamera")))
            .map_err(|e| format!("XML write error: {}", e))?;
    } else {
        write_xml_text_element(
            &mut writer,
            "FieldOfView",
            &format!("{}", vp.field_of_view),
        )?;
        writer
            .write_event(Event::End(BytesEnd::new("PerspectiveCamera")))
            .map_err(|e| format!("XML write error: {}", e))?;
    }

    // Components
    let has_components =
        !vp.selected_components.is_empty() || !vp.hidden_components.is_empty();
    if has_components {
        writer
            .write_event(Event::Start(BytesStart::new("Components")))
            .map_err(|e| format!("XML write error: {}", e))?;

        // Selection
        if !vp.selected_components.is_empty() {
            writer
                .write_event(Event::Start(BytesStart::new("Selection")))
                .map_err(|e| format!("XML write error: {}", e))?;
            for ifc_guid in &vp.selected_components {
                let mut comp = BytesStart::new("Component");
                comp.push_attribute(("IfcGuid", ifc_guid.as_str()));
                writer
                    .write_event(Event::Empty(comp))
                    .map_err(|e| format!("XML write error: {}", e))?;
            }
            writer
                .write_event(Event::End(BytesEnd::new("Selection")))
                .map_err(|e| format!("XML write error: {}", e))?;
        }

        // Visibility (exceptions = hidden components)
        if !vp.hidden_components.is_empty() {
            let mut vis_elem = BytesStart::new("Visibility");
            vis_elem.push_attribute(("DefaultVisibility", "true"));
            writer
                .write_event(Event::Start(vis_elem))
                .map_err(|e| format!("XML write error: {}", e))?;

            writer
                .write_event(Event::Start(BytesStart::new("Exceptions")))
                .map_err(|e| format!("XML write error: {}", e))?;
            for ifc_guid in &vp.hidden_components {
                let mut comp = BytesStart::new("Component");
                comp.push_attribute(("IfcGuid", ifc_guid.as_str()));
                writer
                    .write_event(Event::Empty(comp))
                    .map_err(|e| format!("XML write error: {}", e))?;
            }
            writer
                .write_event(Event::End(BytesEnd::new("Exceptions")))
                .map_err(|e| format!("XML write error: {}", e))?;
            writer
                .write_event(Event::End(BytesEnd::new("Visibility")))
                .map_err(|e| format!("XML write error: {}", e))?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("Components")))
            .map_err(|e| format!("XML write error: {}", e))?;
    }

    // </VisualizationInfo>
    writer
        .write_event(Event::End(BytesEnd::new("VisualizationInfo")))
        .map_err(|e| format!("XML write error: {}", e))?;

    Ok(buf)
}

// ---------------------------------------------------------------------------
// XML helpers — parse
// ---------------------------------------------------------------------------

/// Parse markup.bcf XML into a `BcfTopic` and a list of viewpoint GUIDs.
///
/// Returns `(topic, viewpoint_guids)` where `viewpoint_guids` are the GUIDs
/// of viewpoints referenced in the markup file (actual viewpoint data must be
/// parsed separately from the .bcfv files).
pub fn parse_markup_xml(data: &[u8]) -> Result<(BcfTopic, Vec<String>), String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(data);
    reader.config_mut().trim_text(true);

    let mut topic = BcfTopic {
        guid: String::new(),
        title: String::new(),
        description: String::new(),
        status: BcfTopicStatus::Open,
        priority: BcfPriority::Normal,
        topic_type: "Issue".to_string(),
        creation_date: String::new(),
        creation_author: String::new(),
        modified_date: None,
        assigned_to: None,
        comments: Vec::new(),
        viewpoints: Vec::new(),
    };

    let mut viewpoint_guids: Vec<String> = Vec::new();
    let mut buf = Vec::new();

    // Current parsing context
    let mut in_topic = false;
    let mut in_comment = false;
    let mut current_comment = BcfComment {
        guid: String::new(),
        date: String::new(),
        author: String::new(),
        text: String::new(),
        viewpoint_guid: None,
    };
    let mut current_tag = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                current_tag = tag_name.clone();

                match tag_name.as_str() {
                    "Topic" => {
                        in_topic = true;
                        for attr in e.attributes().flatten() {
                            let key =
                                String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let val =
                                String::from_utf8_lossy(&attr.value).to_string();
                            match key.as_str() {
                                "Guid" => topic.guid = val,
                                "TopicType" => topic.topic_type = val,
                                "TopicStatus" => {
                                    topic.status =
                                        val.parse().unwrap_or(BcfTopicStatus::Open);
                                }
                                _ => {}
                            }
                        }
                    }
                    "Comment" if !in_topic => {
                        in_comment = true;
                        current_comment = BcfComment {
                            guid: String::new(),
                            date: String::new(),
                            author: String::new(),
                            text: String::new(),
                            viewpoint_guid: None,
                        };
                        for attr in e.attributes().flatten() {
                            let key =
                                String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let val =
                                String::from_utf8_lossy(&attr.value).to_string();
                            if key == "Guid" {
                                current_comment.guid = val;
                            }
                        }
                    }
                    "Viewpoints" => {
                        for attr in e.attributes().flatten() {
                            let key =
                                String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let val =
                                String::from_utf8_lossy(&attr.value).to_string();
                            if key == "Guid" {
                                viewpoint_guids.push(val);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e
                    .unescape()
                    .map_err(|err| format!("XML unescape error: {}", err))?
                    .to_string();

                if in_comment {
                    match current_tag.as_str() {
                        "Date" => current_comment.date = text,
                        "Author" => current_comment.author = text,
                        "Comment" => current_comment.text = text,
                        _ => {}
                    }
                } else if in_topic {
                    match current_tag.as_str() {
                        "Title" => topic.title = text,
                        "Description" => topic.description = text,
                        "CreationDate" => topic.creation_date = text,
                        "CreationAuthor" => topic.creation_author = text,
                        "ModifiedDate" => topic.modified_date = Some(text),
                        "Priority" => {
                            topic.priority =
                                text.parse().unwrap_or(BcfPriority::Normal);
                        }
                        "AssignedTo" => topic.assigned_to = Some(text),
                        _ => {}
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag_name == "Viewpoint" && in_comment {
                    for attr in e.attributes().flatten() {
                        let key =
                            String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let val = String::from_utf8_lossy(&attr.value).to_string();
                        if key == "Guid" {
                            current_comment.viewpoint_guid = Some(val);
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match tag_name.as_str() {
                    "Topic" => in_topic = false,
                    "Comment" if in_comment => {
                        in_comment = false;
                        topic.comments.push(current_comment.clone());
                    }
                    _ => {}
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Ok((topic, viewpoint_guids))
}

/// Parse viewpoint.bcfv XML into a `BcfViewpoint`.
pub fn parse_viewpoint_xml(data: &[u8]) -> Result<BcfViewpoint, String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(data);
    reader.config_mut().trim_text(true);

    let mut vp = BcfViewpoint {
        guid: String::new(),
        camera_position: [0.0; 3],
        camera_direction: [0.0, 0.0, -1.0],
        camera_up: [0.0, 1.0, 0.0],
        field_of_view: 60.0,
        is_orthographic: false,
        selected_components: Vec::new(),
        hidden_components: Vec::new(),
        snapshot_png: None,
    };

    let mut buf = Vec::new();
    let mut tag_stack: Vec<String> = Vec::new();
    let mut current_xyz: [f32; 3] = [0.0; 3];
    let mut in_selection = false;
    let mut in_exceptions = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag_name.as_str() {
                    "VisualizationInfo" => {
                        for attr in e.attributes().flatten() {
                            let key =
                                String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let val =
                                String::from_utf8_lossy(&attr.value).to_string();
                            if key == "Guid" {
                                vp.guid = val;
                            }
                        }
                    }
                    "OrthogonalCamera" => vp.is_orthographic = true,
                    "PerspectiveCamera" => vp.is_orthographic = false,
                    "CameraViewPoint" | "CameraDirection" | "CameraUpVector" => {
                        current_xyz = [0.0; 3];
                    }
                    "Selection" => in_selection = true,
                    "Exceptions" => in_exceptions = true,
                    "Visibility" => {
                        // Check DefaultVisibility attribute (we handle it but
                        // our model assumes default=true with hidden exceptions)
                    }
                    _ => {}
                }

                tag_stack.push(tag_name);
            }
            Ok(Event::Text(ref e)) => {
                let text = e
                    .unescape()
                    .map_err(|err| format!("XML unescape error: {}", err))?
                    .to_string();

                if let Some(tag) = tag_stack.last() {
                    match tag.as_str() {
                        "X" => {
                            current_xyz[0] =
                                text.parse().unwrap_or(0.0);
                        }
                        "Y" => {
                            current_xyz[1] =
                                text.parse().unwrap_or(0.0);
                        }
                        "Z" => {
                            current_xyz[2] =
                                text.parse().unwrap_or(0.0);
                        }
                        "FieldOfView" => {
                            vp.field_of_view =
                                text.parse().unwrap_or(60.0);
                        }
                        "ViewToWorldScale" => {
                            vp.field_of_view =
                                text.parse().unwrap_or(1.0);
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag_name == "Component" {
                    for attr in e.attributes().flatten() {
                        let key =
                            String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let val = String::from_utf8_lossy(&attr.value).to_string();
                        if key == "IfcGuid" {
                            if in_selection {
                                vp.selected_components.push(val);
                            } else if in_exceptions {
                                vp.hidden_components.push(val);
                            }
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag_name.as_str() {
                    "CameraViewPoint" => vp.camera_position = current_xyz,
                    "CameraDirection" => vp.camera_direction = current_xyz,
                    "CameraUpVector" => vp.camera_up = current_xyz,
                    "Selection" => in_selection = false,
                    "Exceptions" => in_exceptions = false,
                    _ => {}
                }

                tag_stack.pop();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(vp)
}

// ---------------------------------------------------------------------------
// Private XML writing helpers
// ---------------------------------------------------------------------------

/// Write a simple `<Tag>text</Tag>` element.
fn write_xml_text_element<W: Write>(
    writer: &mut quick_xml::Writer<W>,
    tag: &str,
    text: &str,
) -> Result<(), String> {
    use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};

    writer
        .write_event(Event::Start(BytesStart::new(tag)))
        .map_err(|e| format!("XML write error: {}", e))?;
    writer
        .write_event(Event::Text(BytesText::new(text)))
        .map_err(|e| format!("XML write error: {}", e))?;
    writer
        .write_event(Event::End(BytesEnd::new(tag)))
        .map_err(|e| format!("XML write error: {}", e))?;
    Ok(())
}

/// Write a `<Tag><X>...</X><Y>...</Y><Z>...</Z></Tag>` element.
fn write_xyz_element<W: Write>(
    writer: &mut quick_xml::Writer<W>,
    tag: &str,
    xyz: [f32; 3],
) -> Result<(), String> {
    use quick_xml::events::{BytesEnd, BytesStart, Event};

    writer
        .write_event(Event::Start(BytesStart::new(tag)))
        .map_err(|e| format!("XML write error: {}", e))?;
    write_xml_text_element(writer, "X", &format!("{}", xyz[0]))?;
    write_xml_text_element(writer, "Y", &format!("{}", xyz[1]))?;
    write_xml_text_element(writer, "Z", &format!("{}", xyz[2]))?;
    writer
        .write_event(Event::End(BytesEnd::new(tag)))
        .map_err(|e| format!("XML write error: {}", e))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_project() -> BcfProject {
        BcfProject {
            project_id: "test-project-id".to_string(),
            name: "Test Project".to_string(),
            topics: vec![BcfTopic {
                guid: "topic-guid-1".to_string(),
                title: "Wall missing insulation".to_string(),
                description: "The exterior wall needs insulation.".to_string(),
                status: BcfTopicStatus::Open,
                priority: BcfPriority::High,
                topic_type: "Issue".to_string(),
                creation_date: "2026-02-21T00:00:00Z".to_string(),
                creation_author: "user@example.com".to_string(),
                modified_date: None,
                assigned_to: Some("engineer@example.com".to_string()),
                comments: vec![BcfComment {
                    guid: "comment-guid-1".to_string(),
                    date: "2026-02-21T00:00:00Z".to_string(),
                    author: "user@example.com".to_string(),
                    text: "Please review this area.".to_string(),
                    viewpoint_guid: None,
                }],
                viewpoints: vec![BcfViewpoint {
                    guid: "vp-guid-1".to_string(),
                    camera_position: [10.0, 5.0, 15.0],
                    camera_direction: [-0.5, -0.2, -0.8],
                    camera_up: [0.0, 1.0, 0.0],
                    field_of_view: 60.0,
                    is_orthographic: false,
                    selected_components: vec!["3Dt98nDEX0BxIq3VX1gkTi".to_string()],
                    hidden_components: vec!["2$aV1p02v0nQM1iDcAPbvq".to_string()],
                    snapshot_png: None,
                }],
            }],
        }
    }

    #[test]
    fn test_markup_xml_roundtrip() {
        let project = sample_project();
        let topic = &project.topics[0];

        let xml = write_markup_xml(topic).expect("write_markup_xml failed");
        let (parsed_topic, vp_guids) =
            parse_markup_xml(&xml).expect("parse_markup_xml failed");

        assert_eq!(parsed_topic.title, topic.title);
        assert_eq!(parsed_topic.description, topic.description);
        assert_eq!(parsed_topic.status, topic.status);
        assert_eq!(parsed_topic.priority, topic.priority);
        assert_eq!(parsed_topic.creation_author, topic.creation_author);
        assert_eq!(parsed_topic.comments.len(), 1);
        assert_eq!(parsed_topic.comments[0].text, "Please review this area.");
        assert_eq!(vp_guids.len(), 1);
        assert_eq!(vp_guids[0], "vp-guid-1");
    }

    #[test]
    fn test_viewpoint_xml_roundtrip() {
        let project = sample_project();
        let vp = &project.topics[0].viewpoints[0];

        let xml = write_viewpoint_xml(vp).expect("write_viewpoint_xml failed");
        let parsed_vp = parse_viewpoint_xml(&xml).expect("parse_viewpoint_xml failed");

        assert_eq!(parsed_vp.guid, vp.guid);
        assert!((parsed_vp.camera_position[0] - 10.0).abs() < 0.001);
        assert!((parsed_vp.camera_position[1] - 5.0).abs() < 0.001);
        assert!((parsed_vp.camera_position[2] - 15.0).abs() < 0.001);
        assert!((parsed_vp.field_of_view - 60.0).abs() < 0.001);
        assert!(!parsed_vp.is_orthographic);
        assert_eq!(parsed_vp.selected_components.len(), 1);
        assert_eq!(parsed_vp.hidden_components.len(), 1);
    }

    #[test]
    fn test_bcf_zip_roundtrip() {
        let project = sample_project();

        let zip_bytes = export_bcf_zip(&project).expect("export_bcf_zip failed");
        assert!(!zip_bytes.is_empty());

        let imported = import_bcf_zip(&zip_bytes).expect("import_bcf_zip failed");
        assert_eq!(imported.topics.len(), 1);
        assert_eq!(imported.topics[0].title, "Wall missing insulation");
        assert_eq!(imported.topics[0].comments.len(), 1);
        assert_eq!(imported.topics[0].viewpoints.len(), 1);
    }

    #[test]
    fn test_bcf_json_roundtrip() {
        let project = sample_project();

        let json = export_bcf_json(&project).expect("export_bcf_json failed");
        let imported = import_bcf_json(&json).expect("import_bcf_json failed");

        assert_eq!(imported.project_id, project.project_id);
        assert_eq!(imported.topics.len(), 1);
        assert_eq!(imported.topics[0].title, project.topics[0].title);
    }

    #[test]
    fn test_status_display_parse() {
        for status in [
            BcfTopicStatus::Open,
            BcfTopicStatus::InProgress,
            BcfTopicStatus::Closed,
            BcfTopicStatus::ReOpened,
        ] {
            let s = status.to_string();
            let parsed: BcfTopicStatus = s.parse().unwrap();
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn test_priority_display_parse() {
        for prio in [
            BcfPriority::Low,
            BcfPriority::Normal,
            BcfPriority::High,
            BcfPriority::Critical,
        ] {
            let s = prio.to_string();
            let parsed: BcfPriority = s.parse().unwrap();
            assert_eq!(parsed, prio);
        }
    }
}
