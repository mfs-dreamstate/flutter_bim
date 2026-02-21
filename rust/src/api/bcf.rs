//! BCF FFI API — exposed to Flutter via flutter_rust_bridge.
//!
//! Manages BCF projects, topics, comments, viewpoints, and import/export.
//! State is stored in a separate static (BCF_STATE) to avoid modifying AppState.

use flutter_rust_bridge::frb;

use super::state::with_state;
use crate::bim::bcf::*;

use std::sync::{LazyLock, Mutex};

// ---------------------------------------------------------------------------
// BCF-specific state (separate from AppState to avoid modification)
// ---------------------------------------------------------------------------

static BCF_STATE: LazyLock<Mutex<Option<BcfProject>>> = LazyLock::new(|| Mutex::new(None));

fn with_bcf<T>(f: impl FnOnce(&mut Option<BcfProject>) -> T) -> T {
    let mut state = BCF_STATE.lock().unwrap();
    f(&mut state)
}

// ---------------------------------------------------------------------------
// Summary / detail structs for Flutter
// ---------------------------------------------------------------------------

/// Summary of a BCF topic for list views.
#[derive(Debug, Clone)]
pub struct BcfTopicSummary {
    pub guid: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub comment_count: i32,
    pub creation_date: String,
}

/// Full detail of a BCF topic including comments.
#[derive(Debug, Clone)]
pub struct BcfTopicDetail {
    pub guid: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub topic_type: String,
    pub creation_date: String,
    pub creation_author: String,
    pub modified_date: Option<String>,
    pub assigned_to: Option<String>,
    pub comments: Vec<BcfCommentData>,
    pub viewpoint_count: i32,
}

/// Comment data for Flutter.
#[derive(Debug, Clone)]
pub struct BcfCommentData {
    pub guid: String,
    pub date: String,
    pub author: String,
    pub text: String,
    pub viewpoint_guid: Option<String>,
}

// ---------------------------------------------------------------------------
// Project management
// ---------------------------------------------------------------------------

/// Create a new BCF project, replacing any existing one. Returns the project ID.
#[frb(sync)]
pub fn create_bcf_project(name: String) -> String {
    let project_id = uuid::Uuid::new_v4().to_string();
    let pid = project_id.clone();
    with_bcf(|state| {
        *state = Some(BcfProject {
            project_id,
            name,
            topics: Vec::new(),
        });
    });
    pid
}

/// Clear the current BCF project.
#[frb(sync)]
pub fn clear_bcf_project() -> Result<(), String> {
    with_bcf(|state| {
        *state = None;
        Ok(())
    })
}

/// Get the number of topics in the current BCF project.
#[frb(sync)]
pub fn get_topic_count() -> i32 {
    with_bcf(|state| {
        state
            .as_ref()
            .map(|p| p.topics.len() as i32)
            .unwrap_or(0)
    })
}

// ---------------------------------------------------------------------------
// Topic CRUD
// ---------------------------------------------------------------------------

/// Create a new BCF topic. Returns the topic GUID.
#[frb(sync)]
pub fn create_bcf_topic(
    title: String,
    description: String,
    topic_type: String,
) -> Result<String, String> {
    with_bcf(|state| {
        let project = state.as_mut().ok_or("No BCF project. Call create_bcf_project first.")?;

        let guid = uuid::Uuid::new_v4().to_string();
        let topic = BcfTopic {
            guid: guid.clone(),
            title,
            description,
            status: BcfTopicStatus::Open,
            priority: BcfPriority::Normal,
            topic_type,
            creation_date: "2026-02-21T00:00:00Z".to_string(),
            creation_author: "flutter_bim_user".to_string(),
            modified_date: None,
            assigned_to: None,
            comments: Vec::new(),
            viewpoints: Vec::new(),
        };
        project.topics.push(topic);
        Ok(guid)
    })
}

/// Delete a topic by GUID.
#[frb(sync)]
pub fn delete_topic(topic_guid: String) -> Result<(), String> {
    with_bcf(|state| {
        let project = state.as_mut().ok_or("No BCF project")?;
        let before = project.topics.len();
        project.topics.retain(|t| t.guid != topic_guid);
        if project.topics.len() == before {
            Err(format!("Topic '{}' not found", topic_guid))
        } else {
            Ok(())
        }
    })
}

/// Get all topics as summaries.
#[frb(sync)]
pub fn get_all_topics() -> Vec<BcfTopicSummary> {
    with_bcf(|state| {
        let Some(project) = state.as_ref() else {
            return Vec::new();
        };
        project
            .topics
            .iter()
            .map(|t| BcfTopicSummary {
                guid: t.guid.clone(),
                title: t.title.clone(),
                status: t.status.to_string(),
                priority: t.priority.to_string(),
                comment_count: t.comments.len() as i32,
                creation_date: t.creation_date.clone(),
            })
            .collect()
    })
}

