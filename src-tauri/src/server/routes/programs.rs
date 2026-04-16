use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use crate::db::programs::{ProgramScreenInput, ProgramPopupInput};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_programs))
        .route("/", post(create_program))
        .route("/{id}", put(update_program))
        .route("/{id}", delete(delete_program))
        .route("/{id}/upload-image", post(upload_program_image))
        .route("/{id}/plugin-prefs", get(get_plugin_prefs).put(set_plugin_prefs))
        .route("/{id}/plugin-popup-overrides", get(get_popup_overrides).put(set_popup_overrides))
}

async fn list_programs(State(state): State<AppState>) -> impl IntoResponse {
    let db = state.db.lock().await;
    match tokio::task::block_in_place(|| crate::db::programs::get_all_programs(&db)) {
        Ok(programs) => Json(json!({ "programs": programs })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct CreateProgramBody {
    name: String,
}

async fn create_program(
    State(state): State<AppState>,
    Json(body): Json<CreateProgramBody>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    match tokio::task::block_in_place(|| crate::db::programs::create_program(&db, &body.name)) {
        Ok(program) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("program-created", &json!({ "success": true, "program": &program })).await;
                    let _ = io.emit("update-programs", &json!({})).await;
                }
            }
            Json(json!({ "success": true, "program": program })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct ProgramScreenBody {
    screen_id: i64,
    layer: Option<i64>,
}

#[derive(Deserialize)]
struct ProgramPopupBody {
    popup_id: i64,
    #[serde(rename = "popup_launch_type")]
    trigger_type: Option<String>,
    duration: Option<i64>,
    frequency: Option<i64>,
    layer: Option<i64>,
}

#[derive(Deserialize)]
struct UpdateProgramBody {
    name: String,
    logo_path: Option<String>,
    #[serde(rename = "background_graphics_path")]
    bg_path: Option<String>,
    /// New screens array with per-screen layer assignments.
    screens: Option<Vec<ProgramScreenBody>>,
    popups: Option<Vec<ProgramPopupBody>>,
}

async fn update_program(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateProgramBody>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    let screens: Vec<ProgramScreenInput> = body.screens.unwrap_or_default().into_iter().map(|s| ProgramScreenInput {
        screen_id: s.screen_id,
        layer: s.layer.unwrap_or(1),
    }).collect();
    let popups: Vec<ProgramPopupInput> = body.popups.unwrap_or_default().into_iter().map(|p| ProgramPopupInput {
        popup_id: p.popup_id,
        trigger_type: p.trigger_type.unwrap_or_else(|| "manual".to_string()),
        duration: p.duration.unwrap_or(10),
        frequency: p.frequency.unwrap_or(1),
        layer: p.layer.unwrap_or(1),
    }).collect();

    match tokio::task::block_in_place(|| {
        crate::db::programs::update_program(
            &db,
            id,
            &body.name,
            body.logo_path.as_deref(),
            body.bg_path.as_deref(),
            &screens,
            &popups,
        )
    }) {
        Ok(Some(program)) => {
            // ── Reconcile active runtime state with new layer assignments ──────────
            // When a running program is edited, any screen or popup that was moved
            // to a different layer must also move in the server's runtime state so
            // that every client that requests get-studio-state sees the correct
            // layer. Screens / popups that were removed from the program are cleared.
            {
                let mut states = state.studio_states.lock().await;
                let runtime = states.entry(1).or_default();

                // Only reconcile when this is the program currently loaded.
                if runtime.program_id == Some(id) {
                    // ── Screens ──────────────────────────────────────────────────
                    for layer_idx in 0..3usize {
                        let layer_num = (layer_idx + 1) as i64;
                        if let Some(screen_id) = runtime.layers[layer_idx].screen_id {
                            let new_layer: Option<i64> = db.query_row(
                                "SELECT layer FROM program_screens WHERE program_id = ?1 AND screen_id = ?2",
                                rusqlite::params![id, screen_id],
                                |r| r.get(0),
                            ).ok();

                            match new_layer {
                                Some(nl) if nl != layer_num => {
                                    // Screen was moved to a different layer — migrate the
                                    // screen fields, leaving popup fields in place.
                                    let new_idx = (nl - 1) as usize;
                                    let scr_id    = runtime.layers[layer_idx].screen_id.take();
                                    let scr_path  = runtime.layers[layer_idx].screen_path.take();
                                    let scr_allow = runtime.layers[layer_idx].screen_allow_popups;
                                    let scr_mt    = runtime.layers[layer_idx].screen_media_type.take();
                                    let scr_html  = runtime.layers[layer_idx].screen_html_content.take();
                                    runtime.layers[layer_idx].screen_allow_popups = false;
                                    runtime.layers[new_idx].screen_id            = scr_id;
                                    runtime.layers[new_idx].screen_path          = scr_path;
                                    runtime.layers[new_idx].screen_allow_popups  = scr_allow;
                                    runtime.layers[new_idx].screen_media_type    = scr_mt;
                                    runtime.layers[new_idx].screen_html_content  = scr_html;
                                }
                                None => {
                                    // Screen was removed from the program entirely.
                                    runtime.layers[layer_idx].screen_id            = None;
                                    runtime.layers[layer_idx].screen_path          = None;
                                    runtime.layers[layer_idx].screen_allow_popups  = false;
                                    runtime.layers[layer_idx].screen_media_type    = None;
                                    runtime.layers[layer_idx].screen_html_content  = None;
                                }
                                _ => {} // Layer unchanged — nothing to do.
                            }
                        }
                    }

                    // ── Popups ───────────────────────────────────────────────────
                    for layer_idx in 0..3usize {
                        let layer_num = (layer_idx + 1) as i64;
                        if let Some(popup_id) = runtime.layers[layer_idx].popup_id {
                            let new_layer: Option<i64> = db.query_row(
                                "SELECT layer FROM program_popups WHERE program_id = ?1 AND popup_id = ?2",
                                rusqlite::params![id, popup_id],
                                |r| r.get(0),
                            ).ok();

                            match new_layer {
                                Some(nl) if nl != layer_num => {
                                    let new_idx = (nl - 1) as usize;
                                    let pp_id   = runtime.layers[layer_idx].popup_id.take();
                                    let pp_path = runtime.layers[layer_idx].popup_path.take();
                                    let pp_dur  = runtime.layers[layer_idx].popup_duration;
                                    let pp_dir  = runtime.layers[layer_idx].popup_direction.take();
                                    let pp_pos  = runtime.layers[layer_idx].popup_position.take();
                                    let pp_mt   = runtime.layers[layer_idx].popup_media_type.take();
                                    let pp_html = runtime.layers[layer_idx].popup_html_content.take();
                                    let pp_w    = runtime.layers[layer_idx].popup_width.take();
                                    let pp_h    = runtime.layers[layer_idx].popup_height.take();
                                    runtime.layers[layer_idx].popup_duration = 0;
                                    runtime.layers[new_idx].popup_id           = pp_id;
                                    runtime.layers[new_idx].popup_path         = pp_path;
                                    runtime.layers[new_idx].popup_duration     = pp_dur;
                                    runtime.layers[new_idx].popup_direction    = pp_dir;
                                    runtime.layers[new_idx].popup_position     = pp_pos;
                                    runtime.layers[new_idx].popup_media_type   = pp_mt;
                                    runtime.layers[new_idx].popup_html_content = pp_html;
                                    runtime.layers[new_idx].popup_width        = pp_w;
                                    runtime.layers[new_idx].popup_height       = pp_h;
                                }
                                None => {
                                    runtime.layers[layer_idx].popup_id           = None;
                                    runtime.layers[layer_idx].popup_path         = None;
                                    runtime.layers[layer_idx].popup_duration     = 0;
                                    runtime.layers[layer_idx].popup_direction    = None;
                                    runtime.layers[layer_idx].popup_position     = None;
                                    runtime.layers[layer_idx].popup_media_type   = None;
                                    runtime.layers[layer_idx].popup_html_content = None;
                                    runtime.layers[layer_idx].popup_width        = None;
                                    runtime.layers[layer_idx].popup_height       = None;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            } // studio_states lock released here

            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("program-updated", &json!({ "success": true, "program": &program })).await;
                    let _ = io.emit("update-programs", &json!({})).await;
                }
            }
            Json(json!({ "success": true, "program": program })).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Program not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn delete_program(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    match tokio::task::block_in_place(|| crate::db::programs::delete_program(&db, id)) {
        Ok(true) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("program-deleted", &json!({ "success": true, "id": id })).await;
                    let _ = io.emit("update-programs", &json!({})).await;
                }
            }
            Json(json!({ "success": true, "id": id })).into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Program not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn upload_program_image(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_ext = "bin".to_string();
    let mut upload_type = "logo".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "image" => {
                let filename = field.file_name().unwrap_or("upload.bin").to_string();
                file_ext = filename.rsplit('.').next().unwrap_or("bin").to_lowercase();
                file_bytes = field.bytes().await.ok().map(|b| b.to_vec());
            }
            "upload_type" => {
                if let Ok(val) = field.text().await {
                    upload_type = val;
                }
            }
            _ => {}
        }
    }

    let Some(bytes) = file_bytes else {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "No image provided" }))).into_response();
    };

    let sub = match upload_type.as_str() {
        "background" => "backgrounds",
        _ => "logos",
    };
    let rel_dir = format!("media/programs/{id}/{sub}");
    let abs_dir = state.app_data_dir.join(&rel_dir);
    if let Err(e) = tokio::fs::create_dir_all(&abs_dir).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response();
    }

    let filename = format!("{}.{}", Uuid::new_v4(), file_ext);
    let abs_path = abs_dir.join(&filename);
    if let Err(e) = tokio::fs::write(&abs_path, &bytes).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response();
    }

    let rel_path = format!("{rel_dir}/{filename}");

    // Update DB
    let db = state.db.lock().await;
    let rel_path_clone = rel_path.clone();
    let update_result = tokio::task::block_in_place(|| {
        if upload_type == "background" {
            db.execute(
                "UPDATE programs SET bg_path = ?1 WHERE id = ?2",
                rusqlite::params![rel_path_clone, id],
            ).map_err(anyhow::Error::from)
        } else {
            db.execute(
                "UPDATE programs SET logo_path = ?1 WHERE id = ?2",
                rusqlite::params![rel_path_clone, id],
            ).map_err(anyhow::Error::from)
        }
    });

    match update_result {
        Ok(_) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("update-programs", &json!({})).await;
                }
            }
            Json(json!({ "success": true, "imagePath": rel_path })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

// ── Per-program plugin preferences ───────────────────────────────────────────

#[derive(Deserialize)]
struct SetPluginPrefsBody {
    plugin_ids: Vec<String>,
}

async fn get_plugin_prefs(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    let result = tokio::task::block_in_place(|| {
        db.query_row(
            "SELECT plugin_ids FROM program_plugin_prefs WHERE program_id = ?1",
            rusqlite::params![id],
            |r| r.get::<_, String>(0),
        )
    });
    match result {
        Ok(json_str) => {
            let ids: Vec<String> = serde_json::from_str(&json_str).unwrap_or_default();
            Json(json!({ "plugin_ids": ids })).into_response()
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let empty: Vec<String> = vec![];
            Json(json!({ "plugin_ids": empty })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn set_plugin_prefs(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<SetPluginPrefsBody>,
) -> impl IntoResponse {
    let json_str = match serde_json::to_string(&body.plugin_ids) {
        Ok(s) => s,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    };
    let db = state.db.lock().await;
    let result = tokio::task::block_in_place(|| {
        db.execute(
            "INSERT INTO program_plugin_prefs (program_id, plugin_ids) VALUES (?1, ?2)
             ON CONFLICT(program_id) DO UPDATE SET plugin_ids = excluded.plugin_ids",
            rusqlite::params![id, json_str],
        )
    });
    match result {
        Ok(_) => Json(json!({ "success": true })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

// ── Per-program plugin popup overrides ───────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
struct PluginPopupOverride {
    plugin_id: String,
    template_id: String,
    popup_id: Option<i64>,
    duration: i64,
}

#[derive(Deserialize)]
struct SetPopupOverridesBody {
    overrides: Vec<PluginPopupOverride>,
}

async fn get_popup_overrides(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    let result = tokio::task::block_in_place(|| {
        let mut stmt = db.prepare(
            "SELECT plugin_id, template_id, popup_id, duration \
             FROM program_plugin_popup_overrides \
             WHERE program_id = ?1 \
             ORDER BY plugin_id, template_id",
        )?;
        let overrides: Vec<PluginPopupOverride> = stmt
            .query_map([id], |r| {
                Ok(PluginPopupOverride {
                    plugin_id: r.get(0)?,
                    template_id: r.get(1)?,
                    popup_id: r.get(2)?,
                    duration: r.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok::<_, anyhow::Error>(overrides)
    });
    match result {
        Ok(overrides) => Json(json!({ "overrides": overrides })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn set_popup_overrides(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<SetPopupOverridesBody>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    let result = tokio::task::block_in_place(|| {
        // Replace all overrides for this program atomically.
        db.execute(
            "DELETE FROM program_plugin_popup_overrides WHERE program_id = ?1",
            [id],
        )?;
        for ov in &body.overrides {
            db.execute(
                "INSERT INTO program_plugin_popup_overrides \
                     (program_id, plugin_id, template_id, popup_id, duration) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, &ov.plugin_id, &ov.template_id, ov.popup_id, ov.duration],
            )?;
        }
        Ok::<_, anyhow::Error>(())
    });
    match result {
        Ok(_) => Json(json!({ "success": true })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}
