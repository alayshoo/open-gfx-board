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
pub struct ScreenProgram {
    pub id: i64,
    pub name: String,
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
    /// Raw HTML template. Only populated when `media_type` is `"html"`.
    /// May contain `{{var:…}}` / `{{db:…}}` expressions resolved at display time.
    pub html_content: Option<String>,
    pub programs: Vec<ScreenProgram>,
    pub created_at: String,
    /// Set when this screen was installed by a plugin; `None` for user-created screens.
    pub plugin_id: Option<String>,
    /// The template id within the plugin manifest that produced this screen.
    pub plugin_template_id: Option<String>,
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
    /// Raw HTML template. Only populated when `media_type` is `"html"`.
    pub html_content: Option<String>,
    pub direction: String,
    pub position: f64,
    /// Explicit width in pixels.  When `None`, image/video popups use their
    /// natural media dimensions; HTML popups fall back to a client-side default.
    pub width: Option<i64>,
    /// Explicit height in pixels.  Same fallback rules as `width`.
    pub height: Option<i64>,
    pub programs: Vec<PopupProgram>,
    pub created_at: String,
    /// Set when this pop-up was installed by a plugin; `None` for user-created pop-ups.
    pub plugin_id: Option<String>,
    /// The template id within the plugin manifest that produced this pop-up.
    pub plugin_template_id: Option<String>,
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
    #[serde(rename = "activePopUp")]
    pub active_popup: Option<ActivePopup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveOverlay {
    #[serde(rename = "graphicId")]
    pub graphic_id: i64,
    #[serde(rename = "graphicPath")]
    pub graphic_path: Option<String>,
    #[serde(rename = "allowPopUps")]
    pub allow_popups: bool,
    #[serde(rename = "mediaType")]
    pub media_type: String,
    /// Processed HTML content (template expressions already resolved).
    /// Only present when `media_type` is `"html"`.
    #[serde(rename = "htmlContent")]
    pub html_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePopup {
    #[serde(rename = "popupId")]
    pub popup_id: i64,
    #[serde(rename = "imagePath")]
    pub image_path: Option<String>,
    pub duration: i64,
    pub direction: String,
    pub position: f64,
    #[serde(rename = "mediaType")]
    pub media_type: String,
    /// Processed HTML content (template expressions already resolved).
    /// Only present when `media_type` is `"html"`.
    #[serde(rename = "htmlContent")]
    pub html_content: Option<String>,
    /// Explicit popup width in pixels (None = use natural media size or client default).
    pub width: Option<i64>,
    /// Explicit popup height in pixels.
    pub height: Option<i64>,
}