/// Get full detail for a single topic.
#[frb(sync)]
pub fn get_topic_detail(topic_guid: String) -> Result<BcfTopicDetail, String> {
    with_bcf(|state| {
        let project = state.as_ref().ok_or("No BCF project")?;
        let topic = project
            .topics
            .iter()
            .find(|t| t.guid == topic_guid)
            .ok_or_else(|| format!("Topic '{}' not found", topic_guid))?;

        Ok(BcfTopicDetail {
            guid: topic.guid.clone(),
            title: topic.title.clone(),
            description: topic.description.clone(),
            status: topic.status.to_string(),
            priority: topic.priority.to_string(),
            topic_type: topic.topic_type.clone(),
            creation_date: topic.creation_date.clone(),
            creation_author: topic.creation_author.clone(),
            modified_date: topic.modified_date.clone(),
            assigned_to: topic.assigned_to.clone(),
            comments: topic
                .comments
                .iter()
                .map(|c| BcfCommentData {
                    guid: c.guid.clone(),
                    date: c.date.clone(),
                    author: c.author.clone(),
                    text: c.text.clone(),
                    viewpoint_guid: c.viewpoint_guid.clone(),
                })
                .collect(),
            viewpoint_count: topic.viewpoints.len() as i32,
        })
    })
}

// ---------------------------------------------------------------------------
// Topic metadata updates
// ---------------------------------------------------------------------------

/// Set topic status. 0=Open, 1=InProgress, 2=Closed, 3=ReOpened.
#[frb(sync)]
pub fn set_topic_status(topic_guid: String, status: i32) -> Result<(), String> {
    with_bcf(|state| {
        let project = state.as_mut().ok_or("No BCF project")?;
        let topic = project
            .topics
            .iter_mut()
            .find(|t| t.guid == topic_guid)
            .ok_or_else(|| format!("Topic '{}' not found", topic_guid))?;

        topic.status = match status {
            0 => BcfTopicStatus::Open,
            1 => BcfTopicStatus::InProgress,
            2 => BcfTopicStatus::Closed,
            3 => BcfTopicStatus::ReOpened,
            _ => return Err(format!("Invalid status code: {}", status)),
        };
        topic.modified_date = Some("2026-02-21T00:00:00Z".to_string());
        Ok(())
    })
}

/// Set topic priority. 0=Low, 1=Normal, 2=High, 3=Critical.
#[frb(sync)]
pub fn set_topic_priority(topic_guid: String, priority: i32) -> Result<(), String> {
    with_bcf(|state| {
        let project = state.as_mut().ok_or("No BCF project")?;
        let topic = project
            .topics
            .iter_mut()
            .find(|t| t.guid == topic_guid)
            .ok_or_else(|| format!("Topic '{}' not found", topic_guid))?;

        topic.priority = match priority {
            0 => BcfPriority::Low,
            1 => BcfPriority::Normal,
            2 => BcfPriority::High,
            3 => BcfPriority::Critical,
            _ => return Err(format!("Invalid priority code: {}", priority)),
        };
        topic.modified_date = Some("2026-02-21T00:00:00Z".to_string());
        Ok(())
    })
}

