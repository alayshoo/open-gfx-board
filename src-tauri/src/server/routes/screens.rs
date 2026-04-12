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
        .route("/", get(list_screens))
        .route("/", post(create_screen))
        .route("/upload-image", post(upload_screen_image))
        .route("/{id}", put(update_screen))
        .route("/{id}", delete(delete_screen))
        .route("/{id}/duplicate", post(duplicate_screen))
}

async fn list_screens(State(state): State<AppState>) -> impl IntoResponse {
    let db = state.db.lock().await;
    match tokio::task::block_in_place(|| crate::db::screens::get_all_screens(&db)) {
        Ok(screens) => Json(json!({ "screens": screens })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct CreateScreenBody {
    name: String,
    comments: Option<String>,
    allow_popups: Option<bool>,
    media_type: Option<String>,
    html_content: Option<String>,
}

async fn create_screen(
    State(state): State<AppState>,
    Json(body): Json<CreateScreenBody>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    let comments = body.comments.as_deref().unwrap_or("");
    let allow_popups = body.allow_popups.unwrap_or(true);
    let media_type = body.media_type.as_deref().unwrap_or("image");
    let html_content = body.html_content.as_deref();
    match tokio::task::block_in_place(|| crate::db::screens::create_screen(&db, &body.name, comments, allow_popups, media_type, html_content)) {
        Ok(screen) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("screen-created", &json!({ "success": true, "screen": &screen })).await;
                    let _ = io.emit("update-screens", &json!({})).await;
                }
            }
            Json(json!({ "success": true, "screen": screen })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct UpdateScreenBody {
    name: String,
    comments: Option<String>,
    allow_popups: Option<bool>,
    media_type: Option<String>,
    html_content: Option<String>,
}

async fn update_screen(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateScreenBody>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    let comments = body.comments.as_deref().unwrap_or("");
    let allow_popups = body.allow_popups.unwrap_or(true);
    let media_type = body.media_type.as_deref().unwrap_or("image");
    let html_content = body.html_content.as_deref();
    match tokio::task::block_in_place(|| crate::db::screens::update_screen(&db, id, &body.name, comments, allow_popups, media_type, html_content)) {
        Ok(Some(screen)) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("screen-updated", &json!({ "success": true, "screen": &screen })).await;
                    let _ = io.emit("update-screens", &json!({})).await;
                }
            }
            Json(json!({ "success": true, "screen": screen })).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Screen not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn delete_screen(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    match tokio::task::block_in_place(|| crate::db::screens::delete_screen(&db, id)) {
        Ok(true) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("screen-deleted", &json!({ "success": true, "id": id })).await;
                    let _ = io.emit("update-screens", &json!({})).await;
                }
            }
            Json(json!({ "success": true, "id": id })).into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Screen not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn duplicate_screen(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    match tokio::task::block_in_place(|| crate::db::screens::duplicate_screen(&db, id)) {
        Ok(Some(screen)) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("screen-created", &json!({ "success": true, "screen": &screen })).await;
                    let _ = io.emit("update-screens", &json!({})).await;
                }
            }
            Json(json!({ "success": true, "screen": screen })).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Screen not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn upload_screen_image(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_ext = "bin".to_string();
    let mut screen_id: Option<i64> = None;

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
                    screen_id = val.parse().ok();
                }
            }
            _ => {}
        }
    }

    let Some(bytes) = file_bytes else {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "No image provided" }))).into_response();
    };
    let Some(id) = screen_id else {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "No id provided" }))).into_response();
    };

    let rel_dir = format!("media/screens/{id}");
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
    match tokio::task::block_in_place(|| crate::db::screens::set_media_path(&db, id, &rel_path_clone)) {
        Ok(_) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("update-screens", &json!({})).await;
                }
            }
            Json(json!({ "success": true, "imagePath": rel_path })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}
