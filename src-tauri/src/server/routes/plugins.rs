use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::plugins::{db_ops, manager, state as plugin_state};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_plugins))
        .route("/install", post(install_plugin))
        .route("/{id}", delete(uninstall_plugin))
        .route("/{id}/enable", put(enable_plugin))
        .route("/{id}/disable", put(disable_plugin))
        .route("/{id}/refresh", put(refresh_plugin))
        .route("/{id}/manifest", get(get_manifest))
        .route("/{id}/state", get(get_state).put(set_state))
        .route(
            "/{id}/data/{table}",
            get(list_data).post(insert_data),
        )
        .route(
            "/{id}/data/{table}/query",
            get(query_data),
        )
        .route(
            "/{id}/data/{table}/{row_id}",
            get(get_data_row).put(update_data_row).delete(delete_data_row),
        )
        .route("/{id}/trigger-popup", post(trigger_popup))
        .route("/{id}/assets/{*path}", get(serve_asset))
}

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct PluginInfo {
    id: String,
    name: String,
    version: String,
    description: String,
    author: String,
    enabled: bool,
    has_control: bool,
    has_editor: bool,
    is_bundled: bool,
}

#[derive(Deserialize)]
struct InstallBody {
    path: String,
}

#[derive(Deserialize)]
struct TriggerPopupBody {
    template_id: String,
    #[serde(default)]
    context: HashMap<String, String>,
    duration: Option<i64>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn list_plugins(State(state): State<AppState>) -> impl IntoResponse {
    let db = state.db.lock().await;
    let mut stmt = match db.prepare(
        "SELECT id, name, version, description, author, enabled, manifest_json, is_bundled FROM plugins",
    ) {
        Ok(s) => s,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))),
    };

    let plugins: Vec<PluginInfo> = stmt
        .query_map([], |row| {
            let manifest_json: String = row.get(6)?;
            let manifest: Option<crate::plugins::manifest::PluginManifest> =
                serde_json::from_str(&manifest_json).ok();
            Ok(PluginInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                description: row.get(3)?,
                author: row.get(4)?,
                enabled: row.get(5)?,
                has_control: manifest.as_ref().and_then(|m| m.control.as_ref()).is_some(),
                has_editor: manifest.as_ref().and_then(|m| m.editor.as_ref()).is_some(),
                is_bundled: row.get::<_, bool>(7)?,
            })
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    (StatusCode::OK, Json(json!(plugins)))
}

async fn install_plugin(
    State(state): State<AppState>,
    Json(body): Json<InstallBody>,
) -> impl IntoResponse {
    let source_dir = std::path::PathBuf::from(&body.path);
    if !source_dir.exists() || !source_dir.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Source directory does not exist"})),
        );
    }

    let db = state.db.lock().await;
    match manager::install_plugin(&state.app_data_dir, &source_dir, &db) {
        Ok(manifest) => {
            // Update in-memory caches
            let mut manifests = state.plugin_manifests.lock().await;
            let mut states = state.plugin_states.lock().await;

            let plugin_st = plugin_state::load_plugin_state(&db, &manifest.id).unwrap_or_default();
            // Merge with defaults
            let mut merged = HashMap::new();
            for (key, var_def) in &manifest.state {
                merged.insert(
                    key.clone(),
                    plugin_st.get(key).cloned().unwrap_or_else(|| var_def.default.clone()),
                );
            }
            states.insert(manifest.id.clone(), merged);
            manifests.insert(manifest.id.clone(), manifest.clone());

            // Broadcast
            broadcast_event(&state, "plugin-installed", json!({"pluginId": manifest.id}));

            (StatusCode::OK, Json(json!({"success": true, "pluginId": manifest.id})))
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

async fn uninstall_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    match manager::uninstall_plugin(&state.app_data_dir, &id, &db) {
        Ok(()) => {
            state.plugin_manifests.lock().await.remove(&id);
            state.plugin_states.lock().await.remove(&id);
            broadcast_event(&state, "plugin-uninstalled", json!({"pluginId": id}));
            (StatusCode::OK, Json(json!({"success": true})))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

async fn enable_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    if let Err(e) = manager::enable_plugin(&db, &id) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})));
    }

    // Reload manifest into cache
    let manifest_json: Option<String> = db
        .query_row("SELECT manifest_json FROM plugins WHERE id = ?1", [&id], |r| r.get(0))
        .ok();
    if let Some(json_str) = manifest_json {
        if let Ok(manifest) = serde_json::from_str::<crate::plugins::manifest::PluginManifest>(&json_str) {
            let plugin_st = plugin_state::load_plugin_state(&db, &id).unwrap_or_default();
            let mut merged = HashMap::new();
            for (key, var_def) in &manifest.state {
                merged.insert(key.clone(), plugin_st.get(key).cloned().unwrap_or_else(|| var_def.default.clone()));
            }
            state.plugin_states.lock().await.insert(id.clone(), merged);
            state.plugin_manifests.lock().await.insert(id.clone(), manifest);
        }
    }

    broadcast_event(&state, "plugin-enabled", json!({"pluginId": id}));
    (StatusCode::OK, Json(json!({"success": true})))
}