/// Set the assigned-to field on a topic.
#[frb(sync)]
pub fn set_topic_assigned_to(topic_guid: String, assigned_to: String) -> Result<(), String> {
    with_bcf(|state| {
        let project = state.as_mut().ok_or("No BCF project")?;
        let topic = project
            .topics
            .iter_mut()
            .find(|t| t.guid == topic_guid)
            .ok_or_else(|| format!("Topic '{}' not found", topic_guid))?;

        topic.assigned_to = if assigned_to.is_empty() {
            None
        } else {
            Some(assigned_to)
        };
        topic.modified_date = Some("2026-02-21T00:00:00Z".to_string());
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// Comments
// ---------------------------------------------------------------------------

/// Add a comment to a topic. Returns the comment GUID.
#[frb(sync)]
pub fn add_bcf_comment(
    topic_guid: String,
    author: String,
    text: String,
) -> Result<String, String> {
    with_bcf(|state| {
        let project = state.as_mut().ok_or("No BCF project")?;
        let topic = project
            .topics
            .iter_mut()
            .find(|t| t.guid == topic_guid)
            .ok_or_else(|| format!("Topic '{}' not found", topic_guid))?;

        let guid = uuid::Uuid::new_v4().to_string();
        topic.comments.push(BcfComment {
            guid: guid.clone(),
            date: "2026-02-21T00:00:00Z".to_string(),
            author,
            text,
            viewpoint_guid: None,
        });
        topic.modified_date = Some("2026-02-21T00:00:00Z".to_string());
        Ok(guid)
    })
}

// ---------------------------------------------------------------------------
// Viewpoints — capture from current renderer state
// ---------------------------------------------------------------------------

/// Capture the current camera position, selection, and hidden elements as a
/// BCF viewpoint and attach it to the specified topic. Returns the viewpoint GUID.
#[frb(sync)]
pub fn create_viewpoint_from_current(topic_guid: String) -> Result<String, String> {
    // First, read camera and selection state from AppState
    let (position, target, selected_global_ids, hidden_global_ids) = with_state(|s| {
        let r = s
            .renderer
            .as_ref()
            .ok_or_else(|| "Renderer not initialized".to_string())?;
        let pos = r.camera.position();
        let tgt = r.camera.target();

        // Map selected element IDs to IFC GlobalIDs
        let mut sel_ids = Vec::new();
        let mut hid_ids = Vec::new();
        for (_model_id, reg_model) in s.registry.iter_visible() {
            for elem in reg_model.elements() {
                if s.selected_elements.contains(&elem.id) {
                    sel_ids.push(elem.global_id.clone());
                }
                if s.hidden_elements.contains(&elem.id) {
                    hid_ids.push(elem.global_id.clone());
                }
            }
        }

        Ok::<_, String>((pos, tgt, sel_ids, hid_ids))
    })?;

    // Compute direction from position to target
    let dx = target[0] - position[0];
    let dy = target[1] - position[1];
    let dz = target[2] - position[2];
    let len = (dx * dx + dy * dy + dz * dz).sqrt().max(0.0001);
    let direction = [dx / len, dy / len, dz / len];

    let vp_guid = uuid::Uuid::new_v4().to_string();
    let vp = BcfViewpoint {
        guid: vp_guid.clone(),
        camera_position: position,
        camera_direction: direction,
        camera_up: [0.0, 1.0, 0.0],
        field_of_view: 45.0, // default; camera fov is private
        is_orthographic: false,
        selected_components: selected_global_ids,
        hidden_components: hidden_global_ids,
        snapshot_png: None,
    };

    // Now add the viewpoint to the BCF topic
    with_bcf(|state| {
        let project = state.as_mut().ok_or("No BCF project")?;
        let topic = project
            .topics
            .iter_mut()
            .find(|t| t.guid == topic_guid)
            .ok_or_else(|| format!("Topic '{}' not found", topic_guid))?;
        topic.viewpoints.push(vp);
        topic.modified_date = Some("2026-02-21T00:00:00Z".to_string());
        Ok(vp_guid)
    })
}

/// Navigate to a BCF viewpoint: restore camera position/target and apply
/// selection/visibility state.
#[frb(sync)]
pub fn navigate_to_viewpoint(
    topic_guid: String,
    viewpoint_guid: String,
) -> Result<(), String> {
    // Read viewpoint data from BCF state
    let vp = with_bcf(|state| {
        let project = state.as_ref().ok_or("No BCF project")?;
        let topic = project
            .topics
            .iter()
            .find(|t| t.guid == topic_guid)
            .ok_or_else(|| format!("Topic '{}' not found", topic_guid))?;
        let viewpoint = topic
            .viewpoints
            .iter()
            .find(|v| v.guid == viewpoint_guid)
            .ok_or_else(|| format!("Viewpoint '{}' not found", viewpoint_guid))?;
        Ok::<_, String>(viewpoint.clone())
    })?;

    // Compute the camera target from position + direction
    let target = [
        vp.camera_position[0] + vp.camera_direction[0] * 10.0,
        vp.camera_position[1] + vp.camera_direction[1] * 10.0,
        vp.camera_position[2] + vp.camera_direction[2] * 10.0,
    ];

    // Apply camera and selection state
    with_state(|s| {
        let r = s
            .renderer
            .as_mut()
            .ok_or_else(|| "Renderer not initialized".to_string())?;
        r.camera.set_position(vp.camera_position);
        r.camera.set_target(target);
        r.camera.set_orthographic(vp.is_orthographic);

        // Restore selection: clear current, then select matching GlobalIDs
        s.selected_elements.clear();
        s.hidden_elements.clear();

        for (_model_id, reg_model) in s.registry.iter_visible() {
            for elem in reg_model.elements() {
                if vp.selected_components.contains(&elem.global_id) {
                    s.selected_elements.insert(elem.id);
                }
                if vp.hidden_components.contains(&elem.global_id) {
                    s.hidden_elements.insert(elem.id);
                }
            }
        }

        s.selected_element = s.selected_elements.iter().next().copied();

        Ok(())
    })
}

// ---------------------------------------------------------------------------
// Import / Export
// ---------------------------------------------------------------------------

/// Export the current BCF project as BCF 2.1 ZIP bytes.
#[frb(sync)]
pub fn export_bcf_to_bytes() -> Result<Vec<u8>, String> {
    with_bcf(|state| {
        let project = state.as_ref().ok_or("No BCF project")?;
        export_bcf_zip(project)
    })
}

/// Import a BCF 2.1 ZIP from bytes, replacing the current project.
/// Returns the number of topics imported.
#[frb(sync)]
pub fn import_bcf_from_bytes(data: Vec<u8>) -> Result<i32, String> {
    let project = import_bcf_zip(&data)?;
    let count = project.topics.len() as i32;
    with_bcf(|state| {
        *state = Some(project);
    });
    Ok(count)
}

/// Export the current BCF project as BCF 3.0 JSON string.
#[frb(sync)]
pub fn export_bcf_json_string() -> Result<String, String> {
    with_bcf(|state| {
        let project = state.as_ref().ok_or("No BCF project")?;
        export_bcf_json(project)
    })
}

/// Import a BCF 3.0 JSON string, replacing the current project.
/// Returns the number of topics imported.
#[frb(sync)]
pub fn import_bcf_json_string(json: String) -> Result<i32, String> {
    let project = import_bcf_json(&json)?;
    let count = project.topics.len() as i32;
    with_bcf(|state| {
        *state = Some(project);
    });
    Ok(count)
}

// ===========================================================================
// BCF API Server Integration — data layer
// ===========================================================================
//
// Since the Rust side cannot make HTTP calls (no reqwest), we provide:
//   1. Configuration management (server URL, auth token, project ID)
//   2. Request builders  — produce JSON `BcfApiRequest` for Flutter to execute
//   3. Response parsers  — parse BCF API JSON responses into local types
//   4. Sync helpers      — merge remote topics into the local BCF store

/// Configuration for connecting to a BCF API server (BIMcollab, Trimble Connect, etc.).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BcfApiConfig {
    pub server_url: String,
    pub api_version: String,
    pub auth_token: Option<String>,
    pub project_id: Option<String>,
}

/// A fully-formed HTTP request that Flutter should execute on behalf of Rust.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BcfApiRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

/// Well-known BCF API endpoint paths (relative to server_url).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BcfApiEndpoints {
    pub topics: String,
    pub comments: String,
    pub viewpoints: String,
    pub project_extensions: String,
}

// ---------------------------------------------------------------------------
// Global API config (separate from BCF_STATE which holds local project data)
// ---------------------------------------------------------------------------

static BCF_API_CONFIG: LazyLock<Mutex<Option<BcfApiConfig>>> =
    LazyLock::new(|| Mutex::new(None));

fn with_api_config<T>(f: impl FnOnce(&mut Option<BcfApiConfig>) -> T) -> T {
    let mut guard = BCF_API_CONFIG.lock().unwrap();
    f(&mut guard)
}

impl BcfApiConfig {
    /// Build the base URL for the BCF REST API.
    fn base_url(&self) -> String {
        let url = self.server_url.trim_end_matches('/');
        format!("{}/bcf/{}", url, self.api_version)
    }

    /// Build the project-scoped URL prefix (e.g. `.../bcf/2.1/projects/{id}`).
    fn project_url(&self) -> Result<String, String> {
        let pid = self
            .project_id
            .as_ref()
            .ok_or_else(|| "No project_id configured. Call set_bcf_project first.".to_string())?;
        Ok(format!("{}/projects/{}", self.base_url(), pid))
    }

    /// Standard headers including auth and content-type.
    fn default_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
        ];
        if let Some(ref token) = self.auth_token {
            h.push(("Authorization".to_string(), format!("Bearer {}", token)));
        }
        h
    }

    /// Resolve endpoints for the current project.
    fn endpoints(&self) -> Result<BcfApiEndpoints, String> {
        let base = self.project_url()?;
        Ok(BcfApiEndpoints {
            topics: format!("{}/topics", base),
            comments: format!("{}/topics/{{topic_guid}}/comments", base),
            viewpoints: format!("{}/topics/{{topic_guid}}/viewpoints", base),
            project_extensions: format!("{}/extensions", base),
        })
    }
}

