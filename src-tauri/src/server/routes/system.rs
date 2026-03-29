use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::io::Write;
use crate::state::AppState;

pub async fn has_data_handler(State(state): State<AppState>) -> impl IntoResponse {
    let db = state.db.lock().await;
    let has_data = tokio::task::block_in_place(|| -> bool {
        let studios: i64 = db
            .query_row("SELECT COUNT(*) FROM studios", [], |r| r.get(0))
            .unwrap_or(0);
        let programs: i64 = db
            .query_row("SELECT COUNT(*) FROM programs", [], |r| r.get(0))
            .unwrap_or(0);
        studios > 0 || programs > 0
    });
    Json(json!({ "has_data": has_data }))
}

pub async fn health_handler() -> impl IntoResponse {
    Json(json!({ "ok": true }))
}

pub async fn local_ip_handler() -> impl IntoResponse {
    let ip = get_local_ip();
    Json(json!({ "ip": ip }))
}

fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    Some(addr.ip().to_string())
}

pub async fn serve_media(
    Path(path): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let full_path = state.app_data_dir.join("media").join(&path);
    match tokio::fs::read(&full_path).await {
        Ok(data) => {
            let ext = full_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let content_type = content_type_for_ext(&ext);
            Response::builder()
                .header(header::CONTENT_TYPE, content_type)
                .body(Body::from(data))
                .unwrap()
                .into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

fn content_type_for_ext(ext: &str) -> &'static str {
    match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }
}

pub async fn export_handler(State(state): State<AppState>) -> impl IntoResponse {
    let app_data_dir = state.app_data_dir.clone();
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
        let mut buf = Vec::new();
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // Add database file
        let db_path = app_data_dir.join("app.db");
        if db_path.exists() {
            zip.start_file("app.db", options)?;
            let db_bytes = std::fs::read(&db_path)?;
            zip.write_all(&db_bytes)?;
        }

        // Add media files
        let media_dir = app_data_dir.join("media");
        if media_dir.exists() {
            add_dir_to_zip(&mut zip, &media_dir, &app_data_dir, options)?;
        }

        zip.finish()?;
        Ok(buf)
    }).await;

    match result {
        Ok(Ok(data)) => Response::builder()
            .header(header::CONTENT_TYPE, "application/zip")
            .header(header::CONTENT_DISPOSITION, "attachment; filename=\"open-gfx-board-export.zip\"")
            .body(Body::from(data))
            .unwrap()
            .into_response(),
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

fn add_dir_to_zip(
    zip: &mut zip::ZipWriter<std::io::Cursor<&mut Vec<u8>>>,
    dir: &std::path::Path,
    base: &std::path::Path,
    options: zip::write::SimpleFileOptions,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(base).unwrap_or(&path);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            zip.add_directory(&rel_str, options)?;
            add_dir_to_zip(zip, &path, base, options)?;
        } else {
            zip.start_file(&rel_str, options)?;
            let bytes = std::fs::read(&path)?;
            zip.write_all(&bytes)?;
        }
    }
    Ok(())
}

pub async fn import_handler(
    State(state): State<AppState>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let app_data_dir = state.app_data_dir.clone();
    let db_arc = state.db.clone();

    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let cursor = std::io::Cursor::new(body.to_vec());
        let mut archive = zip::ZipArchive::new(cursor)?;

        // Extract all files
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = app_data_dir.join(file.name());

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        // Re-open the DB connection with the new file
        // We can't easily replace the Arc<Mutex<Connection>>, but we can re-run migrations
        // The DB file is already replaced on disk; we need to checkpoint
        if let Ok(db) = db_arc.try_lock() {
            let _ = crate::db::schema::run_migrations(&db);
        }

        Ok(())
    }).await;

    match result {
        Ok(Ok(())) => Json(json!({ "success": true })).into_response(),
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}