async fn disable_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    if let Err(e) = manager::disable_plugin(&db, &id) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})));
    }

    // Collect any programs that reference this plugin's screens or popups
    // before clearing the caches, so the warning is included in the response.
    let affected = check_plugin_usage_in_programs(&db, &id);

    state.plugin_manifests.lock().await.remove(&id);
    state.plugin_states.lock().await.remove(&id);
    broadcast_event(&state, "plugin-disabled", json!({"pluginId": id}));

    let mut resp = json!({"success": true});
    if let Some(warning) = affected {
        resp["warning"] = warning;
    }
    (StatusCode::OK, Json(resp))
}

async fn refresh_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let Some(ref bundled_dir) = state.bundled_plugins_dir else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Bundled plugin directory is not available in this build."})),
        );
    };

    let db = state.db.lock().await;
    match manager::refresh_plugin(&state.app_data_dir, bundled_dir, &id, &db) {
        Ok(manifest) => {
            // Reload state defaults in case new keys were added.
            let plugin_st = crate::plugins::state::load_plugin_state(&db, &id).unwrap_or_default();
            let mut merged = HashMap::new();
            for (key, var_def) in &manifest.state {
                merged.insert(
                    key.clone(),
                    plugin_st.get(key).cloned().unwrap_or_else(|| var_def.default.clone()),
                );
            }
            state.plugin_states.lock().await.insert(id.clone(), merged);
            state.plugin_manifests.lock().await.insert(id.clone(), manifest);

            broadcast_event(&state, "plugin-refreshed", json!({"pluginId": id}));
            (StatusCode::OK, Json(json!({"success": true})))
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))),
    }
}

async fn get_manifest(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    let manifest_json: Result<String, _> = db.query_row(
        "SELECT manifest_json FROM plugins WHERE id = ?1",
        [&id],
        |r| r.get(0),
    );
    match manifest_json {
        Ok(json_str) => match serde_json::from_str::<Value>(&json_str) {
            Ok(val) => (StatusCode::OK, Json(val)),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))),
        },
        Err(_) => (StatusCode::NOT_FOUND, Json(json!({"error": "Plugin not found"}))),
    }
}

async fn get_state(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let states = state.plugin_states.lock().await;
    match states.get(&id) {
        Some(s) => (StatusCode::OK, Json(json!(s))),
        None => (StatusCode::NOT_FOUND, Json(json!({"error": "Plugin not found or disabled"}))),
    }
}

async fn set_state(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(updates): Json<HashMap<String, Value>>,
) -> impl IntoResponse {
    // Write to DB
    {
        let db = state.db.lock().await;
        if let Err(e) = plugin_state::set_state_batch(&db, &id, &updates) {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})));
        }
    }

    // Update in-memory cache
    let full_state = {
        let mut states = state.plugin_states.lock().await;
        let entry = states.entry(id.clone()).or_default();
        for (k, v) in &updates {
            entry.insert(k.clone(), v.clone());
        }
        entry.clone()
    };

    // Broadcast
    broadcast_event(
        &state,
        &format!("plugin-state-updated:{}", id),
        json!({"pluginId": id, "state": full_state}),
    );

    (StatusCode::OK, Json(json!(full_state)))
}

// ── Data CRUD ────────────────────────────────────────────────────────────────

async fn list_data(
    State(state): State<AppState>,
    Path((id, table)): Path<(String, String)>,
) -> impl IntoResponse {
    let manifests = state.plugin_manifests.lock().await;
    let manifest = match manifests.get(&id) {
        Some(m) => m.clone(),
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "Plugin not found or disabled"}))),
    };
    drop(manifests);

    let db = state.db.lock().await;
    match db_ops::list_rows(&db, &manifest, &table) {
        Ok(rows) => (StatusCode::OK, Json(json!(rows))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))),
    }
}

async fn get_data_row(
    State(state): State<AppState>,
    Path((id, table, row_id)): Path<(String, String, i64)>,
) -> impl IntoResponse {
    let manifests = state.plugin_manifests.lock().await;
    let manifest = match manifests.get(&id) {
        Some(m) => m.clone(),
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "Plugin not found or disabled"}))),
    };
    drop(manifests);

    let db = state.db.lock().await;
    match db_ops::get_row(&db, &manifest, &table, row_id) {
        Ok(Some(row)) => (StatusCode::OK, Json(row)),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "Row not found"}))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))),
    }
}