// ---------------------------------------------------------------------------
// Server configuration FFI
// ---------------------------------------------------------------------------

/// Configure the BCF API server URL and version.
#[frb(sync)]
pub fn configure_bcf_server(server_url: String, api_version: String) -> Result<(), String> {
    if server_url.is_empty() {
        return Err("server_url must not be empty".to_string());
    }
    if api_version != "2.1" && api_version != "3.0" {
        return Err(format!(
            "Unsupported api_version '{}'. Must be '2.1' or '3.0'.",
            api_version
        ));
    }
    with_api_config(|cfg| {
        *cfg = Some(BcfApiConfig {
            server_url,
            api_version,
            auth_token: None,
            project_id: None,
        });
    });
    Ok(())
}

/// Set the Bearer auth token for the BCF API server.
#[frb(sync)]
pub fn set_bcf_auth_token(token: String) -> Result<(), String> {
    with_api_config(|cfg| {
        let c = cfg
            .as_mut()
            .ok_or_else(|| "No BCF server configured. Call configure_bcf_server first.".to_string())?;
        c.auth_token = Some(token);
        Ok(())
    })
}

/// Clear the Bearer auth token.
#[frb(sync)]
pub fn clear_bcf_auth_token() {
    with_api_config(|cfg| {
        if let Some(c) = cfg.as_mut() {
            c.auth_token = None;
        }
    });
}

