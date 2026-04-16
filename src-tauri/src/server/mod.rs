//src-tauri/src/server/mod.rs

pub mod routes;
pub mod socket_handlers;

use std::path::PathBuf;
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    services::{ServeDir, ServeFile},
};
use socketioxide::SocketIo;
use crate::state::AppState;
use routes::{studios, programs, popups, screens, system, plugins};

pub fn build_router(app_state: AppState, build_dir: Option<PathBuf>) -> Router {
    let (socket_layer, io) = SocketIo::builder()
        .with_state(app_state.clone())
        .build_layer();

    // Register socket handlers
    socket_handlers::register_handlers(&io, app_state.clone());

    // Store io in state (set once at startup, std::sync::Mutex is fine here)
    if let Ok(mut guard) = app_state.io.lock() {
        *guard = Some(io);
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

        let mut api_router = Router::new()
        .nest("/studios", studios::router())
        .nest("/programs", programs::router())
        .nest("/popups", popups::router())
        .nest("/screens", screens::router())
        .nest("/plugins", plugins::router())
        .route("/has-data", get(system::has_data_handler))
        .route("/health", get(system::health_handler))
        .route("/local-ip", get(system::local_ip_handler))
        .route("/export", get(system::export_handler))
        .route("/import", post(system::import_handler))
        .route("/media/{*path}", get(system::serve_media))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100 MB
        .layer(cors)
        .with_state(app_state);

    if let Some(dir) = build_dir {
        if dir.exists() {
            let index = dir.join("index.html");
            api_router = api_router.fallback_service(
                ServeDir::new(&dir).not_found_service(ServeFile::new(index))
            );
        }
    }

    api_router.layer(socket_layer)
}
