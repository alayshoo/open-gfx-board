use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_popups))
        .route("/", post(create_popup))
        .route("/upload-image", post(upload_popup_image))
        .route("/{id}", put(update_popup))
        .route("/{id}", delete(delete_popup))
        .route("/{id}/duplicate", post(duplicate_popup))
}

async fn list_popups(State(state): State<AppState>) -> impl IntoResponse {
    let db = state.db.lock().await;
    match tokio::task::block_in_place(|| crate::db::popups::get_all_popups(&db)) {
        Ok(popups) => Json(json!({ "popups": popups })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct CreatePopupBody {
    name: String,
    sponsor_name: Option<String>,
    comments: Option<String>,
    direction: Option<String>,
    position: Option<f64>,
    media_type: Option<String>,
    html_content: Option<String>,
    width: Option<i64>,
    height: Option<i64>,
    direction_vertical: Option<String>,
    position_vertical: Option<f64>,
}

async fn create_popup(
    State(state): State<AppState>,
    Json(body): Json<CreatePopupBody>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    let sponsor = body.sponsor_name.as_deref().unwrap_or("");
    let comments = body.comments.as_deref().unwrap_or("");
    let direction = body.direction.as_deref().unwrap_or("bottom");
    let position = body.position.unwrap_or(50.0);
    let media_type = body.media_type.as_deref().unwrap_or("image");
    let html_content = body.html_content.as_deref();
    let direction_vertical = body.direction_vertical.as_deref();
    let position_vertical = body.position_vertical;
    match tokio::task::block_in_place(|| crate::db::popups::create_popup(&db, &body.name, sponsor, comments, direction, position, media_type, html_content, body.width, body.height, direction_vertical, position_vertical)) {
        Ok(popup) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("popup-created", &json!({ "success": true, "popup": &popup })).await;
                    let _ = io.emit("update-popups", &json!({})).await;
                }
            }
            Json(json!({ "success": true, "popup": popup })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct UpdatePopupBody {
    name: String,
    sponsor_name: Option<String>,
    comments: Option<String>,
    direction: Option<String>,
    position: Option<f64>,
    media_type: Option<String>,
    html_content: Option<String>,
    width: Option<i64>,
    height: Option<i64>,
    direction_vertical: Option<String>,
    position_vertical: Option<f64>,
}

async fn update_popup(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdatePopupBody>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    let sponsor = body.sponsor_name.as_deref().unwrap_or("");
    let comments = body.comments.as_deref().unwrap_or("");
    let direction = body.direction.as_deref().unwrap_or("bottom");
    let position = body.position.unwrap_or(50.0);
    let media_type = body.media_type.as_deref().unwrap_or("image");
    let html_content = body.html_content.as_deref();
    let direction_vertical = body.direction_vertical.as_deref();
    let position_vertical = body.position_vertical;
    match tokio::task::block_in_place(|| crate::db::popups::update_popup(&db, id, &body.name, sponsor, comments, direction, position, media_type, html_content, body.width, body.height, direction_vertical, position_vertical)) {
        Ok(Some(popup)) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("popup-updated", &json!({ "success": true, "popup": &popup })).await;
                    let _ = io.emit("update-popups", &json!({})).await;
                }
            }
            Json(json!({ "success": true, "popup": popup })).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Popup not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn delete_popup(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    match tokio::task::block_in_place(|| crate::db::popups::delete_popup(&db, id)) {
        Ok(true) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("popup-deleted", &json!({ "success": true, "id": id })).await;
                    let _ = io.emit("update-popups", &json!({})).await;
                }
            }
            Json(json!({ "success": true, "id": id })).into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Popup not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn duplicate_popup(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    match tokio::task::block_in_place(|| crate::db::popups::duplicate_popup(&db, id)) {
        Ok(Some(popup)) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("popup-created", &json!({ "success": true, "popup": &popup })).await;
                    let _ = io.emit("update-popups", &json!({})).await;
                }
            }
            Json(json!({ "success": true, "popup": popup })).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Pop-up not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn upload_popup_image(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_ext = "bin".to_string();
    let mut popup_id: Option<i64> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "image" => {
                let filename = field.file_name().unwrap_or("upload.bin").to_string();
                file_ext = filename.rsplit('.').next().unwrap_or("bin").to_lowercase();
                file_bytes = field.bytes().await.ok().map(|b| b.to_vec());
            }
            "id" => {
                if let Ok(val) = field.text().await {
                    popup_id = val.parse().ok();
                }
            }
            _ => {}
        }
    }

    let Some(bytes) = file_bytes else {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "No image provided" }))).into_response();
    };
    let Some(id) = popup_id else {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "No id provided" }))).into_response();
    };

    let rel_dir = format!("media/popups/{id}");
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
    let db = state.db.lock().await;
    let rel_path_clone = rel_path.clone();
    match tokio::task::block_in_place(|| crate::db::popups::set_media_path(&db, id, &rel_path_clone)) {
        Ok(_) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("update-popups", &json!({})).await;
                }
            }
            Json(json!({ "success": true, "imagePath": rel_path })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}