/// Set the project ID used for all subsequent API requests.
#[frb(sync)]
pub fn set_bcf_project(project_id: String) -> Result<(), String> {
    with_api_config(|cfg| {
        let c = cfg
            .as_mut()
            .ok_or_else(|| "No BCF server configured. Call configure_bcf_server first.".to_string())?;
        c.project_id = Some(project_id);
        Ok(())
    })
}

/// Return the current BCF API configuration as JSON.
#[frb(sync)]
pub fn get_bcf_config() -> String {
    with_api_config(|cfg| match cfg {
        Some(c) => serde_json::to_string(c).unwrap_or_else(|_| "{}".to_string()),
        None => "null".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Request builders — each returns a JSON-serialized BcfApiRequest
// ---------------------------------------------------------------------------

fn build_request(
    method: &str,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
) -> Result<String, String> {
    let req = BcfApiRequest {
        method: method.to_string(),
        url,
        headers,
        body,
    };
    serde_json::to_string(&req).map_err(|e| format!("Failed to serialize request: {}", e))
}

fn require_config() -> Result<BcfApiConfig, String> {
    with_api_config(|cfg| {
        cfg.clone()
            .ok_or_else(|| "No BCF server configured. Call configure_bcf_server first.".to_string())
    })
}

/// Build a request to list all topics for the configured project.
#[frb(sync)]
pub fn build_list_topics_request() -> Result<String, String> {
    let cfg = require_config()?;
    let endpoints = cfg.endpoints()?;
    build_request("GET", endpoints.topics, cfg.default_headers(), None)
}

/// Build a request to get a single topic by GUID.
#[frb(sync)]
pub fn build_get_topic_request(topic_guid: String) -> Result<String, String> {
    let cfg = require_config()?;
    let endpoints = cfg.endpoints()?;
    let url = format!("{}/{}", endpoints.topics, topic_guid);
    build_request("GET", url, cfg.default_headers(), None)
}

/// Build a request to create a new topic.
#[frb(sync)]
pub fn build_create_topic_request(
    title: String,
    description: String,
    topic_type: String,
    priority: String,
    status: String,
) -> Result<String, String> {
    let cfg = require_config()?;
    let endpoints = cfg.endpoints()?;

    let body = serde_json::json!({
        "title": title,
        "description": description,
        "topic_type": topic_type,
        "priority": priority,
        "topic_status": status,
    });

    build_request(
        "POST",
        endpoints.topics,
        cfg.default_headers(),
        Some(body.to_string()),
    )
}

/// Build a request to update an existing topic. `updates_json` is a JSON
/// object with the fields to patch (e.g. `{"title":"new","priority":"High"}`).
#[frb(sync)]
pub fn build_update_topic_request(
    topic_guid: String,
    updates_json: String,
) -> Result<String, String> {
    let cfg = require_config()?;
    let endpoints = cfg.endpoints()?;
    let url = format!("{}/{}", endpoints.topics, topic_guid);

    // Validate that the body is valid JSON
    let _: serde_json::Value = serde_json::from_str(&updates_json)
        .map_err(|e| format!("Invalid updates JSON: {}", e))?;

    build_request("PUT", url, cfg.default_headers(), Some(updates_json))
}

/// Build a request to list comments for a topic.
#[frb(sync)]
pub fn build_list_comments_request(topic_guid: String) -> Result<String, String> {
    let cfg = require_config()?;
    let endpoints = cfg.endpoints()?;
    let url = endpoints
        .comments
        .replace("{topic_guid}", &topic_guid);
    build_request("GET", url, cfg.default_headers(), None)
}

/// Build a request to create a comment on a topic.
#[frb(sync)]
pub fn build_create_comment_request(
    topic_guid: String,
    comment: String,
) -> Result<String, String> {
    let cfg = require_config()?;
    let endpoints = cfg.endpoints()?;
    let url = endpoints
        .comments
        .replace("{topic_guid}", &topic_guid);

    let body = serde_json::json!({ "comment": comment });
    build_request("POST", url, cfg.default_headers(), Some(body.to_string()))
}

/// Build a request to list viewpoints for a topic.
#[frb(sync)]
pub fn build_list_viewpoints_request(topic_guid: String) -> Result<String, String> {
    let cfg = require_config()?;
    let endpoints = cfg.endpoints()?;
    let url = endpoints
        .viewpoints
        .replace("{topic_guid}", &topic_guid);
    build_request("GET", url, cfg.default_headers(), None)
}

/// Build a request to create a viewpoint on a topic. `viewpoint_json` is the
/// BCF viewpoint payload (camera, components, etc.).
#[frb(sync)]
pub fn build_create_viewpoint_request(
    topic_guid: String,
    viewpoint_json: String,
) -> Result<String, String> {
    let cfg = require_config()?;
    let endpoints = cfg.endpoints()?;
    let url = endpoints
        .viewpoints
        .replace("{topic_guid}", &topic_guid);

    let _: serde_json::Value = serde_json::from_str(&viewpoint_json)
        .map_err(|e| format!("Invalid viewpoint JSON: {}", e))?;

    build_request("POST", url, cfg.default_headers(), Some(viewpoint_json))
}

// ---------------------------------------------------------------------------
// Response parsers
// ---------------------------------------------------------------------------

/// A topic as returned by the BCF REST API (JSON representation).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ApiTopic {
    guid: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default, alias = "topic_status")]
    topic_status: String,
    #[serde(default)]
    priority: String,
    #[serde(default)]
    topic_type: String,
    #[serde(default)]
    creation_date: String,
    #[serde(default)]
    creation_author: String,
    #[serde(default)]
    modified_date: Option<String>,
    #[serde(default)]
    assigned_to: Option<String>,
}

