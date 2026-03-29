use anyhow::Result;
use rusqlite::Connection;
use crate::models::{ObsCommand, Preset, Studio};

pub struct CommandInput {
    pub name: String,
    pub color: String,
    pub shortcut: String,
    pub description: String,
}

pub struct PresetInput {
    pub name: String,
    pub commands: Vec<CommandInput>,
}

pub fn get_all_studios(conn: &Connection) -> Result<Vec<Studio>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, obs_browser_source_address, created_at FROM studios ORDER BY id"
    )?;
    let studio_rows: Vec<(i64, String, String, String)> = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?.collect::<rusqlite::Result<Vec<_>>>()?;

    let mut studios = Vec::new();
    for (id, name, obs_addr, created_at) in studio_rows {
        let presets = load_presets_for_studio(conn, id)?;
        let commands: Vec<ObsCommand> = presets.iter()
            .flat_map(|p| p.commands.iter().cloned())
            .collect();
        studios.push(Studio {
            id,
            name,
            obs_browser_source_address: obs_addr,
            presets,
            commands,
            created_at,
        });
    }
    Ok(studios)
}

pub fn get_studio(conn: &Connection, id: i64) -> Result<Option<Studio>> {
    let result = conn.query_row(
        "SELECT id, name, obs_browser_source_address, created_at FROM studios WHERE id = ?1",
        [id],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)),
    );
    match result {
        Ok((sid, name, obs_addr, created_at)) => {
            let presets = load_presets_for_studio(conn, sid)?;
            let commands: Vec<ObsCommand> = presets.iter()
                .flat_map(|p| p.commands.iter().cloned())
                .collect();
            Ok(Some(Studio {
                id: sid,
                name,
                obs_browser_source_address: obs_addr,
                presets,
                commands,
                created_at,
            }))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn create_studio(conn: &Connection, name: &str) -> Result<Studio> {
    conn.execute(
        "INSERT INTO studios (name, obs_browser_source_address) VALUES (?1, '')",
        [name],
    )?;
    let id = conn.last_insert_rowid();
    // Update obs_browser_source_address with the actual id
    let obs_addr = format!("/obs-overlay?studio={id}");
    conn.execute(
        "UPDATE studios SET obs_browser_source_address = ?1 WHERE id = ?2",
        rusqlite::params![obs_addr, id],
    )?;
    let studio = get_studio(conn, id)?.expect("studio just inserted must exist");
    Ok(studio)
}

pub fn update_studio(
    conn: &Connection,
    id: i64,
    name: &str,
    obs_browser_source_address: &str,
    presets: &[PresetInput],
) -> Result<Option<Studio>> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM studios WHERE id = ?1",
        [id],
        |row| row.get::<_, i64>(0),
    )? > 0;

    if !exists {
        return Ok(None);
    }

    // Update core studio fields
    conn.execute(
        "UPDATE studios SET name = ?1, obs_browser_source_address = ?2 WHERE id = ?3",
        rusqlite::params![name, obs_browser_source_address, id],
    )?;

    // Delete all existing presets (cascades to commands)
    conn.execute("DELETE FROM presets WHERE studio_id = ?1", [id])?;

    // Re-insert presets and commands
    for preset_input in presets {
        conn.execute(
            "INSERT INTO presets (studio_id, name) VALUES (?1, ?2)",
            rusqlite::params![id, preset_input.name],
        )?;
        let preset_id = conn.last_insert_rowid();
        for cmd in &preset_input.commands {
            conn.execute(
                "INSERT INTO obs_commands (preset_id, name, color, shortcut, description) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![preset_id, cmd.name, cmd.color, cmd.shortcut, cmd.description],
            )?;
        }
    }

    Ok(get_studio(conn, id)?)
}

pub fn delete_studio(conn: &Connection, id: i64) -> Result<bool> {
    let rows = conn.execute("DELETE FROM studios WHERE id = ?1", [id])?;
    Ok(rows > 0)
}

fn load_presets_for_studio(conn: &Connection, studio_id: i64) -> Result<Vec<Preset>> {
    let mut stmt = conn.prepare(
        "SELECT id, name FROM presets WHERE studio_id = ?1 ORDER BY id"
    )?;
    let preset_rows: Vec<(i64, String)> = stmt.query_map([studio_id], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?.collect::<rusqlite::Result<Vec<_>>>()?;

    let mut presets = Vec::new();
    for (preset_id, preset_name) in preset_rows {
        let commands = load_commands_for_preset(conn, preset_id, studio_id)?;
        presets.push(Preset {
            id: Some(preset_id),
            studio_id: Some(studio_id),
            name: preset_name,
            commands,
        });
    }
    Ok(presets)
}

fn load_commands_for_preset(conn: &Connection, preset_id: i64, studio_id: i64) -> Result<Vec<ObsCommand>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, shortcut, description FROM obs_commands WHERE preset_id = ?1 ORDER BY id"
    )?;
    let commands: Vec<ObsCommand> = stmt.query_map([preset_id], |row| {
        Ok(ObsCommand {
            id: Some(row.get(0)?),
            preset_id: Some(preset_id),
            studio_id: Some(studio_id),
            name: row.get(1)?,
            color: row.get(2)?,
            shortcut: row.get(3)?,
            description: row.get(4)?,
        })
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(commands)
}
