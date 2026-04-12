use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level plugin manifest (`plugin.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,

    /// Database table definitions.  Key = logical table name (e.g. "teams").
    /// The actual SQLite table will be created as `plugin_{id}_{key}`.
    #[serde(default)]
    pub database: HashMap<String, PluginTable>,

    /// Runtime state variables synced over websockets.
    #[serde(default)]
    pub state: HashMap<String, PluginStateVar>,

    /// Custom event names the plugin can fire / listen to.
    #[serde(default)]
    pub events: Vec<String>,

    /// Control panel web component.
    pub control: Option<PluginComponent>,

    /// Editor page web component.
    pub editor: Option<PluginComponent>,

    /// Screen (persistent overlay) templates.
    #[serde(default)]
    pub screens: Vec<PluginScreenDef>,

    /// Popup (temporary overlay) templates.
    #[serde(default)]
    pub popups: Vec<PluginPopupDef>,
}

/// A database table declared by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginTable {
    /// column_name -> raw SQLite column definition
    /// e.g. `"name": "TEXT NOT NULL"`, `"team_id": "INTEGER NOT NULL REFERENCES plugin_football_teams(id) ON DELETE CASCADE"`
    pub columns: HashMap<String, String>,
}

/// A runtime state variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStateVar {
    #[serde(rename = "type")]
    pub var_type: String,
    pub default: serde_json::Value,
}

/// Path to a web component JS module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginComponent {
    pub component: String,
}

/// A screen (persistent overlay) defined by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginScreenDef {
    pub template_id: String,
    pub name: String,
    /// Relative path to the HTML template file within the plugin folder.
    pub template: String,
    #[serde(default = "default_true")]
    pub allow_popups: bool,
}

/// A popup (temporary overlay) defined by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPopupDef {
    pub template_id: String,
    pub name: String,
    /// Relative path to the HTML template file within the plugin folder.
    pub template: String,
    #[serde(default = "default_direction")]
    pub direction: String,
    #[serde(default = "default_position")]
    pub position: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    #[serde(default = "default_duration")]
    pub duration: i64,
}

fn default_true() -> bool {
    true
}
fn default_direction() -> String {
    "bottom".to_string()
}
fn default_position() -> i64 {
    50
}
fn default_duration() -> i64 {
    10
}

/// Validate that a plugin ID only contains safe characters.
pub fn validate_plugin_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Validate that a table/column name only contains safe characters.
pub fn validate_identifier(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}