/// A comment as returned by the BCF REST API (JSON representation).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ApiComment {
    guid: String,
    #[serde(default)]
    date: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    comment: String,
    #[serde(default)]
    viewpoint_guid: Option<String>,
}

/// Parse a BCF API topics list response. Returns our internal `BcfTopic[]` as JSON.
#[frb(sync)]
pub fn parse_topics_response(response_json: String) -> Result<String, String> {
    let api_topics: Vec<ApiTopic> = serde_json::from_str(&response_json)
        .map_err(|e| format!("Failed to parse topics response: {}", e))?;

    let topics: Vec<BcfTopic> = api_topics
        .into_iter()
        .map(|t| BcfTopic {
            guid: t.guid,
            title: t.title,
            description: t.description,
            status: t.topic_status.parse().unwrap_or(BcfTopicStatus::Open),
            priority: t.priority.parse().unwrap_or(BcfPriority::Normal),
            topic_type: t.topic_type,
            creation_date: t.creation_date,
            creation_author: t.creation_author,
            modified_date: t.modified_date,
            assigned_to: t.assigned_to,
            comments: Vec::new(),
            viewpoints: Vec::new(),
        })
        .collect();

    serde_json::to_string(&topics)
        .map_err(|e| format!("Failed to serialize parsed topics: {}", e))
}

/// Parse a BCF API comments list response. Returns our internal `BcfComment[]` as JSON.
#[frb(sync)]
pub fn parse_comments_response(response_json: String) -> Result<String, String> {
    let api_comments: Vec<ApiComment> = serde_json::from_str(&response_json)
        .map_err(|e| format!("Failed to parse comments response: {}", e))?;

    let comments: Vec<BcfComment> = api_comments
        .into_iter()
        .map(|c| BcfComment {
            guid: c.guid,
            date: c.date,
            author: c.author,
            text: c.comment,
            viewpoint_guid: c.viewpoint_guid,
        })
        .collect();

    serde_json::to_string(&comments)
        .map_err(|e| format!("Failed to serialize parsed comments: {}", e))
}

// ---------------------------------------------------------------------------
// Sync helpers
// ---------------------------------------------------------------------------

