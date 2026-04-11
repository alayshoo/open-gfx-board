use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use socketioxide::SocketIo;

#[derive(Debug, Clone, Default)]
pub struct StudioRuntimeState {
    pub program_id: Option<i64>,
    // Active overlay
    pub active_screen_id: Option<i64>,
    pub active_screen_path: Option<String>,
    pub active_screen_allow_popups: bool,
    pub active_screen_media_type: Option<String>,
    /// Processed HTML (template already resolved) for the active screen overlay.
    pub active_screen_html_content: Option<String>,
    // Active popup
    pub active_popup_id: Option<i64>,
    pub active_popup_path: Option<String>,
    pub active_popup_duration: i64,
    pub active_popup_direction: Option<String>,
    pub active_popup_position: Option<i64>,
    pub active_popup_media_type: Option<String>,
    /// Processed HTML (template already resolved) for the active popup.
    pub active_popup_html_content: Option<String>,
    pub active_popup_width: Option<i64>,
    pub active_popup_height: Option<i64>,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<rusqlite::Connection>>,
    pub studio_states: Arc<Mutex<HashMap<i64, StudioRuntimeState>>>,
    pub app_data_dir: PathBuf,
    pub io: Arc<std::sync::Mutex<Option<SocketIo>>>,
}
