mod state;
mod models;
mod keyboard;
mod db;
mod commands;
mod server;
pub mod html_template;
pub mod plugins;

use state::AppState;
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use tauri::{Emitter, Manager};

// Holds a pending update so it can be installed when the user confirms.
// The public key is embedded at compile time via the TAURI_SIGNING_PUBLIC_KEY
// environment variable (set as a GitHub Secret in CI).
struct PendingUpdate(Mutex<Option<tauri_plugin_updater::Update>>);

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

/// Returns the pending update version if the startup check already found one.
/// Used by the frontend on mount to catch updates that fired before the listener was ready.
#[tauri::command]
async fn get_pending_update(
    state: tauri::State<'_, PendingUpdate>,
) -> Result<Option<String>, String> {
    let lock = state.0.lock().await;
    Ok(lock.as_ref().map(|u| u.version.clone()))
}

/// Called by the frontend when the user manually requests an update check.
/// Returns the available version string, or None if already up to date.
#[tauri::command]
async fn check_for_updates(
    app: tauri::AppHandle,
    state: tauri::State<'_, PendingUpdate>,
) -> Result<Option<String>, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| e.to_string())?;
    let check = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        updater.check(),
    )
    .await
    .map_err(|_| "Update check timed out".to_string())?
    .map_err(|e| e.to_string())?;

    match check {
        Some(update) => {
            let version = update.version.clone();
            *state.0.lock().await = Some(update);
            let _ = app.emit("update-available", version.clone());
            Ok(Some(version))
        }
        None => Ok(None),
    }
}

/// Called by the frontend when the user accepts the update.
/// Downloads, verifies the Ed25519 signature, installs, and restarts.
#[allow(unreachable_code)]
#[tauri::command]
async fn install_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, PendingUpdate>,
) -> Result<(), String> {
    let update = {
        let mut lock = state.0.lock().await;
        lock.take()
    };

    match update {
        Some(update) => {
            update
                .download_and_install(
                    |_downloaded, _total| { /* progress – could emit an event here */ },
                    || { /* download complete, about to install */ },
                )
                .await
                .map_err(|e| e.to_string())?;
            app.restart();
        }
        None => return Err("No pending update".into()),
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // The updater plugin is always registered; the pubkey and endpoints
    // come from tauri.conf.json (plugins.updater section).
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build());

    builder
        .manage(PendingUpdate(Mutex::new(None)))
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

            // Initialise plugin system
            let plugins_dir = app_data_dir.join("plugins");
            std::fs::create_dir_all(&plugins_dir)?;

            // Locate the bundled plugins directory (resource dir in production, dev path otherwise).
            let bundled_plugins_dir: Option<PathBuf> = app.path().resource_dir().ok()
                .map(|d: PathBuf| d.join("_up_/plugins"))
                .filter(|d: &PathBuf| d.exists())
                .or_else(|| {
                    let dev = PathBuf::from("plugins");
                    if dev.exists() { Some(dev) } else { None }
                });

            // Copy any bundled plugin that hasn't been installed yet, and track their IDs.
            let mut bundled_plugin_ids: Vec<String> = Vec::new();
            if let Some(bundled_dir) = &bundled_plugins_dir {
                if let Ok(entries) = std::fs::read_dir(bundled_dir) {
                    for entry in entries.flatten() {
                        let src = entry.path();
                        if src.is_dir() && src.join("plugin.json").exists() {
                            let name = entry.file_name();
                            let dst = plugins_dir.join(&name);
                            if !dst.exists() {
                                let _ = plugins::manager::copy_dir_public(&src, &dst);
                            }
                            if let Some(id) = name.to_str() {
                                bundled_plugin_ids.push(id.to_string());
                            }
                        }
                    }
                }
            }

            let plugin_manifests = plugins::manager::startup_sync(&app_data_dir, &bundled_plugin_ids, &conn)
                .unwrap_or_else(|e| {
                    eprintln!("Plugin startup error: {e}");
                    HashMap::new()
                });
            let plugin_states = plugins::state::load_all_states(&conn, &plugin_manifests)
                .unwrap_or_default();

            let app_state = AppState {
                db: Arc::new(Mutex::new(conn)),
                studio_states: Arc::new(Mutex::new(HashMap::new())),
                app_data_dir: app_data_dir.clone(),
                io: Arc::new(std::sync::Mutex::new(None)),
                plugin_states: Arc::new(Mutex::new(plugin_states)),
                plugin_manifests: Arc::new(Mutex::new(plugin_manifests)),
                bundled_plugins_dir,
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

            // Check for updates in release builds. Runs in the background so it
            // never delays startup.
            #[cfg(not(debug_assertions))]
            {
                use tauri_plugin_updater::UpdaterExt;
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    match handle.updater() {
                        Ok(updater) => match updater.check().await {
                            Ok(Some(update)) => {
                                let version = update.version.clone();
                                if let Some(state) = handle.try_state::<PendingUpdate>() {
                                    *state.0.lock().await = Some(update);
                                }
                                let _ = handle.emit("update-available", version);
                            }
                            Ok(None) => {}
                            Err(e) => eprintln!("Update check failed: {e}"),
                        },
                        Err(e) => eprintln!("Updater init failed: {e}"),
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            close_splashscreen,
            get_preferred_port,
            set_preferred_port,
            get_pending_update,
            check_for_updates,
            install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