/// Import topics from a BCF API topics list response JSON into the local BCF
/// store, creating a project if none exists. Returns the number of topics imported.
#[frb(sync)]
pub fn sync_topics_from_response(response_json: String) -> Result<usize, String> {
    let api_topics: Vec<ApiTopic> = serde_json::from_str(&response_json)
        .map_err(|e| format!("Failed to parse topics response: {}", e))?;

    let count = api_topics.len();

    let topics: Vec<BcfTopic> = api_topics
        .into_iter()
        .map(|t| BcfTopic {
            guid: t.guid,
            title: t.title,
            description: t.description,
            status: t.topic_status.parse().unwrap_or(BcfTopicStatus::Open),
            priority: t.priority.parse().unwrap_or(BcfPriority::Normal),
            topic_type: t.topic_type,
            creation_date: t.creation_date,
            creation_author: t.creation_author,
            modified_date: t.modified_date,
            assigned_to: t.assigned_to,
            comments: Vec::new(),
            viewpoints: Vec::new(),
        })
        .collect();

    with_bcf(|state| {
        let project = state.get_or_insert_with(|| BcfProject {
            project_id: uuid::Uuid::new_v4().to_string(),
            name: "Synced from server".to_string(),
            topics: Vec::new(),
        });

        // Merge: update existing topics by GUID, insert new ones
        for new_topic in topics {
            if let Some(existing) = project.topics.iter_mut().find(|t| t.guid == new_topic.guid) {
                existing.title = new_topic.title;
                existing.description = new_topic.description;
                existing.status = new_topic.status;
                existing.priority = new_topic.priority;
                existing.topic_type = new_topic.topic_type;
                existing.creation_date = new_topic.creation_date;
                existing.creation_author = new_topic.creation_author;
                existing.modified_date = new_topic.modified_date;
                existing.assigned_to = new_topic.assigned_to;
            } else {
                project.topics.push(new_topic);
            }
        }
    });

    Ok(count)
}

/// Prepare all local topics as BCF API-compatible JSON, ready for upload.
/// Returns a JSON array of topic objects matching the API schema.
#[frb(sync)]
pub fn prepare_topics_for_upload() -> Result<String, String> {
    with_bcf(|state| {
        let project = state
            .as_ref()
            .ok_or_else(|| "No BCF project".to_string())?;

        let api_topics: Vec<serde_json::Value> = project
            .topics
            .iter()
            .map(|t| {
                serde_json::json!({
                    "guid": t.guid,
                    "title": t.title,
                    "description": t.description,
                    "topic_status": t.status.to_string(),
                    "priority": t.priority.to_string(),
                    "topic_type": t.topic_type,
                    "creation_date": t.creation_date,
                    "creation_author": t.creation_author,
                    "modified_date": t.modified_date,
                    "assigned_to": t.assigned_to,
                })
            })
            .collect();

        serde_json::to_string(&api_topics)
            .map_err(|e| format!("Failed to serialize topics for upload: {}", e))
    })
}

