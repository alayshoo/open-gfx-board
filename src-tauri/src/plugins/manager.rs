use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{bail, Context, Result};
use rusqlite::Connection;

use super::manifest::{validate_identifier, validate_plugin_id, PluginManifest};
use super::state;

// ── Startup ──────────────────────────────────────────────────────────────────

/// Called once at startup.  Scans the plugins directory, ensures DB tables
/// exist, initialises default state, and syncs plugin screens/popups into
/// the core tables.  Returns a map of `plugin_id -> PluginManifest` for
/// **enabled** plugins.
///
/// `bundled_ids` is the list of plugin IDs that originate from the app's
/// bundled plugin directory; they are marked `is_bundled = 1` in the DB.
pub fn startup_sync(
    app_data_dir: &Path,
    bundled_ids: &[String],
    conn: &Connection,
) -> Result<HashMap<String, PluginManifest>> {
    let plugins_dir = app_data_dir.join("plugins");
    if !plugins_dir.exists() {
        std::fs::create_dir_all(&plugins_dir)?;
    }

    // Scan filesystem for plugin folders
    let fs_manifests = scan_plugins(&plugins_dir)?;

    // Reconcile with database
    for manifest in &fs_manifests {
        let already_installed = conn
            .query_row(
                "SELECT COUNT(*) FROM plugins WHERE id = ?1",
                [&manifest.id],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        let is_bundled = bundled_ids.contains(&manifest.id);

        if already_installed {
            // Update manifest in DB in case files changed
            let manifest_json = serde_json::to_string(manifest)?;
            conn.execute(
                "UPDATE plugins SET name = ?1, version = ?2, description = ?3, author = ?4, manifest_json = ?5, is_bundled = ?6 WHERE id = ?7",
                rusqlite::params![
                    manifest.name,
                    manifest.version,
                    manifest.description,
                    manifest.author,
                    manifest_json,
                    is_bundled as i64,
                    manifest.id,
                ],
            )?;
        } else {
            // First time seeing this plugin on disk – install it
            let manifest_json = serde_json::to_string(manifest)?;
            conn.execute(
                "INSERT INTO plugins (id, name, version, description, author, enabled, manifest_json, is_bundled) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7)",
                rusqlite::params![
                    manifest.id,
                    manifest.name,
                    manifest.version,
                    manifest.description,
                    manifest.author,
                    manifest_json,
                    is_bundled as i64,
                ],
            )?;
        }

        // Ensure dynamic tables exist
        create_plugin_tables(conn, manifest)?;

        // Initialise default state values (INSERT OR IGNORE)
        state::initialize_defaults(conn, manifest)?;

        // Sync screens and popups into core tables
        sync_plugin_screens(conn, manifest, app_data_dir)?;
        sync_plugin_popups(conn, manifest, app_data_dir)?;
    }

    // Build the result: only enabled plugins
    let mut result = HashMap::new();
    for manifest in fs_manifests {
        let enabled: bool = conn
            .query_row(
                "SELECT enabled FROM plugins WHERE id = ?1",
                [&manifest.id],
                |r| r.get::<_, bool>(0),
            )
            .unwrap_or(false);

        if enabled {
            result.insert(manifest.id.clone(), manifest);
        }
    }

    Ok(result)
}

// ── Scanning ─────────────────────────────────────────────────────────────────

/// Scan the plugins directory for valid plugin.json files.
pub fn scan_plugins(plugins_dir: &Path) -> Result<Vec<PluginManifest>> {
    let mut manifests = Vec::new();

    if !plugins_dir.exists() {
        return Ok(manifests);
    }

    for entry in std::fs::read_dir(plugins_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("plugin.json");
        if !manifest_path.exists() {
            continue;
        }

        match load_manifest(&manifest_path) {
            Ok(manifest) => {
                if !validate_plugin_id(&manifest.id) {
                    eprintln!(
                        "Plugin at {:?} has invalid id '{}', skipping",
                        path, manifest.id
                    );
                    continue;
                }
                manifests.push(manifest);
            }
            Err(e) => {
                eprintln!("Failed to load plugin manifest at {:?}: {}", manifest_path, e);
            }
        }
    }

    Ok(manifests)
}

/// Load and parse a plugin manifest from disk.
pub fn load_manifest(path: &Path) -> Result<PluginManifest> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading {:?}", path))?;
    let manifest: PluginManifest = serde_json::from_str(&content)
        .with_context(|| format!("parsing {:?}", path))?;
    Ok(manifest)
}

