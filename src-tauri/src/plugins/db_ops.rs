use std::collections::HashMap;
use anyhow::{bail, Result};
use rusqlite::Connection;
use serde_json::{Map, Value};

use super::manifest::{validate_identifier, PluginManifest};

/// Build the full SQLite table name from plugin id + logical table name.
fn full_table_name(plugin_id: &str, table_name: &str) -> String {
    format!("plugin_{}_{}", plugin_id.replace('-', "_"), table_name)
}

/// Validate that a table exists in the manifest and return its column names.
fn validated_columns(
    manifest: &PluginManifest,
    table_name: &str,
) -> Result<Vec<String>> {
    let table_def = manifest
        .database
        .get(table_name)
        .ok_or_else(|| anyhow::anyhow!("Table '{}' not declared in plugin manifest", table_name))?;
    Ok(table_def.columns.keys().cloned().collect())
}

/// List all rows from a plugin table.
pub fn list_rows(
    conn: &Connection,
    manifest: &PluginManifest,
    table_name: &str,
) -> Result<Vec<Value>> {
    if !validate_identifier(table_name) {
        bail!("Invalid table name: '{}'", table_name);
    }
    let _ = validated_columns(manifest, table_name)?;
    let full_name = full_table_name(&manifest.id, table_name);

    let mut stmt = conn.prepare(&format!("SELECT * FROM \"{}\" ORDER BY id", full_name))?;
    let col_count = stmt.column_count();
    let col_names: Vec<String> = (0..col_count)
        .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
        .collect();

    let rows = stmt.query_map([], |row| {
        let mut obj = Map::new();
        for (i, name) in col_names.iter().enumerate() {
            let val = row_value_at(row, i);
            obj.insert(name.clone(), val);
        }
        Ok(Value::Object(obj))
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Get a single row by id.
pub fn get_row(
    conn: &Connection,
    manifest: &PluginManifest,
    table_name: &str,
    row_id: i64,
) -> Result<Option<Value>> {
    if !validate_identifier(table_name) {
        bail!("Invalid table name: '{}'", table_name);
    }
    let _ = validated_columns(manifest, table_name)?;
    let full_name = full_table_name(&manifest.id, table_name);

    let mut stmt = conn.prepare(&format!("SELECT * FROM \"{}\" WHERE id = ?1", full_name))?;
    let col_count = stmt.column_count();
    let col_names: Vec<String> = (0..col_count)
        .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
        .collect();

    let result = stmt.query_row([row_id], |row| {
        let mut obj = Map::new();
        for (i, name) in col_names.iter().enumerate() {
            let val = row_value_at(row, i);
            obj.insert(name.clone(), val);
        }
        Ok(Value::Object(obj))
    });

    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Insert a new row.  `data` is a JSON object with column values.
/// Returns the inserted row.
pub fn insert_row(
    conn: &Connection,
    manifest: &PluginManifest,
    table_name: &str,
    data: &Value,
) -> Result<Value> {
    if !validate_identifier(table_name) {
        bail!("Invalid table name: '{}'", table_name);
    }
    let allowed_cols = validated_columns(manifest, table_name)?;
    let full_name = full_table_name(&manifest.id, table_name);

    let obj = data
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Insert data must be a JSON object"))?;

    let mut col_names = Vec::new();
    let mut placeholders = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    for (key, val) in obj {
        if !validate_identifier(key) || !allowed_cols.contains(key) {
            continue; // skip unknown columns silently
        }
        col_names.push(format!("\"{}\"", key));
        placeholders.push(format!("?{}", values.len() + 1));
        values.push(json_to_sql(val));
    }

    if col_names.is_empty() {
        bail!("No valid columns provided for insert");
    }

    let sql = format!(
        "INSERT INTO \"{}\" ({}) VALUES ({})",
        full_name,
        col_names.join(", "),
        placeholders.join(", ")
    );

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&sql, param_refs.as_slice())?;

    let row_id = conn.last_insert_rowid();
    get_row(conn, manifest, table_name, row_id)?
        .ok_or_else(|| anyhow::anyhow!("Failed to read back inserted row"))
}

/// Update a row by id.  `data` is a JSON object with column values to update.
/// Returns the updated row.
pub fn update_row(
    conn: &Connection,
    manifest: &PluginManifest,
    table_name: &str,
    row_id: i64,
    data: &Value,
) -> Result<Option<Value>> {
    if !validate_identifier(table_name) {
        bail!("Invalid table name: '{}'", table_name);
    }
    let allowed_cols = validated_columns(manifest, table_name)?;
    let full_name = full_table_name(&manifest.id, table_name);

    let obj = data
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Update data must be a JSON object"))?;

    let mut set_clauses = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    for (key, val) in obj {
        if !validate_identifier(key) || !allowed_cols.contains(key) {
            continue;
        }
        set_clauses.push(format!("\"{}\" = ?{}", key, values.len() + 1));
        values.push(json_to_sql(val));
    }

    if set_clauses.is_empty() {
        bail!("No valid columns provided for update");
    }

    // The WHERE id placeholder
    let id_param_idx = values.len() + 1;
    values.push(Box::new(row_id));

    let sql = format!(
        "UPDATE \"{}\" SET {} WHERE id = ?{}",
        full_name,
        set_clauses.join(", "),
        id_param_idx
    );

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    let affected = conn.execute(&sql, param_refs.as_slice())?;

    if affected == 0 {
        return Ok(None);
    }
    get_row(conn, manifest, table_name, row_id)
}

/// Delete a row by id.  Returns true if a row was deleted.
pub fn delete_row(
    conn: &Connection,
    manifest: &PluginManifest,
    table_name: &str,
    row_id: i64,
) -> Result<bool> {
    if !validate_identifier(table_name) {
        bail!("Invalid table name: '{}'", table_name);
    }
    let _ = validated_columns(manifest, table_name)?;
    let full_name = full_table_name(&manifest.id, table_name);

    let affected = conn.execute(
        &format!("DELETE FROM \"{}\" WHERE id = ?1", full_name),
        [row_id],
    )?;
    Ok(affected > 0)
}

/// Query rows with simple equality filters.
/// `filters` is a map of column_name -> value.
pub fn query_rows(
    conn: &Connection,
    manifest: &PluginManifest,
    table_name: &str,
    filters: &HashMap<String, String>,
) -> Result<Vec<Value>> {
    if !validate_identifier(table_name) {
        bail!("Invalid table name: '{}'", table_name);
    }
    let allowed_cols = validated_columns(manifest, table_name)?;
    let full_name = full_table_name(&manifest.id, table_name);

    let mut where_clauses = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    for (key, val) in filters {
        if !validate_identifier(key) || !allowed_cols.contains(key) {
            continue;
        }
        where_clauses.push(format!("\"{}\" = ?{}", key, values.len() + 1));
        values.push(Box::new(val.clone()));
    }

    let sql = if where_clauses.is_empty() {
        format!("SELECT * FROM \"{}\" ORDER BY id", full_name)
    } else {
        format!(
            "SELECT * FROM \"{}\" WHERE {} ORDER BY id",
            full_name,
            where_clauses.join(" AND ")
        )
    };

    let mut stmt = conn.prepare(&sql)?;
    let col_count = stmt.column_count();
    let col_names: Vec<String> = (0..col_count)
        .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
        .collect();

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        let mut obj = Map::new();
        for (i, name) in col_names.iter().enumerate() {
            let val = row_value_at(row, i);
            obj.insert(name.clone(), val);
        }
        Ok(Value::Object(obj))
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Extract a value from a rusqlite row at the given index.
fn row_value_at(row: &rusqlite::Row, idx: usize) -> Value {
    // Try integer first, then float, then string, then null
    if let Ok(v) = row.get::<_, i64>(idx) {
        return Value::Number(v.into());
    }
    if let Ok(v) = row.get::<_, f64>(idx) {
        return serde_json::Number::from_f64(v)
            .map(Value::Number)
            .unwrap_or(Value::Null);
    }
    if let Ok(v) = row.get::<_, String>(idx) {
        return Value::String(v);
    }
    Value::Null
}

/// Convert a JSON value to a boxed ToSql for rusqlite.
fn json_to_sql(val: &Value) -> Box<dyn rusqlite::types::ToSql> {
    match val {
        Value::Null => Box::new(Option::<String>::None),
        Value::Bool(b) => Box::new(*b as i64),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Box::new(i)
            } else if let Some(f) = n.as_f64() {
                Box::new(f)
            } else {
                Box::new(n.to_string())
            }
        }
        Value::String(s) => Box::new(s.clone()),
        _ => Box::new(serde_json::to_string(val).unwrap_or_default()),
    }
}
