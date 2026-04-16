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

    if version < 2 {
        conn.execute_batch(
            "ALTER TABLE advertisements ADD COLUMN direction TEXT NOT NULL DEFAULT 'bottom';
             ALTER TABLE advertisements ADD COLUMN position INTEGER NOT NULL DEFAULT 50;
             INSERT INTO schema_version VALUES (2);",
        )?;
    }

    if version < 3 {
        conn.execute_batch(
            "ALTER TABLE advertisements RENAME TO popups;
             ALTER TABLE program_ads RENAME TO program_popups;
             ALTER TABLE program_popups RENAME COLUMN ad_id TO popup_id;
             ALTER TABLE studio_state RENAME COLUMN active_ad_id TO active_popup_id;
             ALTER TABLE screens RENAME COLUMN allow_ads TO allow_popups;
             INSERT INTO schema_version VALUES (3);",
        )?;
    }

    // Migration 4 — HTML content support for screens and popups.
    // When media_type = 'html', the html_content column holds the raw HTML
    // template string (may contain {{var:…}} / {{db:…}} expressions that are
    // resolved at display time).
    if version < 4 {
        conn.execute_batch(
            "ALTER TABLE screens ADD COLUMN html_content TEXT;
             ALTER TABLE popups  ADD COLUMN html_content TEXT;
             INSERT INTO schema_version VALUES (4);",
        )?;
    }

    // Migration 5 — user-configurable popup dimensions.
    // Nullable: when NULL, image/video popups use their natural media size;
    // HTML popups fall back to a sensible default on the client.
    if version < 5 {
        conn.execute_batch(
            "ALTER TABLE popups ADD COLUMN width  INTEGER;
             ALTER TABLE popups ADD COLUMN height INTEGER;
             INSERT INTO schema_version VALUES (5);",
        )?;
    }

    // Migration 6 — plugin system core tables.
    if version < 6 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS plugins (
                 id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 version TEXT NOT NULL,
                 description TEXT NOT NULL DEFAULT '',
                 author TEXT NOT NULL DEFAULT '',
                 enabled INTEGER NOT NULL DEFAULT 1,
                 manifest_json TEXT NOT NULL,
                 installed_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE TABLE IF NOT EXISTS plugin_state (
                 plugin_id TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
                 key TEXT NOT NULL,
                 value TEXT,
                 PRIMARY KEY (plugin_id, key)
             );
             INSERT INTO schema_version VALUES (6);",
        )?;
    }

    // Migration 7 — link plugin-managed screens and popups.
    if version < 7 {
        conn.execute_batch(
            "ALTER TABLE screens ADD COLUMN plugin_id TEXT REFERENCES plugins(id) ON DELETE CASCADE;
             ALTER TABLE screens ADD COLUMN plugin_template_id TEXT;
             ALTER TABLE popups  ADD COLUMN plugin_id TEXT REFERENCES plugins(id) ON DELETE CASCADE;
             ALTER TABLE popups  ADD COLUMN plugin_template_id TEXT;
             INSERT INTO schema_version VALUES (7);",
        )?;
    }

    // Migration 8 — distinguish bundled (shipped with the app) plugins from user-installed ones.
    if version < 8 {
        conn.execute_batch(
            "ALTER TABLE plugins ADD COLUMN is_bundled INTEGER NOT NULL DEFAULT 0;
             INSERT INTO schema_version VALUES (8);",
        )?;
    }

    // Migration 9 — add 'hidden' launch type.
    // SQLite cannot ALTER a CHECK constraint, so we recreate program_popups.
    if version < 9 {
        conn.execute_batch(
            "PRAGMA foreign_keys=OFF;
             CREATE TABLE program_popups_new (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 program_id INTEGER NOT NULL REFERENCES programs(id) ON DELETE CASCADE,
                 popup_id   INTEGER NOT NULL REFERENCES popups(id)   ON DELETE CASCADE,
                 trigger_type TEXT NOT NULL DEFAULT 'manual'
                     CHECK(trigger_type IN ('manual','automatic','both','filler','hidden')),
                 duration  INTEGER NOT NULL DEFAULT 10,
                 frequency INTEGER NOT NULL DEFAULT 1,
                 UNIQUE(program_id, popup_id)
             );
             INSERT INTO program_popups_new SELECT * FROM program_popups;
             DROP TABLE program_popups;
             ALTER TABLE program_popups_new RENAME TO program_popups;
             PRAGMA foreign_keys=ON;
             INSERT INTO schema_version VALUES (9);",
        )?;
    }

    // Migration 10 — per-program plugin preferences stored server-side.
    // Replaces the previous localStorage-based approach so preferences roam
    // across all clients (Tauri window, browser, remote devices).
    if version < 10 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS program_plugin_prefs (
                 program_id INTEGER PRIMARY KEY REFERENCES programs(id) ON DELETE CASCADE,
                 plugin_ids TEXT NOT NULL DEFAULT '[]'
             );
             INSERT INTO schema_version VALUES (10);",
        )?;
    }

    // Migration 11 — per-program plugin popup overrides.
    // Allows users to choose which popup is triggered for each plugin popup
    // template on a per-program basis, overriding the plugin's built-in default.
    if version < 11 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS program_plugin_popup_overrides (
                 program_id  INTEGER NOT NULL REFERENCES programs(id) ON DELETE CASCADE,
                 plugin_id   TEXT    NOT NULL,
                 template_id TEXT    NOT NULL,
                 popup_id    INTEGER REFERENCES popups(id) ON DELETE SET NULL,
                 PRIMARY KEY (program_id, plugin_id, template_id)
             );
             INSERT INTO schema_version VALUES (11);",
        )?;
    }

    // Migration 12 — add duration to plugin popup overrides.
    if version < 12 {
        conn.execute_batch(
            "ALTER TABLE program_plugin_popup_overrides ADD COLUMN duration INTEGER NOT NULL DEFAULT 10;
             INSERT INTO schema_version VALUES (12);",
        )?;
    }

    // Migration 13 — layers support.
    // Each screen and popup in a program can be assigned to one of three layers
    // (1 = top, 2 = middle, 3 = bottom).  Multiple items can be active
    // simultaneously as long as they are on different layers.
    if version < 13 {
        conn.execute_batch(
            "ALTER TABLE program_screens ADD COLUMN layer INTEGER NOT NULL DEFAULT 1;
             ALTER TABLE program_popups  ADD COLUMN layer INTEGER NOT NULL DEFAULT 1;
             INSERT INTO schema_version VALUES (13);",
        )?;
    }

    Ok(())
}
