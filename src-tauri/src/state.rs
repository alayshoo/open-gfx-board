use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use socketioxide::SocketIo;

#[derive(Debug, Clone, Default)]
pub struct StudioRuntimeState {
    pub program_id: Option<i64>,
    pub active_screen_id: Option<i64>,
    pub active_ad_id: Option<i64>,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<rusqlite::Connection>>,
    pub studio_states: Arc<Mutex<HashMap<i64, StudioRuntimeState>>>,
    pub app_data_dir: PathBuf,
    pub io: Arc<std::sync::Mutex<Option<SocketIo>>>,
}
