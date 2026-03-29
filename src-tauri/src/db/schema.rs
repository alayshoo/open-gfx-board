use rusqlite::Connection;
use anyhow::Result;

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);",
    )?;

    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    // Migration 1 — initial schema
    if version < 1 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS studios (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT NOT NULL,
                 obs_browser_source_address TEXT NOT NULL DEFAULT '',
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE TABLE IF NOT EXISTS presets (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 studio_id INTEGER NOT NULL REFERENCES studios(id) ON DELETE CASCADE,
                 name TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS obs_commands (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 preset_id INTEGER NOT NULL REFERENCES presets(id) ON DELETE CASCADE,
                 name TEXT NOT NULL,
                 color TEXT NOT NULL DEFAULT '#38bdf8',
                 shortcut TEXT NOT NULL,
                 description TEXT NOT NULL DEFAULT ''
             );
             CREATE TABLE IF NOT EXISTS programs (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT NOT NULL,
                 logo_path TEXT,
                 bg_path TEXT,
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE TABLE IF NOT EXISTS screens (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT NOT NULL,
                 comments TEXT NOT NULL DEFAULT '',
                 media_path TEXT,
                 media_type TEXT NOT NULL DEFAULT 'image',
                 allow_ads INTEGER NOT NULL DEFAULT 1,
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE TABLE IF NOT EXISTS program_screens (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 program_id INTEGER NOT NULL REFERENCES programs(id) ON DELETE CASCADE,
                 screen_id INTEGER NOT NULL REFERENCES screens(id) ON DELETE CASCADE,
                 UNIQUE(program_id, screen_id)
             );
             CREATE TABLE IF NOT EXISTS advertisements (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT NOT NULL,
                 sponsor_name TEXT NOT NULL DEFAULT '',
                 comments TEXT NOT NULL DEFAULT '',
                 media_path TEXT,
                 media_type TEXT NOT NULL DEFAULT 'image',
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE TABLE IF NOT EXISTS program_ads (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 program_id INTEGER NOT NULL REFERENCES programs(id) ON DELETE CASCADE,
                 ad_id INTEGER NOT NULL REFERENCES advertisements(id) ON DELETE CASCADE,
                 trigger_type TEXT NOT NULL DEFAULT 'manual'
                     CHECK(trigger_type IN ('manual','automatic','both','filler')),
                 duration INTEGER NOT NULL DEFAULT 10,
                 frequency INTEGER NOT NULL DEFAULT 1,
                 UNIQUE(program_id, ad_id)
             );
             CREATE TABLE IF NOT EXISTS studio_state (
                 studio_id        INTEGER PRIMARY KEY REFERENCES studios(id) ON DELETE CASCADE,
                 program_id       INTEGER REFERENCES programs(id) ON DELETE SET NULL,
                 active_screen_id INTEGER REFERENCES screens(id) ON DELETE SET NULL,
                 active_ad_id     INTEGER REFERENCES advertisements(id) ON DELETE SET NULL
             );
             INSERT INTO schema_version VALUES (1);",
        )?;
    }

    // Add future migrations here, e.g.:
    // if version < 2 {
    //     conn.execute_batch(
    //         "ALTER TABLE screens ADD COLUMN some_new_col TEXT NOT NULL DEFAULT '';
    //          INSERT INTO schema_version VALUES (2);",
    //     )?;
    // }

    Ok(())
}