// ── Installation ─────────────────────────────────────────────────────────────

/// Install a plugin from a source directory into the plugins folder.
pub fn install_plugin(
    app_data_dir: &Path,
    source_dir: &Path,
    conn: &Connection,
) -> Result<PluginManifest> {
    let manifest_path = source_dir.join("plugin.json");
    if !manifest_path.exists() {
        bail!("No plugin.json found in {:?}", source_dir);
    }

    let manifest = load_manifest(&manifest_path)?;

    if !validate_plugin_id(&manifest.id) {
        bail!("Invalid plugin id: '{}'", manifest.id);
    }

    // Check for conflicts
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM plugins WHERE id = ?1",
            [&manifest.id],
            |r| r.get::<_, i64>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if exists {
        bail!(
            "Plugin '{}' is already installed. Uninstall it first.",
            manifest.id
        );
    }

    // Copy plugin files to app_data_dir/plugins/{id}/
    let target_dir = app_data_dir.join("plugins").join(&manifest.id);
    if target_dir.exists() {
        std::fs::remove_dir_all(&target_dir)?;
    }
    copy_dir_recursive(source_dir, &target_dir)?;

    // Insert into DB
    let manifest_json = serde_json::to_string(&manifest)?;
    conn.execute(
        "INSERT INTO plugins (id, name, version, description, author, enabled, manifest_json) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
        rusqlite::params![
            manifest.id,
            manifest.name,
            manifest.version,
            manifest.description,
            manifest.author,
            manifest_json,
        ],
    )?;

    create_plugin_tables(conn, &manifest)?;
    state::initialize_defaults(conn, &manifest)?;
    sync_plugin_screens(conn, &manifest, app_data_dir)?;
    sync_plugin_popups(conn, &manifest, app_data_dir)?;

    Ok(manifest)
}

/// Uninstall a plugin: drop tables, remove files, delete DB rows.
pub fn uninstall_plugin(
    app_data_dir: &Path,
    plugin_id: &str,
    conn: &Connection,
) -> Result<()> {
    if !validate_plugin_id(plugin_id) {
        bail!("Invalid plugin id: '{}'", plugin_id);
    }

    // Load manifest from DB to know which tables to drop
    let manifest_json: Option<String> = conn
        .query_row(
            "SELECT manifest_json FROM plugins WHERE id = ?1",
            [plugin_id],
            |r| r.get(0),
        )
        .ok();

    if let Some(json) = manifest_json {
        if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&json) {
            drop_plugin_tables(conn, &manifest)?;
        }
    }

    // Delete from core tables (CASCADE handles plugin_state)
    conn.execute(
        "DELETE FROM screens WHERE plugin_id = ?1",
        [plugin_id],
    )?;
    conn.execute(
        "DELETE FROM popups WHERE plugin_id = ?1",
        [plugin_id],
    )?;
    conn.execute("DELETE FROM plugins WHERE id = ?1", [plugin_id])?;

    // Remove files
    let plugin_dir = app_data_dir.join("plugins").join(plugin_id);
    if plugin_dir.exists() {
        std::fs::remove_dir_all(&plugin_dir)?;
    }

    Ok(())
}

// ── Enable / Disable ─────────────────────────────────────────────────────────

pub fn enable_plugin(conn: &Connection, plugin_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE plugins SET enabled = 1 WHERE id = ?1",
        [plugin_id],
    )?;
    Ok(())
}

pub fn disable_plugin(conn: &Connection, plugin_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE plugins SET enabled = 0 WHERE id = ?1",
        [plugin_id],
    )?;
    Ok(())
}

// ── Dynamic Table Management ─────────────────────────────────────────────────

