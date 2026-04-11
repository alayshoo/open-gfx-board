use anyhow::Result;
use rusqlite::Connection;
use crate::models::{Screen, ScreenProgram};

pub fn get_all_screens(conn: &Connection) -> Result<Vec<Screen>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, comments, media_path, media_type, allow_popups, html_content, created_at FROM screens ORDER BY id"
    )?;
    let rows: Vec<(i64, String, String, Option<String>, String, i64, Option<String>, String)> = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?))
    })?.collect::<rusqlite::Result<Vec<_>>>()?;

    let mut screens = Vec::new();
    for (id, name, comments, media_path, media_type, allow_popups_int, html_content, created_at) in rows {
        let programs = load_programs_for_screen(conn, id)?;
        screens.push(Screen {
            id,
            name,
            comments,
            media_path,
            media_type,
            allow_popups: allow_popups_int != 0,
            html_content,
            programs,
            created_at,
        });
    }
    Ok(screens)
}

pub fn get_screen(conn: &Connection, id: i64) -> Result<Option<Screen>> {
    let result = conn.query_row(
        "SELECT id, name, comments, media_path, media_type, allow_popups, html_content, created_at FROM screens WHERE id = ?1",
        [id],
        |row| Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, String>(7)?,
        )),
    );
    match result {
        Ok((sid, name, comments, media_path, media_type, allow_popups_int, html_content, created_at)) => {
            let programs = load_programs_for_screen(conn, sid)?;
            Ok(Some(Screen {
                id: sid,
                name,
                comments,
                media_path,
                media_type,
                allow_popups: allow_popups_int != 0,
                html_content,
                programs,
                created_at,
            }))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn create_screen(
    conn: &Connection,
    name: &str,
    comments: &str,
    allow_popups: bool,
    media_type: &str,
    html_content: Option<&str>,
) -> Result<Screen> {
    conn.execute(
        "INSERT INTO screens (name, comments, allow_popups, media_type, html_content) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![name, comments, allow_popups as i64, media_type, html_content],
    )?;
    let id = conn.last_insert_rowid();
    Ok(get_screen(conn, id)?.expect("screen just inserted must exist"))
}

pub fn update_screen(
    conn: &Connection,
    id: i64,
    name: &str,
    comments: &str,
    allow_popups: bool,
    media_type: &str,
    html_content: Option<&str>,
) -> Result<Option<Screen>> {
    let rows = conn.execute(
        "UPDATE screens SET name = ?1, comments = ?2, allow_popups = ?3, media_type = ?4, html_content = ?5 WHERE id = ?6",
        rusqlite::params![name, comments, allow_popups as i64, media_type, html_content, id],
    )?;
    if rows == 0 {
        return Ok(None);
    }
    Ok(get_screen(conn, id)?)
}

pub fn delete_screen(conn: &Connection, id: i64) -> Result<bool> {
    let rows = conn.execute("DELETE FROM screens WHERE id = ?1", [id])?;
    Ok(rows > 0)
}

pub fn set_media_path(conn: &Connection, id: i64, path: &str) -> Result<()> {
    conn.execute(
        "UPDATE screens SET media_path = ?1 WHERE id = ?2",
        rusqlite::params![path, id],
    )?;
    Ok(())
}

fn load_programs_for_screen(conn: &Connection, screen_id: i64) -> Result<Vec<ScreenProgram>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name FROM programs p
         JOIN program_screens ps ON ps.program_id = p.id
         WHERE ps.screen_id = ?1
         ORDER BY p.id"
    )?;
    let programs: Vec<ScreenProgram> = stmt.query_map([screen_id], |row| {
        Ok(ScreenProgram {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(programs)
}
