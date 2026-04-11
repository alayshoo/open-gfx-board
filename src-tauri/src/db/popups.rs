use anyhow::Result;
use rusqlite::Connection;
use crate::models::{PopupProgram, Popup};

pub fn get_all_popups(conn: &Connection) -> Result<Vec<Popup>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, sponsor_name, comments, media_path, media_type, html_content, direction, position, width, height, created_at FROM popups ORDER BY id"
    )?;

    let mut popups = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let programs = load_programs_for_popup(conn, id)?;
        popups.push(Popup {
            id,
            name: row.get(1)?,
            sponsor_name: row.get(2)?,
            comments: row.get(3)?,
            media_path: row.get(4)?,
            media_type: row.get(5)?,
            html_content: row.get(6)?,
            direction: row.get(7)?,
            position: row.get(8)?,
            width: row.get(9)?,
            height: row.get(10)?,
            programs,
            created_at: row.get(11)?,
        });
    }
    Ok(popups)
}

pub fn get_popup(conn: &Connection, id: i64) -> Result<Option<Popup>> {
    let result = conn.query_row(
        "SELECT id, name, sponsor_name, comments, media_path, media_type, html_content, direction, position, width, height, created_at FROM popups WHERE id = ?1",
        [id],
        |row| Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, i64>(8)?,
            row.get::<_, Option<i64>>(9)?,
            row.get::<_, Option<i64>>(10)?,
            row.get::<_, String>(11)?,
        )),
    );
    match result {
        Ok((pid, name, sponsor_name, comments, media_path, media_type, html_content, direction, position, width, height, created_at)) => {
            let programs = load_programs_for_popup(conn, pid)?;
            Ok(Some(Popup {
                id: pid,
                name,
                sponsor_name,
                comments,
                media_path,
                media_type,
                html_content,
                direction,
                position,
                width,
                height,
                programs,
                created_at,
            }))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn create_popup(conn: &Connection, name: &str, sponsor_name: &str, comments: &str, direction: &str, position: i64, media_type: &str, html_content: Option<&str>, width: Option<i64>, height: Option<i64>) -> Result<Popup> {
    conn.execute(
        "INSERT INTO popups (name, sponsor_name, comments, direction, position, media_type, html_content, width, height) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![name, sponsor_name, comments, direction, position, media_type, html_content, width, height],
    )?;
    let id = conn.last_insert_rowid();
    Ok(get_popup(conn, id)?.expect("popup just inserted must exist"))
}

pub fn update_popup(conn: &Connection, id: i64, name: &str, sponsor_name: &str, comments: &str, direction: &str, position: i64, media_type: &str, html_content: Option<&str>, width: Option<i64>, height: Option<i64>) -> Result<Option<Popup>> {
    let rows = conn.execute(
        "UPDATE popups SET name = ?1, sponsor_name = ?2, comments = ?3, direction = ?4, position = ?5, media_type = ?6, html_content = ?7, width = ?8, height = ?9 WHERE id = ?10",
        rusqlite::params![name, sponsor_name, comments, direction, position, media_type, html_content, width, height, id],
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