async fn insert_data(
    State(state): State<AppState>,
    Path((id, table)): Path<(String, String)>,
    Json(data): Json<Value>,
) -> impl IntoResponse {
    let manifests = state.plugin_manifests.lock().await;
    let manifest = match manifests.get(&id) {
        Some(m) => m.clone(),
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "Plugin not found or disabled"}))),
    };
    drop(manifests);

    let db = state.db.lock().await;
    match db_ops::insert_row(&db, &manifest, &table, &data) {
        Ok(row) => {
            let row_id = row.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            broadcast_event(
                &state,
                &format!("plugin-data-changed:{}:{}", id, table),
                json!({"pluginId": id, "table": table, "action": "insert", "rowId": row_id}),
            );
            (StatusCode::CREATED, Json(row))
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))),
    }
}

async fn update_data_row(
    State(state): State<AppState>,
    Path((id, table, row_id)): Path<(String, String, i64)>,
    Json(data): Json<Value>,
) -> impl IntoResponse {
    let manifests = state.plugin_manifests.lock().await;
    let manifest = match manifests.get(&id) {
        Some(m) => m.clone(),
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "Plugin not found or disabled"}))),
    };
    drop(manifests);

    let db = state.db.lock().await;
    match db_ops::update_row(&db, &manifest, &table, row_id, &data) {
        Ok(Some(row)) => {
            broadcast_event(
                &state,
                &format!("plugin-data-changed:{}:{}", id, table),
                json!({"pluginId": id, "table": table, "action": "update", "rowId": row_id}),
            );
            (StatusCode::OK, Json(row))
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "Row not found"}))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))),
    }
}

async fn delete_data_row(
    State(state): State<AppState>,
    Path((id, table, row_id)): Path<(String, String, i64)>,
) -> impl IntoResponse {
    let manifests = state.plugin_manifests.lock().await;
    let manifest = match manifests.get(&id) {
        Some(m) => m.clone(),
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "Plugin not found or disabled"}))),
    };
    drop(manifests);

    let db = state.db.lock().await;
    match db_ops::delete_row(&db, &manifest, &table, row_id) {
        Ok(true) => {
            broadcast_event(
                &state,
                &format!("plugin-data-changed:{}:{}", id, table),
                json!({"pluginId": id, "table": table, "action": "delete", "rowId": row_id}),
            );
            (StatusCode::OK, Json(json!({"success": true})))
        }
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({"error": "Row not found"}))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))),
    }
}

async fn query_data(
    State(state): State<AppState>,
    Path((id, table)): Path<(String, String)>,
    Query(filters): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let manifests = state.plugin_manifests.lock().await;
    let manifest = match manifests.get(&id) {
        Some(m) => m.clone(),
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "Plugin not found or disabled"}))),
    };
    drop(manifests);

    let db = state.db.lock().await;
    match db_ops::query_rows(&db, &manifest, &table, &filters) {
        Ok(rows) => (StatusCode::OK, Json(json!(rows))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))),
    }
}

// ── Plugin Popup Trigger ─────────────────────────────────────────────────────

