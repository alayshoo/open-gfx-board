use std::collections::HashMap;
use anyhow::Result;
use rusqlite::Connection;
use serde_json::Value;

use super::manifest::PluginManifest;

/// Load all plugin states from the database into an in-memory map.
/// Returns `plugin_id -> (key -> JSON value)`.
pub fn load_all_states(
    conn: &Connection,
    manifests: &HashMap<String, PluginManifest>,
) -> Result<HashMap<String, HashMap<String, Value>>> {
    let mut all_states: HashMap<String, HashMap<String, Value>> = HashMap::new();

    for (plugin_id, manifest) in manifests {
        let state = load_plugin_state(conn, plugin_id)?;

        // Merge with defaults: manifest defaults for keys not yet in DB
        let mut merged = HashMap::new();
        for (key, var_def) in &manifest.state {
            merged.insert(
                key.clone(),
                state
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| var_def.default.clone()),
            );
        }
        // Also keep any DB keys that are no longer in manifest (data preservation)
        for (key, val) in &state {
            merged.entry(key.clone()).or_insert_with(|| val.clone());
        }

        all_states.insert(plugin_id.clone(), merged);
    }

    Ok(all_states)
}

/// Load all state key-value pairs for a single plugin from the database.
pub fn load_plugin_state(
    conn: &Connection,
    plugin_id: &str,
) -> Result<HashMap<String, Value>> {
    let mut stmt =
        conn.prepare("SELECT key, value FROM plugin_state WHERE plugin_id = ?1")?;
    let rows = stmt.query_map([plugin_id], |row| {
        let key: String = row.get(0)?;
        let value_str: Option<String> = row.get(1)?;
        Ok((key, value_str))
    })?;

    let mut state = HashMap::new();
    for row in rows {
        let (key, value_str) = row?;
        let value: Value = match value_str {
            Some(s) => serde_json::from_str(&s).unwrap_or(Value::String(s)),
            None => Value::Null,
        };
        state.insert(key, value);
    }
    Ok(state)
}

/// Initialize default state values for a plugin (inserts only missing keys).
pub fn initialize_defaults(
    conn: &Connection,
    manifest: &PluginManifest,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "INSERT OR IGNORE INTO plugin_state (plugin_id, key, value) VALUES (?1, ?2, ?3)",
    )?;
    for (key, var_def) in &manifest.state {
        let value_str = serde_json::to_string(&var_def.default)?;
        stmt.execute(rusqlite::params![manifest.id, key, value_str])?;
    }
    Ok(())
}

/// Set a single state key for a plugin (upsert).
pub fn set_state_key(
    conn: &Connection,
    plugin_id: &str,
    key: &str,
    value: &Value,
) -> Result<()> {
    let value_str = serde_json::to_string(value)?;
    conn.execute(
        "INSERT INTO plugin_state (plugin_id, key, value) VALUES (?1, ?2, ?3)
         ON CONFLICT(plugin_id, key) DO UPDATE SET value = excluded.value",
        rusqlite::params![plugin_id, key, value_str],
    )?;
    Ok(())
}

/// Batch-update multiple state keys for a plugin (upsert each).
pub fn set_state_batch(
    conn: &Connection,
    plugin_id: &str,
    updates: &HashMap<String, Value>,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "INSERT INTO plugin_state (plugin_id, key, value) VALUES (?1, ?2, ?3)
         ON CONFLICT(plugin_id, key) DO UPDATE SET value = excluded.value",
    )?;
    for (key, value) in updates {
        let value_str = serde_json::to_string(value)?;
        stmt.execute(rusqlite::params![plugin_id, key, value_str])?;
    }
    Ok(())
}

/// Delete all state for a plugin.
pub fn delete_all_state(conn: &Connection, plugin_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM plugin_state WHERE plugin_id = ?1",
        [plugin_id],
    )?;
    Ok(())
}
