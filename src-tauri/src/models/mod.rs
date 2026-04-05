use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsCommand {
    pub id: Option<i64>,
    pub preset_id: Option<i64>,
    pub studio_id: Option<i64>,
    #[serde(rename = "obs_command_name")]
    pub name: String,
    #[serde(rename = "obs_command_color")]
    pub color: String,
    #[serde(rename = "obs_command_shortcut")]
    pub shortcut: String,
    #[serde(rename = "obs_command_description")]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub id: Option<i64>,
    pub studio_id: Option<i64>,
    pub name: String,
    pub commands: Vec<ObsCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Studio {
    pub id: i64,
    pub name: String,
    pub obs_browser_source_address: String,
    pub presets: Vec<Preset>,
    pub commands: Vec<ObsCommand>, // flattened from all presets
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screen {
    pub id: i64,
    #[serde(rename = "graphics_name")]
    pub name: String,
    pub comments: String,
    #[serde(rename = "graphics_path")]
    pub media_path: Option<String>,
    pub media_type: String,
    pub allow_popups: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Popup {
    pub id: i64,
    pub name: String,
    pub sponsor_name: String,
    pub comments: String,
    #[serde(rename = "image_path")]
    pub media_path: Option<String>,
    pub media_type: String,
    pub direction: String,
    pub position: i64,
    pub programs: Vec<PopupProgram>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupProgram {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramPopup {
    pub id: i64,
    pub program_id: i64,
    pub popup_id: i64,
    #[serde(rename = "popup_launch_type")]
    pub trigger_type: String,
    pub duration: i64,
    pub frequency: i64,
    pub popup: Option<Popup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub id: i64,
    pub name: String,
    pub logo_path: Option<String>,
    #[serde(rename = "background_graphics_path")]
    pub bg_path: Option<String>,
    #[serde(rename = "graphics")]
    pub screens: Vec<Screen>,
    pub program_popups: Vec<ProgramPopup>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioState {
    #[serde(rename = "studioId")]
    pub studio_id: i64,
    #[serde(rename = "programId")]
    pub program_id: Option<i64>,
    pub program: Option<Program>,
    #[serde(rename = "activeOverlay")]
    pub active_overlay: Option<ActiveOverlay>,
    #[serde(rename = "activePopup")]
    pub active_popup: Option<ActivePopup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveOverlay {
    #[serde(rename = "graphicId")]
    pub graphic_id: i64,
    #[serde(rename = "graphicPath")]
    pub graphic_path: Option<String>,
    #[serde(rename = "allowPopups")]
    pub allow_popups: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePopup {
    #[serde(rename = "popupId")]
    pub popup_id: i64,
    #[serde(rename = "imagePath")]
    pub image_path: Option<String>,
    pub duration: i64,
    pub direction: String,
    pub position: i64,
}
