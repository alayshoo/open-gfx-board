use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use socketioxide::SocketIo;

use crate::plugins::manifest::PluginManifest;

/// Runtime state for one active layer (screen overlay + popup).
#[derive(Debug, Clone, Default)]
pub struct LayerState {
    // Active screen overlay on this layer
    pub screen_id: Option<i64>,
    pub screen_path: Option<String>,
    pub screen_allow_popups: bool,
    pub screen_media_type: Option<String>,
    /// Processed HTML (template already resolved) for the active screen overlay.
    pub screen_html_content: Option<String>,
    // Active popup on this layer
    pub popup_id: Option<i64>,
    pub popup_path: Option<String>,
    pub popup_duration: i64,
    pub popup_direction: Option<String>,
    pub popup_position: Option<f64>,
    pub popup_media_type: Option<String>,
    /// Processed HTML (template already resolved) for the active popup.
    pub popup_html_content: Option<String>,
    pub popup_width: Option<i64>,
    pub popup_height: Option<i64>,
}

/// Full runtime state for one studio.
/// `layers` is indexed 0..2 where index 0 = layer 1 (top), index 2 = layer 3 (bottom).
#[derive(Debug, Clone)]
pub struct StudioRuntimeState {
    pub program_id: Option<i64>,
    /// Per-layer state.  Index 0 → layer 1, index 1 → layer 2, index 2 → layer 3.
    pub layers: [LayerState; 3],
}

impl Default for StudioRuntimeState {
    fn default() -> Self {
        Self {
            program_id: None,
            layers: [LayerState::default(), LayerState::default(), LayerState::default()],
        }
    }
}

impl StudioRuntimeState {
    /// Return a mutable reference to the layer state for the given layer number
    /// (1–3).  Clamps out-of-range values to layer 1.
    pub fn layer_mut(&mut self, layer: i64) -> &mut LayerState {
        let idx = (layer.clamp(1, 3) - 1) as usize;
        &mut self.layers[idx]
    }

    /// Return a shared reference to the layer state for the given layer number.
    #[allow(dead_code)]
    pub fn layer(&self, layer: i64) -> &LayerState {
        let idx = (layer.clamp(1, 3) - 1) as usize;
        &self.layers[idx]
    }

    /// Reset everything (program change / clear).
    pub fn reset(&mut self) {
        self.layers = [LayerState::default(), LayerState::default(), LayerState::default()];
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<rusqlite::Connection>>,
    pub studio_states: Arc<Mutex<HashMap<i64, StudioRuntimeState>>>,
    pub app_data_dir: PathBuf,
    pub io: Arc<std::sync::Mutex<Option<SocketIo>>>,
    /// In-memory cache: plugin_id -> (key -> JSON value).
    pub plugin_states: Arc<Mutex<HashMap<String, HashMap<String, serde_json::Value>>>>,
    /// Cached manifests for enabled plugins.
    pub plugin_manifests: Arc<Mutex<HashMap<String, PluginManifest>>>,
    /// Directory containing the original bundled plugin sources (None when not available).
    pub bundled_plugins_dir: Option<PathBuf>,
}