/// Create all dynamic tables declared in a plugin manifest.
/// Uses CREATE TABLE IF NOT EXISTS, safe to call multiple times.
pub fn create_plugin_tables(conn: &Connection, manifest: &PluginManifest) -> Result<()> {
    for (table_name, table_def) in &manifest.database {
        if !validate_identifier(table_name) {
            eprintln!(
                "Plugin '{}' has invalid table name '{}', skipping",
                manifest.id, table_name
            );
            continue;
        }

        let full_table_name = format!("plugin_{}_{}", manifest.id.replace('-', "_"), table_name);

        let mut col_defs = vec!["id INTEGER PRIMARY KEY AUTOINCREMENT".to_string()];
        for (col_name, col_type) in &table_def.columns {
            if !validate_identifier(col_name) {
                eprintln!(
                    "Plugin '{}' table '{}' has invalid column '{}', skipping",
                    manifest.id, table_name, col_name
                );
                continue;
            }
            col_defs.push(format!("\"{}\" {}", col_name, col_type));
        }
        col_defs.push("created_at TEXT NOT NULL DEFAULT (datetime('now'))".to_string());

        let sql = format!(
            "CREATE TABLE IF NOT EXISTS \"{}\" ({})",
            full_table_name,
            col_defs.join(", ")
        );
        conn.execute_batch(&sql)?;
    }
    Ok(())
}

/// Drop all dynamic tables for a plugin.
fn drop_plugin_tables(conn: &Connection, manifest: &PluginManifest) -> Result<()> {
    for table_name in manifest.database.keys() {
        if !validate_identifier(table_name) {
            continue;
        }
        let full_table_name = format!("plugin_{}_{}", manifest.id.replace('-', "_"), table_name);
        conn.execute_batch(&format!("DROP TABLE IF EXISTS \"{}\"", full_table_name))?;
    }
    Ok(())
}

// ── Screen / Popup Sync ──────────────────────────────────────────────────────

/// Sync plugin-defined screens into the core `screens` table.
fn sync_plugin_screens(
    conn: &Connection,
    manifest: &PluginManifest,
    app_data_dir: &Path,
) -> Result<()> {
    for screen_def in &manifest.screens {
        // Read template HTML from disk
        let template_path = app_data_dir
            .join("plugins")
            .join(&manifest.id)
            .join(&screen_def.template);
        let html_content = std::fs::read_to_string(&template_path).unwrap_or_default();

        // Check if already exists
        let existing_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM screens WHERE plugin_id = ?1 AND plugin_template_id = ?2",
                rusqlite::params![manifest.id, screen_def.template_id],
                |r| r.get(0),
            )
            .ok();

        if let Some(id) = existing_id {
            // Update
            conn.execute(
                "UPDATE screens SET name = ?1, media_type = 'html', html_content = ?2, allow_popups = ?3 WHERE id = ?4",
                rusqlite::params![
                    screen_def.name,
                    html_content,
                    screen_def.allow_popups,
                    id,
                ],
            )?;
        } else {
            // Insert
            conn.execute(
                "INSERT INTO screens (name, comments, media_type, allow_popups, html_content, plugin_id, plugin_template_id)
                 VALUES (?1, '', 'html', ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    screen_def.name,
                    screen_def.allow_popups,
                    html_content,
                    manifest.id,
                    screen_def.template_id,
                ],
            )?;
        }
    }

    // Remove screens for template_ids that no longer exist in the manifest
    let template_ids: Vec<String> = manifest.screens.iter().map(|s| s.template_id.clone()).collect();
    if template_ids.is_empty() {
        conn.execute(
            "DELETE FROM screens WHERE plugin_id = ?1",
            [&manifest.id],
        )?;
    } else {
        let placeholders: Vec<String> = template_ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 2)).collect();
        let sql = format!(
            "DELETE FROM screens WHERE plugin_id = ?1 AND plugin_template_id NOT IN ({})",
            placeholders.join(", ")
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(manifest.id.clone())];
        for tid in &template_ids {
            params.push(Box::new(tid.clone()));
        }
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        stmt.execute(param_refs.as_slice())?;
    }

    Ok(())
}

