use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use crate::db::studios::{CommandInput, PresetInput};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_studios))
        .route("/", post(create_studio))
        .route("/{id}", put(update_studio))
        .route("/{id}", delete(delete_studio))
}

async fn list_studios(State(state): State<AppState>) -> impl IntoResponse {
    let db = state.db.lock().await;
    match tokio::task::block_in_place(|| crate::db::studios::get_all_studios(&db)) {
        Ok(studios) => Json(json!({ "studios": studios })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct CreateStudioBody {
    name: String,
}

async fn create_studio(
    State(state): State<AppState>,
    Json(body): Json<CreateStudioBody>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    match tokio::task::block_in_place(|| crate::db::studios::create_studio(&db, &body.name)) {
        Ok(studio) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("studio-created", &json!({ "success": true, "studio": studio })).await;
                }
            }
            // Re-fetch to return fresh data
            match tokio::task::block_in_place(|| crate::db::studios::get_studio(&db, studio.id)) {
                Ok(Some(s)) => Json(json!({ "success": true, "studio": s })).into_response(),
                _ => Json(json!({ "success": true, "studio": studio })).into_response(),
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct UpdateCommandBody {
    #[serde(rename = "obs_command_name")]
    name: Option<String>,
    #[serde(rename = "obs_command_color")]
    color: Option<String>,
    #[serde(rename = "obs_command_shortcut")]
    shortcut: Option<String>,
    #[serde(rename = "obs_command_description")]
    description: Option<String>,
}

#[derive(Deserialize)]
struct UpdatePresetBody {
    name: Option<String>,
    commands: Option<Vec<UpdateCommandBody>>,
}

#[derive(Deserialize)]
struct UpdateStudioBody {
    name: String,
    obs_browser_source_address: Option<String>,
    presets: Option<Vec<UpdatePresetBody>>,
    // Legacy: flat commands list treated as single preset
    obs_commands: Option<Vec<UpdateCommandBody>>,
}

async fn update_studio(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateStudioBody>,
) -> impl IntoResponse {
    let db = state.db.lock().await;

    // Determine obs_browser_source_address
    let obs_addr = body.obs_browser_source_address
        .unwrap_or_else(|| "/obs".to_string());

    // Build presets list
    let presets: Vec<PresetInput> = if let Some(presets_body) = body.presets {
        presets_body.into_iter().map(|p| PresetInput {
            name: p.name.unwrap_or_else(|| "Default".to_string()),
            commands: p.commands.unwrap_or_default().into_iter().map(|c| CommandInput {
                name: c.name.unwrap_or_default(),
                color: c.color.unwrap_or_else(|| "#38bdf8".to_string()),
                shortcut: c.shortcut.unwrap_or_default(),
                description: c.description.unwrap_or_default(),
            }).collect(),
        }).collect()
    } else if let Some(cmds) = body.obs_commands {
        // Legacy flat commands: wrap in single "Default" preset
        vec![PresetInput {
            name: "Default".to_string(),
            commands: cmds.into_iter().map(|c| CommandInput {
                name: c.name.unwrap_or_default(),
                color: c.color.unwrap_or_else(|| "#38bdf8".to_string()),
                shortcut: c.shortcut.unwrap_or_default(),
                description: c.description.unwrap_or_default(),
            }).collect(),
        }]
    } else {
        vec![]
    };

    match tokio::task::block_in_place(|| {
        crate::db::studios::update_studio(&db, id, &body.name, &obs_addr, &presets)
    }) {
        Ok(Some(studio)) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("studio-updated", &json!({ "success": true, "studio": &studio })).await;
                    let _ = io.emit("update-studios", &json!({})).await;
                }
            }
            Json(json!({ "success": true, "studio": studio })).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Studio not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn delete_studio(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    match tokio::task::block_in_place(|| crate::db::studios::delete_studio(&db, id)) {
        Ok(true) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("studio-deleted", &json!({ "success": true, "id": id })).await;
                }
            }
            Json(json!({ "success": true, "id": id })).into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Studio not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}
