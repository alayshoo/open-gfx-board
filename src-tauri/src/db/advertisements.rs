use anyhow::Result;
use rusqlite::Connection;
use crate::models::{AdProgram, Advertisement};

pub fn get_all_ads(conn: &Connection) -> Result<Vec<Advertisement>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, sponsor_name, comments, media_path, media_type, created_at FROM advertisements ORDER BY id"
    )?;
    let rows: Vec<(i64, String, String, String, Option<String>, String, String)> = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
    })?.collect::<rusqlite::Result<Vec<_>>>()?;

    let mut ads = Vec::new();
    for (id, name, sponsor_name, comments, media_path, media_type, created_at) in rows {
        let programs = load_programs_for_ad(conn, id)?;
        ads.push(Advertisement {
            id,
            name,
            sponsor_name,
            comments,
            media_path,
            media_type,
            programs,
            created_at,
        });
    }
    Ok(ads)
}

pub fn get_ad(conn: &Connection, id: i64) -> Result<Option<Advertisement>> {
    let result = conn.query_row(
        "SELECT id, name, sponsor_name, comments, media_path, media_type, created_at FROM advertisements WHERE id = ?1",
        [id],
        |row| Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
        )),
    );
    match result {
        Ok((aid, name, sponsor_name, comments, media_path, media_type, created_at)) => {
            let programs = load_programs_for_ad(conn, aid)?;
            Ok(Some(Advertisement {
                id: aid,
                name,
                sponsor_name,
                comments,
                media_path,
                media_type,
                programs,
                created_at,
            }))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn create_ad(conn: &Connection, name: &str, sponsor_name: &str, comments: &str) -> Result<Advertisement> {
    conn.execute(
        "INSERT INTO advertisements (name, sponsor_name, comments) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, sponsor_name, comments],
    )?;
    let id = conn.last_insert_rowid();
    Ok(get_ad(conn, id)?.expect("ad just inserted must exist"))
}

pub fn update_ad(conn: &Connection, id: i64, name: &str, sponsor_name: &str, comments: &str) -> Result<Option<Advertisement>> {
    let rows = conn.execute(
        "UPDATE advertisements SET name = ?1, sponsor_name = ?2, comments = ?3 WHERE id = ?4",
        rusqlite::params![name, sponsor_name, comments, id],
    )?;
    if rows == 0 {
        return Ok(None);
    }
    Ok(get_ad(conn, id)?)
}

pub fn delete_ad(conn: &Connection, id: i64) -> Result<bool> {
    let rows = conn.execute("DELETE FROM advertisements WHERE id = ?1", [id])?;
    Ok(rows > 0)
}

pub fn set_media_path(conn: &Connection, id: i64, path: &str) -> Result<()> {
    conn.execute(
        "UPDATE advertisements SET media_path = ?1 WHERE id = ?2",
        rusqlite::params![path, id],
    )?;
    Ok(())
}

fn load_programs_for_ad(conn: &Connection, ad_id: i64) -> Result<Vec<AdProgram>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name FROM programs p
         JOIN program_ads pa ON pa.program_id = p.id
         WHERE pa.ad_id = ?1
         ORDER BY p.id"
    )?;
    let programs: Vec<AdProgram> = stmt.query_map([ad_id], |row| {
        Ok(AdProgram {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(programs)
}
