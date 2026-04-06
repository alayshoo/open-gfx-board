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

#[tauri::command]
fn close_splashscreen(app: tauri::AppHandle) {
    if let Some(splash) = app.get_webview_window("splashscreen") {
        splash.close().unwrap();
    }
    if let Some(main) = app.get_webview_window("main") {
        main.show().unwrap();
        main.set_focus().unwrap();
    }
}

fn config_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("config.json"))
}

fn read_config(app: &tauri::AppHandle) -> serde_json::Value {
    config_path(app)
        .and_then(|p| std::fs::read_to_string(&p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn write_config(app: &tauri::AppHandle, config: &serde_json::Value) -> Result<(), String> {
    let path = config_path(app).ok_or("cannot resolve app data dir")?;
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_preferred_port(app: tauri::AppHandle) -> Option<u16> {
    read_config(&app)
        .get("preferred_port")
        .and_then(|v| v.as_u64())
        .map(|p| p as u16)
}

#[tauri::command]
fn set_preferred_port(app: tauri::AppHandle, port: Option<u16>) -> Result<(), String> {
    let mut config = read_config(&app);
    match port {
        Some(p) => { config["preferred_port"] = serde_json::json!(p); }
        None => { config.as_object_mut().map(|o| o.remove("preferred_port")); }
    }
    write_config(&app, &config)
}

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

            // Read preferred port from config (set by user in Settings → Server)
            let preferred_port: Option<u16> = std::fs::read_to_string(app_data_dir.join("config.json"))
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| v.get("preferred_port").and_then(|p| p.as_u64()).map(|p| p as u16));

            tauri::async_runtime::spawn(async move {
                // Try preferred port first, then fall back to the auto-discovery list
                let mut ports_to_try: Vec<u16> = Vec::new();
                if let Some(p) = preferred_port {
                    ports_to_try.push(p);
                }
                for p in [5000u16, 5174, 3000, 8080, 8000] {
                    if !ports_to_try.contains(&p) {
                        ports_to_try.push(p);
                    }
                }

                for port in ports_to_try {
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
        .invoke_handler(tauri::generate_handler![
            close_splashscreen,
            get_preferred_port,
            set_preferred_port,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