// ---------------------------------------------------------------------------
// BCF API tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod bcf_api_tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    /// Serialize tests that share global BCF_API_CONFIG / BCF_STATE.
    static TEST_LOCK: StdMutex<()> = StdMutex::new(());

    /// Reset global config state between tests.
    fn reset_api_config() {
        with_api_config(|cfg| *cfg = None);
    }

    fn reset_bcf_state() {
        with_bcf(|state| *state = None);
    }

    #[test]
    fn test_bcf_api_config_set_get() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset_api_config();

        // Initially null
        let json = get_bcf_config();
        assert_eq!(json, "null");

        // Configure server
        configure_bcf_server(
            "https://api.bimcollab.com".to_string(),
            "2.1".to_string(),
        )
        .unwrap();

        set_bcf_auth_token("my-token-123".to_string()).unwrap();
        set_bcf_project("proj-abc".to_string()).unwrap();

        let json = get_bcf_config();
        let cfg: BcfApiConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.server_url, "https://api.bimcollab.com");
        assert_eq!(cfg.api_version, "2.1");
        assert_eq!(cfg.auth_token.as_deref(), Some("my-token-123"));
        assert_eq!(cfg.project_id.as_deref(), Some("proj-abc"));

        // Clear token
        clear_bcf_auth_token();
        let json = get_bcf_config();
        let cfg: BcfApiConfig = serde_json::from_str(&json).unwrap();
        assert!(cfg.auth_token.is_none());

        reset_api_config();
    }

    #[test]
    fn test_bcf_api_config_validation() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset_api_config();

        // Empty URL
        assert!(configure_bcf_server("".to_string(), "2.1".to_string()).is_err());
        // Bad version
        assert!(configure_bcf_server("https://x.com".to_string(), "1.0".to_string()).is_err());
        // No config yet — set_bcf_auth_token should fail
        assert!(set_bcf_auth_token("tok".to_string()).is_err());

        reset_api_config();
    }

    #[test]
    fn test_build_list_topics_request() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset_api_config();
        configure_bcf_server("https://api.example.com".to_string(), "2.1".to_string()).unwrap();
        set_bcf_auth_token("tok-456".to_string()).unwrap();
        set_bcf_project("project-1".to_string()).unwrap();

        let json = build_list_topics_request().unwrap();
        let req: BcfApiRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(req.method, "GET");
        assert_eq!(
            req.url,
            "https://api.example.com/bcf/2.1/projects/project-1/topics"
        );
        assert!(req.body.is_none());
        // Check auth header
        let auth = req.headers.iter().find(|(k, _)| k == "Authorization");
        assert_eq!(auth.unwrap().1, "Bearer tok-456");

        reset_api_config();
    }

    #[test]
    fn test_build_create_topic_request() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset_api_config();
        configure_bcf_server("https://api.example.com".to_string(), "3.0".to_string()).unwrap();
        set_bcf_project("p1".to_string()).unwrap();

        let json = build_create_topic_request(
            "Clash detected".to_string(),
            "Beam vs column".to_string(),
            "Clash".to_string(),
            "High".to_string(),
            "Open".to_string(),
        )
        .unwrap();

        let req: BcfApiRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.method, "POST");
        assert!(req.url.contains("/bcf/3.0/projects/p1/topics"));

        let body: serde_json::Value = serde_json::from_str(req.body.as_ref().unwrap()).unwrap();
        assert_eq!(body["title"], "Clash detected");
        assert_eq!(body["priority"], "High");
        assert_eq!(body["topic_status"], "Open");

        reset_api_config();
    }

    #[test]
    fn test_parse_topics_response() {
        let response = r#"[
            {
                "guid": "t-1",
                "title": "Missing wall",
                "description": "Wall is missing",
                "topic_status": "Open",
                "priority": "High",
                "topic_type": "Issue",
                "creation_date": "2026-01-01T00:00:00Z",
                "creation_author": "user@test.com"
            },
            {
                "guid": "t-2",
                "title": "Door clash",
                "description": "",
                "topic_status": "Closed",
                "priority": "Low",
                "topic_type": "Clash",
                "creation_date": "2026-01-02T00:00:00Z",
                "creation_author": "admin@test.com"
            }
        ]"#;

        let result = parse_topics_response(response.to_string()).unwrap();
        let topics: Vec<BcfTopic> = serde_json::from_str(&result).unwrap();
        assert_eq!(topics.len(), 2);
        assert_eq!(topics[0].guid, "t-1");
        assert_eq!(topics[0].title, "Missing wall");
        assert_eq!(topics[0].status, BcfTopicStatus::Open);
        assert_eq!(topics[0].priority, BcfPriority::High);
        assert_eq!(topics[1].guid, "t-2");
        assert_eq!(topics[1].status, BcfTopicStatus::Closed);
    }

    #[test]
    fn test_parse_comments_response() {
        let response = r#"[
            {
                "guid": "c-1",
                "date": "2026-02-20T12:00:00Z",
                "author": "user@test.com",
                "comment": "Please fix this wall."
            }
        ]"#;

        let result = parse_comments_response(response.to_string()).unwrap();
        let comments: Vec<BcfComment> = serde_json::from_str(&result).unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].guid, "c-1");
        assert_eq!(comments[0].text, "Please fix this wall.");
    }

    #[test]
    fn test_sync_topics_from_response() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset_bcf_state();

        let response = r#"[
            {
                "guid": "sync-1",
                "title": "Synced topic",
                "description": "From server",
                "topic_status": "InProgress",
                "priority": "Normal",
                "topic_type": "Request",
                "creation_date": "2026-02-21T00:00:00Z",
                "creation_author": "server@test.com"
            }
        ]"#;

        let count = sync_topics_from_response(response.to_string()).unwrap();
        assert_eq!(count, 1);

        // The local store should now have the topic
        let topics = get_all_topics();
        assert!(topics.iter().any(|t| t.guid == "sync-1"));

        // Sync again with updated data — should merge, not duplicate
        let response2 = r#"[
            {
                "guid": "sync-1",
                "title": "Updated title",
                "description": "Updated",
                "topic_status": "Closed",
                "priority": "High",
                "topic_type": "Request",
                "creation_date": "2026-02-21T00:00:00Z",
                "creation_author": "server@test.com"
            }
        ]"#;
        let count2 = sync_topics_from_response(response2.to_string()).unwrap();
        assert_eq!(count2, 1);

        let detail = get_topic_detail("sync-1".to_string()).unwrap();
        assert_eq!(detail.title, "Updated title");
        assert_eq!(detail.status, "Closed");

        reset_bcf_state();
    }

    #[test]
    fn test_prepare_topics_for_upload() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset_bcf_state();

        create_bcf_project("Upload Test".to_string());
        create_bcf_topic(
            "Test topic".to_string(),
            "desc".to_string(),
            "Issue".to_string(),
        )
        .unwrap();

        let json = prepare_topics_for_upload().unwrap();
        let topics: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0]["title"], "Test topic");
        assert_eq!(topics[0]["topic_status"], "Open");

        reset_bcf_state();
    }
}
