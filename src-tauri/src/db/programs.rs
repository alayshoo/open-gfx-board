use anyhow::Result;
use rusqlite::Connection;
use crate::models::{Program, ProgramPopup, Screen};
use crate::db::popups::get_popup;

pub struct ProgramPopupInput {
    pub popup_id: i64,
    pub trigger_type: String,
    pub duration: i64,
    pub frequency: i64,
}

pub fn get_all_programs(conn: &Connection) -> Result<Vec<Program>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, logo_path, bg_path, created_at FROM programs ORDER BY id"
    )?;
    let rows: Vec<(i64, String, Option<String>, Option<String>, String)> = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
    })?.collect::<rusqlite::Result<Vec<_>>>()?;

    let mut programs = Vec::new();
    for (id, name, logo_path, bg_path, created_at) in rows {
        let screens = load_screens_for_program(conn, id)?;
        let program_popups = load_program_popups(conn, id)?;
        programs.push(Program {
            id,
            name,
            logo_path,
            bg_path,
            screens,
            program_popups,
            created_at,
        });
    }
    Ok(programs)
}

pub fn get_program(conn: &Connection, id: i64) -> Result<Option<Program>> {
    let result = conn.query_row(
        "SELECT id, name, logo_path, bg_path, created_at FROM programs WHERE id = ?1",
        [id],
        |row| Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, String>(4)?,
        )),
    );
    match result {
        Ok((pid, name, logo_path, bg_path, created_at)) => {
            let screens = load_screens_for_program(conn, pid)?;
            let program_popups = load_program_popups(conn, pid)?;
            Ok(Some(Program {
                id: pid,
                name,
                logo_path,
                bg_path,
                screens,
                program_popups,
                created_at,
            }))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn create_program(conn: &Connection, name: &str) -> Result<Program> {
    conn.execute(
        "INSERT INTO programs (name) VALUES (?1)",
        [name],
    )?;
    let id = conn.last_insert_rowid();
    Ok(get_program(conn, id)?.expect("program just inserted must exist"))
}

pub fn update_program(
    conn: &Connection,
    id: i64,
    name: &str,
    logo_path: Option<&str>,
    bg_path: Option<&str>,
    screen_ids: &[i64],
    popups: &[ProgramPopupInput],
) -> Result<Option<Program>> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM programs WHERE id = ?1",
        [id],
        |row| row.get::<_, i64>(0),
    )? > 0;

    if !exists {
        return Ok(None);
    }

    conn.execute(
        "UPDATE programs SET name = ?1, logo_path = ?2, bg_path = ?3 WHERE id = ?4",
        rusqlite::params![name, logo_path, bg_path, id],
    )?;

    // Replace screen associations
    conn.execute("DELETE FROM program_screens WHERE program_id = ?1", [id])?;
    for &screen_id in screen_ids {
        conn.execute(
            "INSERT OR IGNORE INTO program_screens (program_id, screen_id) VALUES (?1, ?2)",
            rusqlite::params![id, screen_id],
        )?;
    }

    // Replace program popups
    conn.execute("DELETE FROM program_popups WHERE program_id = ?1", [id])?;
    for popup in popups {
        conn.execute(
            "INSERT INTO program_popups (program_id, popup_id, trigger_type, duration, frequency) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, popup.popup_id, popup.trigger_type, popup.duration, popup.frequency],
        )?;
    }

    Ok(get_program(conn, id)?)
}

pub fn delete_program(conn: &Connection, id: i64) -> Result<bool> {
    let rows = conn.execute("DELETE FROM programs WHERE id = ?1", [id])?;
    Ok(rows > 0)
}

fn load_screens_for_program(conn: &Connection, program_id: i64) -> Result<Vec<Screen>> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.name, s.comments, s.media_path, s.media_type, s.allow_popups, s.created_at
         FROM screens s
         JOIN program_screens ps ON ps.screen_id = s.id
         WHERE ps.program_id = ?1
         ORDER BY ps.id"
    )?;
    let screens: Vec<Screen> = stmt.query_map([program_id], |row| {
        let allow_popups_int: i64 = row.get(5)?;
        Ok(Screen {
            id: row.get(0)?,
            name: row.get(1)?,
            comments: row.get(2)?,
            media_path: row.get(3)?,
            media_type: row.get(4)?,
            allow_popups: allow_popups_int != 0,
            created_at: row.get(6)?,
        })
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(screens)
}

fn load_program_popups(conn: &Connection, program_id: i64) -> Result<Vec<ProgramPopup>> {
    let mut stmt = conn.prepare(
        "SELECT id, popup_id, trigger_type, duration, frequency FROM program_popups WHERE program_id = ?1 ORDER BY id"
    )?;
    let rows: Vec<(i64, i64, String, i64, i64)> = stmt.query_map([program_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
    })?.collect::<rusqlite::Result<Vec<_>>>()?;

    let mut program_popups = Vec::new();
    for (pp_id, popup_id, trigger_type, duration, frequency) in rows {
        let popup = get_popup(conn, popup_id)?;
        program_popups.push(ProgramPopup {
            id: pp_id,
            program_id,
            popup_id,
            trigger_type,
            duration,
            frequency,
            popup,
        });
    }
    Ok(program_popups)
}
