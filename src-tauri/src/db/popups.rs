use anyhow::Result;
use rusqlite::Connection;
use crate::models::{PopupProgram, Popup};

pub fn get_all_popups(conn: &Connection) -> Result<Vec<Popup>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, sponsor_name, comments, media_path, media_type, direction, position, created_at FROM popups ORDER BY id"
    )?;
    let rows: Vec<(i64, String, String, String, Option<String>, String, String, i64, String)> = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?))
    })?.collect::<rusqlite::Result<Vec<_>>>()?;

    let mut popups = Vec::new();
    for (id, name, sponsor_name, comments, media_path, media_type, direction, position, created_at) in rows {
        let programs = load_programs_for_popup(conn, id)?;
        popups.push(Popup {
            id,
            name,
            sponsor_name,
            comments,
            media_path,
            media_type,
            direction,
            position,
            programs,
            created_at,
        });
    }
    Ok(popups)
}

pub fn get_popup(conn: &Connection, id: i64) -> Result<Option<Popup>> {
    let result = conn.query_row(
        "SELECT id, name, sponsor_name, comments, media_path, media_type, direction, position, created_at FROM popups WHERE id = ?1",
        [id],
        |row| Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, String>(8)?,
        )),
    );
    match result {
        Ok((pid, name, sponsor_name, comments, media_path, media_type, direction, position, created_at)) => {
            let programs = load_programs_for_popup(conn, pid)?;
            Ok(Some(Popup {
                id: pid,
                name,
                sponsor_name,
                comments,
                media_path,
                media_type,
                direction,
                position,
                programs,
                created_at,
            }))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn create_popup(conn: &Connection, name: &str, sponsor_name: &str, comments: &str, direction: &str, position: i64, media_type: &str) -> Result<Popup> {
    conn.execute(
        "INSERT INTO popups (name, sponsor_name, comments, direction, position, media_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![name, sponsor_name, comments, direction, position, media_type],
    )?;
    let id = conn.last_insert_rowid();
    Ok(get_popup(conn, id)?.expect("popup just inserted must exist"))
}

pub fn update_popup(conn: &Connection, id: i64, name: &str, sponsor_name: &str, comments: &str, direction: &str, position: i64, media_type: &str) -> Result<Option<Popup>> {
    let rows = conn.execute(
        "UPDATE popups SET name = ?1, sponsor_name = ?2, comments = ?3, direction = ?4, position = ?5, media_type = ?6 WHERE id = ?7",
        rusqlite::params![name, sponsor_name, comments, direction, position, media_type, id],
    )?;
    if rows == 0 {
        return Ok(None);
    }
    Ok(get_popup(conn, id)?)
}

pub fn delete_popup(conn: &Connection, id: i64) -> Result<bool> {
    let rows = conn.execute("DELETE FROM popups WHERE id = ?1", [id])?;
    Ok(rows > 0)
}

pub fn set_media_path(conn: &Connection, id: i64, path: &str) -> Result<()> {
    conn.execute(
        "UPDATE popups SET media_path = ?1 WHERE id = ?2",
        rusqlite::params![path, id],
    )?;
    Ok(())
}

fn load_programs_for_popup(conn: &Connection, popup_id: i64) -> Result<Vec<PopupProgram>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name FROM programs p
         JOIN program_popups pp ON pp.program_id = p.id
         WHERE pp.popup_id = ?1
         ORDER BY p.id"
    )?;
    let programs: Vec<PopupProgram> = stmt.query_map([popup_id], |row| {
        Ok(PopupProgram {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(programs)
}