async fn trigger_popup(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<TriggerPopupBody>,
) -> impl IntoResponse {
    let manifests = state.plugin_manifests.lock().await;
    let manifest = match manifests.get(&id) {
        Some(m) => m.clone(),
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "Plugin not found or disabled"}))),
    };
    drop(manifests);

    // Find popup definition in manifest
    let popup_def = match manifest.popups.iter().find(|p| p.template_id == body.template_id) {
        Some(p) => p.clone(),
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "Popup template not found"}))),
    };

    let duration = body.duration.unwrap_or(popup_def.duration);

    // Read template from disk
    let template_path = state
        .app_data_dir
        .join("plugins")
        .join(&id)
        .join(&popup_def.template);
    let template_html = std::fs::read_to_string(&template_path).unwrap_or_default();

    // Build template context with plugin state + trigger context
    let plugin_states = state.plugin_states.lock().await;
    let plugin_st = plugin_states.get(&id).cloned().unwrap_or_default();
    drop(plugin_states);

    let mut ctx = crate::html_template::TemplateContext::new();

    // Add plugin state variables
    let mut plugin_vars = HashMap::new();
    for (key, val) in &plugin_st {
        plugin_vars.insert(key.clone(), value_to_string(val));
    }
    ctx.plugin_contexts.insert(id.clone(), plugin_vars);

    // Add trigger context
    ctx.popup_context = body.context.clone();
    ctx.popup_plugin_id = Some(id.clone());

    // Process template and inject SDK
    let db = state.db.lock().await;
    let processed_html = {
        let processed = crate::html_template::process_template(&template_html, &ctx, Some(&db));
        // Inject SDK (non-live: popups are short-lived but may need fetch helpers like getData)
        crate::plugins::sdk_injection::inject_sdk(&processed, &id, false)
    };
    drop(db);

    // Find the popup's database id
    let db = state.db.lock().await;
    let popup_db_id: Option<i64> = db
        .query_row(
            "SELECT id FROM popups WHERE plugin_id = ?1 AND plugin_template_id = ?2",
            rusqlite::params![id, body.template_id],
            |r| r.get(0),
        )
        .ok();
    drop(db);

    let popup_id = popup_db_id.unwrap_or(0);

    // Update runtime state
    {
        let mut studio_states = state.studio_states.lock().await;
        let studio_state = studio_states.entry(1).or_default();
        studio_state.active_popup_id = Some(popup_id);
        studio_state.active_popup_path = None;
        studio_state.active_popup_duration = duration;
        studio_state.active_popup_direction = Some(popup_def.direction.clone());
        studio_state.active_popup_position = Some(popup_def.position);
        studio_state.active_popup_media_type = Some("html".to_string());
        studio_state.active_popup_html_content = Some(processed_html.clone());
        studio_state.active_popup_width = popup_def.width;
        studio_state.active_popup_height = popup_def.height;
    }

    // Broadcast popup-started (same event as core popups)
    broadcast_event(
        &state,
        "popup-started",
        json!({
            "studioId": 1,
            "popupId": popup_id,
            "imagePath": null,
            "duration": duration,
            "direction": popup_def.direction,
            "position": popup_def.position,
            "mediaType": "html",
            "htmlContent": processed_html,
            "width": popup_def.width,
            "height": popup_def.height,
        }),
    );

    (StatusCode::OK, Json(json!({"success": true})))
}

// ── Asset Serving ────────────────────────────────────────────────────────────

async fn serve_asset(
    State(state): State<AppState>,
    Path((id, path)): Path<(String, String)>,
) -> impl IntoResponse {
    use axum::body::Body;
    use axum::http::{header, Response};

    let file_path = state.app_data_dir.join("plugins").join(&id).join(&path);

    // Security: prevent path traversal
    let canonical = match file_path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap()
        }
    };

    let plugins_dir = match state.app_data_dir.join("plugins").join(&id).canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap()
        }
    };

    if !canonical.starts_with(&plugins_dir) {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::empty())
            .unwrap();
    }

    match tokio::fs::read(&canonical).await {
        Ok(bytes) => {
            let mime = mime_from_ext(canonical.extension().and_then(|e| e.to_str()).unwrap_or(""));
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .header(header::CACHE_CONTROL, "no-cache")
                .body(Body::from(bytes))
                .unwrap()
        }
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap(),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Returns a JSON warning value when any program references screens or popups
/// belonging to `plugin_id`, or `None` when the plugin is not in use.
fn check_plugin_usage_in_programs(conn: &rusqlite::Connection, plugin_id: &str) -> Option<Value> {
    let mut screen_programs: Vec<Value> = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT DISTINCT p.id, p.name FROM programs p \
         JOIN program_screens ps ON ps.program_id = p.id \
         JOIN screens s ON s.id = ps.screen_id \
         WHERE s.plugin_id = ?1",
    ) {
        if let Ok(rows) = stmt.query_map([plugin_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        }) {
            for row in rows.flatten() {
                screen_programs.push(json!({"id": row.0, "name": row.1}));
            }
        }
    }

    let mut popup_programs: Vec<Value> = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT DISTINCT p.id, p.name FROM programs p \
         JOIN program_popups pp ON pp.program_id = p.id \
         JOIN popups po ON po.id = pp.popup_id \
         WHERE po.plugin_id = ?1",
    ) {
        if let Ok(rows) = stmt.query_map([plugin_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        }) {
            for row in rows.flatten() {
                popup_programs.push(json!({"id": row.0, "name": row.1}));
            }
        }
    }

    if screen_programs.is_empty() && popup_programs.is_empty() {
        return None;
    }

    Some(json!({
        "message": "Some programs are using screens or popups from this plugin. They will no longer be available while the plugin is disabled.",
        "affectedPrograms": {
            "screens": screen_programs,
            "popups": popup_programs,
        }
    }))
}

fn broadcast_event(state: &AppState, event: &str, data: Value) {
    if let Ok(guard) = state.io.lock() {
        if let Some(io) = guard.as_ref() {
            let io = io.clone();
            let event = event.to_string();
            tokio::spawn(async move {
                let _ = io.to("studio").emit(&event, &data).await;
            });
        }
    }
}

fn value_to_string(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn mime_from_ext(ext: &str) -> &'static str {
    match ext {
        "js" | "mjs" => "application/javascript",
        "css" => "text/css",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }
}
