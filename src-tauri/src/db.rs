//! Base SQLite : schéma, migrations et accès de bas niveau.
use crate::models::{GameSource, LaunchMethod, SourceId};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::path::Path;

const MIGRATIONS: &[&str] = &[
    // v1 : schéma complet du §5 du cahier des charges.
    r#"
    CREATE TABLE games (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        sort_title TEXT NOT NULL,
        igdb_id INTEGER,
        steamgrid_id INTEGER,
        meta_steam_appid INTEGER,
        cover_path TEXT,
        hero_path TEXT,
        logo_path TEXT,
        description TEXT,
        genres TEXT,
        developer TEXT,
        release_date TEXT,
        favorite INTEGER NOT NULL DEFAULT 0,
        hidden INTEGER NOT NULL DEFAULT 0,
        added_at TEXT NOT NULL,
        custom_title INTEGER NOT NULL DEFAULT 0,
        custom_cover INTEGER NOT NULL DEFAULT 0,
        manual_playtime_minutes INTEGER,
        images_checked INTEGER NOT NULL DEFAULT 0,
        details_checked INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE game_sources (
        id TEXT PRIMARY KEY,
        game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
        source TEXT NOT NULL,
        external_id TEXT NOT NULL,
        installed INTEGER NOT NULL DEFAULT 0,
        install_path TEXT,
        install_path_norm TEXT,
        volume_serial TEXT,
        executable_path TEXT,
        launch_method TEXT NOT NULL,
        launch_target TEXT NOT NULL,
        launch_args TEXT,
        working_dir TEXT,
        source_playtime_minutes INTEGER,
        last_played_source TEXT,
        available INTEGER NOT NULL DEFAULT 1,
        owned INTEGER NOT NULL DEFAULT 0,
        achievements_synced_at TEXT,
        UNIQUE(source, external_id)
    );
    CREATE INDEX idx_sources_game ON game_sources(game_id);
    CREATE INDEX idx_sources_path ON game_sources(install_path_norm);
    CREATE TABLE play_sessions (
        id TEXT PRIMARY KEY,
        game_source_id TEXT NOT NULL REFERENCES game_sources(id) ON DELETE CASCADE,
        started_at TEXT NOT NULL,
        ended_at TEXT NOT NULL,
        duration_seconds INTEGER NOT NULL DEFAULT 0,
        active INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX idx_sessions_source ON play_sessions(game_source_id);
    CREATE TABLE achievements (
        id TEXT PRIMARY KEY,
        game_source_id TEXT NOT NULL REFERENCES game_sources(id) ON DELETE CASCADE,
        api_name TEXT NOT NULL,
        name TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        icon_unlocked TEXT,
        icon_locked TEXT,
        hidden INTEGER NOT NULL DEFAULT 0,
        unlocked INTEGER NOT NULL DEFAULT 0,
        unlocked_at TEXT,
        global_percent REAL,
        UNIQUE(game_source_id, api_name)
    );
    CREATE TABLE watched_folders (
        id TEXT PRIMARY KEY,
        path TEXT NOT NULL UNIQUE,
        added_at TEXT NOT NULL
    );
    CREATE TABLE ignored_candidates (path_norm TEXT PRIMARY KEY);
    CREATE TABLE collections (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL UNIQUE
    );
    CREATE TABLE collection_games (
        collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
        game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
        PRIMARY KEY (collection_id, game_id)
    );
    CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
    "#,
];

pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    init(&conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    init(&conn)?;
    Ok(conn)
}

fn init(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(conn)
}

pub fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        conn.execute_batch(&format!("BEGIN; {sql} PRAGMA user_version = {}; COMMIT;", i + 1))?;
    }
    Ok(())
}

pub const SOURCE_COLS: &str = "id, game_id, source, external_id, installed, install_path, volume_serial, executable_path, launch_method, launch_target, launch_args, working_dir, source_playtime_minutes, last_played_source, available, owned";

pub fn row_to_source(r: &Row) -> rusqlite::Result<GameSource> {
    let source: String = r.get(2)?;
    let method: String = r.get(8)?;
    Ok(GameSource {
        id: r.get(0)?,
        game_id: r.get(1)?,
        source: SourceId::parse(&source).unwrap_or(SourceId::Local),
        external_id: r.get(3)?,
        installed: r.get(4)?,
        install_path: r.get(5)?,
        volume_serial: r.get(6)?,
        executable_path: r.get(7)?,
        launch_method: LaunchMethod::parse(&method),
        launch_target: r.get(9)?,
        launch_args: r.get(10)?,
        working_dir: r.get(11)?,
        source_playtime_minutes: r.get(12)?,
        last_played_source: r.get(13)?,
        available: r.get(14)?,
        owned: r.get(15)?,
    })
}

pub fn find_source(conn: &Connection, source: SourceId, external_id: &str) -> Result<Option<GameSource>> {
    Ok(conn
        .query_row(
            &format!("SELECT {SOURCE_COLS} FROM game_sources WHERE source = ?1 AND external_id = ?2"),
            params![source.as_str(), external_id],
            row_to_source,
        )
        .optional()?)
}

pub fn get_source(conn: &Connection, id: &str) -> Result<Option<GameSource>> {
    Ok(conn
        .query_row(&format!("SELECT {SOURCE_COLS} FROM game_sources WHERE id = ?1"), params![id], row_to_source)
        .optional()?)
}

pub fn sources_of_game(conn: &Connection, game_id: &str) -> Result<Vec<GameSource>> {
    let mut stmt = conn.prepare(&format!("SELECT {SOURCE_COLS} FROM game_sources WHERE game_id = ?1"))?;
    let rows = stmt.query_map(params![game_id], row_to_source)?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn all_sources(conn: &Connection) -> Result<Vec<GameSource>> {
    let mut stmt = conn.prepare(&format!("SELECT {SOURCE_COLS} FROM game_sources"))?;
    let rows = stmt.query_map([], row_to_source)?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| r.get(0)).optional().ok().flatten()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings(key, value) VALUES(?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn delete_setting(conn: &Connection, key: &str) -> Result<()> {
    conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
    Ok(())
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_idempotent() {
        let conn = open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let v: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(v, MIGRATIONS.len() as i64);
    }

    #[test]
    fn settings_roundtrip() {
        let conn = open_in_memory().unwrap();
        assert_eq!(get_setting(&conn, "k"), None);
        set_setting(&conn, "k", "1").unwrap();
        set_setting(&conn, "k", "2").unwrap();
        assert_eq!(get_setting(&conn, "k").as_deref(), Some("2"));
    }
}
