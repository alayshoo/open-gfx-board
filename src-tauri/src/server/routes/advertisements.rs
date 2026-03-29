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
        .route("/", get(list_ads))
        .route("/", post(create_ad))
        .route("/upload-image", post(upload_ad_image))
        .route("/{id}", put(update_ad))
        .route("/{id}", delete(delete_ad))
}

async fn list_ads(State(state): State<AppState>) -> impl IntoResponse {
    let db = state.db.lock().await;
    match tokio::task::block_in_place(|| crate::db::advertisements::get_all_ads(&db)) {
        Ok(ads) => Json(json!({ "ads": ads })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct CreateAdBody {
    name: String,
    sponsor_name: Option<String>,
    comments: Option<String>,
}

async fn create_ad(
    State(state): State<AppState>,
    Json(body): Json<CreateAdBody>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    let sponsor = body.sponsor_name.as_deref().unwrap_or("");
    let comments = body.comments.as_deref().unwrap_or("");
    match tokio::task::block_in_place(|| crate::db::advertisements::create_ad(&db, &body.name, sponsor, comments)) {
        Ok(ad) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("ad-created", &json!({ "success": true, "ad": &ad })).await;
                }
            }
            Json(json!({ "success": true, "ad": ad })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct UpdateAdBody {
    name: String,
    sponsor_name: Option<String>,
    comments: Option<String>,
}

async fn update_ad(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateAdBody>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    let sponsor = body.sponsor_name.as_deref().unwrap_or("");
    let comments = body.comments.as_deref().unwrap_or("");
    match tokio::task::block_in_place(|| crate::db::advertisements::update_ad(&db, id, &body.name, sponsor, comments)) {
        Ok(Some(ad)) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("ad-updated", &json!({ "success": true, "ad": &ad })).await;
                }
            }
            Json(json!({ "success": true, "ad": ad })).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Ad not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn delete_ad(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    match tokio::task::block_in_place(|| crate::db::advertisements::delete_ad(&db, id)) {
        Ok(true) => {
            {
                let io_clone = state.io.lock().ok().and_then(|g| g.clone());
                if let Some(io) = io_clone {
                    let _ = io.emit("ad-deleted", &json!({ "success": true, "id": id })).await;
                }
            }
            Json(json!({ "success": true, "id": id })).into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Ad not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn upload_ad_image(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_ext = "bin".to_string();
    let mut ad_id: Option<i64> = None;

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
                    ad_id = val.parse().ok();
                }
            }
            _ => {}
        }
    }

    let Some(bytes) = file_bytes else {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "No image provided" }))).into_response();
    };
    let Some(id) = ad_id else {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "No id provided" }))).into_response();
    };

    let rel_dir = format!("media/advertisements/{id}");
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
    match tokio::task::block_in_place(|| crate::db::advertisements::set_media_path(&db, id, &rel_path_clone)) {
        Ok(_) => Json(json!({ "success": true, "imagePath": rel_path })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}
