mod state;
mod models;
mod keyboard;
mod db;
mod commands;
mod server;

use state::AppState;
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()
                .expect("failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir)?;

            let media_dir = app_data_dir.join("media");
            std::fs::create_dir_all(&media_dir)?;

            let db_path = app_data_dir.join("app.db");
            let conn = rusqlite::Connection::open(&db_path)?;
            db::schema::run_migrations(&conn)?;
            db::studios::ensure_default_studio(&conn)?;

            let app_state = AppState {
                db: Arc::new(Mutex::new(conn)),
                studio_states: Arc::new(Mutex::new(HashMap::new())),
                app_data_dir: app_data_dir.clone(),
                io: Arc::new(std::sync::Mutex::new(None)),
            };

            // Find build dir for static file serving
            let build_dir = app.path().resource_dir().ok()
                .map(|d: PathBuf| d.join("_up_/build"))
                .filter(|d: &PathBuf| d.exists())
                .or_else(|| {
                    let dev_path = PathBuf::from("../build");
                    if dev_path.exists() { Some(dev_path) } else { None }
                });

            let router = server::build_router(app_state, build_dir);

            tauri::async_runtime::spawn(async move {
                let ports = [5000u16, 5174, 3000, 8080, 8000];
                for port in ports {
                    let addr = format!("0.0.0.0:{port}");
                    match tokio::net::TcpListener::bind(&addr).await {
                        Ok(listener) => {
                            println!("Axum server listening on port {port}");
                            axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>()).await
                                .expect("axum server error");
                            return;
                        }
                        Err(_) => continue,
                    }
                }
                eprintln!("Failed to bind to any port");
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