/// Sync plugin-defined popups into the core `popups` table.
fn sync_plugin_popups(
    conn: &Connection,
    manifest: &PluginManifest,
    app_data_dir: &Path,
) -> Result<()> {
    for popup_def in &manifest.popups {
        let template_path = app_data_dir
            .join("plugins")
            .join(&manifest.id)
            .join(&popup_def.template);
        let html_content = std::fs::read_to_string(&template_path).unwrap_or_default();

        let existing_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM popups WHERE plugin_id = ?1 AND plugin_template_id = ?2",
                rusqlite::params![manifest.id, popup_def.template_id],
                |r| r.get(0),
            )
            .ok();

        if let Some(id) = existing_id {
            conn.execute(
                "UPDATE popups SET name = ?1, media_type = 'html', html_content = ?2, direction = ?3, position = ?4, width = ?5, height = ?6 WHERE id = ?7",
                rusqlite::params![
                    popup_def.name,
                    html_content,
                    popup_def.direction,
                    popup_def.position,
                    popup_def.width,
                    popup_def.height,
                    id,
                ],
            )?;
        } else {
            conn.execute(
                "INSERT INTO popups (name, sponsor_name, comments, media_type, html_content, direction, position, width, height, plugin_id, plugin_template_id)
                 VALUES (?1, '', '', 'html', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    popup_def.name,
                    html_content,
                    popup_def.direction,
                    popup_def.position,
                    popup_def.width,
                    popup_def.height,
                    manifest.id,
                    popup_def.template_id,
                ],
            )?;
        }
    }

    // Remove popups for template_ids that no longer exist
    let template_ids: Vec<String> = manifest.popups.iter().map(|p| p.template_id.clone()).collect();
    if template_ids.is_empty() {
        conn.execute(
            "DELETE FROM popups WHERE plugin_id = ?1",
            [&manifest.id],
        )?;
    } else {
        let placeholders: Vec<String> = template_ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 2)).collect();
        let sql = format!(
            "DELETE FROM popups WHERE plugin_id = ?1 AND plugin_template_id NOT IN ({})",
            placeholders.join(", ")
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(manifest.id.clone())];
        for tid in &template_ids {
            params.push(Box::new(tid.clone()));
        }
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        stmt.execute(param_refs.as_slice())?;
    }

    Ok(())
}

// ── Refresh (bundled plugins only) ───────────────────────────────────────────

/// Re-copy a bundled plugin's files from `bundled_dir/{id}` to the installed
/// location, then refresh the manifest, screens, and popups in the DB.
/// User data (plugin_state, dynamic table rows) is never touched.
pub fn refresh_plugin(
    app_data_dir: &Path,
    bundled_dir: &Path,
    plugin_id: &str,
    conn: &Connection,
) -> Result<PluginManifest> {
    if !validate_plugin_id(plugin_id) {
        bail!("Invalid plugin id: '{}'", plugin_id);
    }

    // Only bundled plugins can be refreshed via this path.
    let is_bundled: bool = conn
        .query_row(
            "SELECT is_bundled FROM plugins WHERE id = ?1",
            [plugin_id],
            |r| r.get::<_, bool>(0),
        )
        .unwrap_or(false);
    if !is_bundled {
        bail!("Plugin '{}' is not a bundled plugin.", plugin_id);
    }

    let source_dir = bundled_dir.join(plugin_id);
    if !source_dir.exists() || !source_dir.join("plugin.json").exists() {
        bail!(
            "Bundled source for plugin '{}' not found at {:?}",
            plugin_id, source_dir
        );
    }

    // Re-copy all asset/template files (DB data lives separately and is preserved).
    let target_dir = app_data_dir.join("plugins").join(plugin_id);
    copy_dir_recursive(&source_dir, &target_dir)?;

    // Reload manifest from the freshly copied plugin.json.
    let manifest = load_manifest(&target_dir.join("plugin.json"))?;
    let manifest_json = serde_json::to_string(&manifest)?;

    conn.execute(
        "UPDATE plugins SET name = ?1, version = ?2, description = ?3, author = ?4, manifest_json = ?5 WHERE id = ?6",
        rusqlite::params![
            manifest.name,
            manifest.version,
            manifest.description,
            manifest.author,
            manifest_json,
            plugin_id,
        ],
    )?;

    // Ensure any new tables from an updated manifest are created (existing ones are untouched).
    create_plugin_tables(conn, &manifest)?;

    // Insert default values for any newly added state keys (existing keys are preserved).
    state::initialize_defaults(conn, &manifest)?;

    // Re-sync screens and popups so template HTML stays current.
    sync_plugin_screens(conn, &manifest, app_data_dir)?;
    sync_plugin_popups(conn, &manifest, app_data_dir)?;

    Ok(manifest)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Copy a directory recursively (public for bundled plugin installation).
pub fn copy_dir_public(src: &Path, dst: &Path) -> Result<()> {
    copy_dir_recursive(src, dst)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Get a plugin's directory path.
pub fn plugin_dir(app_data_dir: &Path, plugin_id: &str) -> PathBuf {
    app_data_dir.join("plugins").join(plugin_id)
}
