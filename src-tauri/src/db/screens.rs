use anyhow::Result;
use rusqlite::Connection;
use crate::models::Screen;

pub fn get_all_screens(conn: &Connection) -> Result<Vec<Screen>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, comments, media_path, media_type, allow_popups, created_at FROM screens ORDER BY id"
    )?;
    let screens: Vec<Screen> = stmt.query_map([], |row| {
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

pub fn get_screen(conn: &Connection, id: i64) -> Result<Option<Screen>> {
    let result = conn.query_row(
        "SELECT id, name, comments, media_path, media_type, allow_popups, created_at FROM screens WHERE id = ?1",
        [id],
        |row| {
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
        },
    );
    match result {
        Ok(screen) => Ok(Some(screen)),
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
) -> Result<Screen> {
    conn.execute(
        "INSERT INTO screens (name, comments, allow_popups, media_type) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![name, comments, allow_popups as i64, media_type],
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
) -> Result<Option<Screen>> {
    let rows = conn.execute(
        "UPDATE screens SET name = ?1, comments = ?2, allow_popups = ?3, media_type = ?4 WHERE id = ?5",
        rusqlite::params![name, comments, allow_popups as i64, media_type, id],
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
