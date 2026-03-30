use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use socketioxide::SocketIo;

#[derive(Debug, Clone, Default)]
pub struct StudioRuntimeState {
    pub program_id: Option<i64>,
    // Active overlay
    pub active_screen_id: Option<i64>,
    pub active_screen_path: Option<String>,
    pub active_screen_allow_ads: bool,
    // Active ad
    pub active_ad_id: Option<i64>,
    pub active_ad_path: Option<String>,
    pub active_ad_duration: i64,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<rusqlite::Connection>>,
    pub studio_states: Arc<Mutex<HashMap<i64, StudioRuntimeState>>>,
    pub app_data_dir: PathBuf,
    pub io: Arc<std::sync::Mutex<Option<SocketIo>>>,
}
