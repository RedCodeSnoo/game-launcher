//! Sauvegarde et restauration de la base (F-PAR-06) : heures, sessions, collections, préférences.
//! Les clés API ne sont pas dans la base (coffre Windows) : une sauvegarde n'en contient donc jamais.
use crate::db;
use crate::error::AppError;
use anyhow::{Context, Result};
use rusqlite::{backup::Backup, Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Exporte une copie cohérente de la base vers `dest`.
pub fn export(conn: &Connection, dest: &Path) -> Result<()> {
    if dest.exists() {
        std::fs::remove_file(dest).context("remplacement du fichier de sauvegarde")?;
    }
    conn.execute("VACUUM INTO ?1", [dest.to_string_lossy().to_string()]).context("écriture de la sauvegarde")?;
    Ok(())
}

/// Restaure la base depuis une sauvegarde, après vérification qu'il s'agit bien d'une base du launcher.
pub fn import(conn: &mut Connection, source: &Path) -> Result<()> {
    let src = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|_| AppError::new("BACKUP_INVALID", source.to_string_lossy().to_string()))?;
    let is_ours = src
        .query_row("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('games', 'game_sources', 'play_sessions')", [], |r| r.get::<_, i64>(0))
        .map(|n| n == 3)
        .unwrap_or(false);
    if !is_ours {
        return Err(AppError::new("BACKUP_INVALID", source.to_string_lossy().to_string()).into());
    }
    {
        let backup = Backup::new(&src, conn)?;
        backup.run_to_completion(200, Duration::from_millis(0), None)?;
    }
    conn.pragma_update(None, "foreign_keys", "ON")?;
    db::migrate(conn)?;
    // Les images ne font pas partie de la sauvegarde : on oublie celles qui manquent pour les récupérer à nouveau.
    forget_missing_images(conn)?;
    Ok(())
}

fn forget_missing_images(conn: &Connection) -> Result<()> {
    let rows: Vec<(String, Option<String>, Option<String>, Option<String>)> = {
        let mut stmt = conn.prepare("SELECT id, cover_path, hero_path, logo_path FROM games")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    for (id, cover, hero, logo) in rows {
        let gone = |p: &Option<String>| p.as_deref().map(|p| !Path::new(p).is_file()).unwrap_or(false);
        if gone(&cover) || gone(&hero) || gone(&logo) {
            conn.execute(
                "UPDATE games SET images_checked = 0,
                    cover_path = CASE WHEN ?2 THEN NULL ELSE cover_path END,
                    custom_cover = CASE WHEN ?2 THEN 0 ELSE custom_cover END,
                    hero_path = CASE WHEN ?3 THEN NULL ELSE hero_path END,
                    logo_path = CASE WHEN ?4 THEN NULL ELSE logo_path END
                 WHERE id = ?1",
                rusqlite::params![id, gone(&cover), gone(&hero), gone(&logo)],
            )?;
        }
    }
    Ok(())
}

pub fn backups_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

/// Sauvegarde automatique : une par jour au démarrage, les 7 plus récentes sont conservées.
pub fn daily(conn: &Connection, data_dir: &Path) -> Result<Option<PathBuf>> {
    let dir = backups_dir(data_dir);
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(format!("library-{}.db", chrono::Local::now().format("%Y%m%d")));
    if dest.exists() {
        return Ok(None);
    }
    export(conn, &dest)?;
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("library-")).unwrap_or(false))
        .collect();
    files.sort();
    let excess = files.len().saturating_sub(7);
    for old in files.into_iter().take(excess) {
        let _ = std::fs::remove_file(old);
    }
    Ok(Some(dest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library;

    fn seeded() -> Connection {
        let conn = db::open_in_memory().unwrap();
        let game = library::insert_game(&conn, "Alpha").unwrap();
        conn.execute(
            "INSERT INTO game_sources(id, game_id, source, external_id, installed, launch_method, launch_target) VALUES('s1', ?1, 'local', 'x', 1, 'exe', 'C:\\x.exe')",
            [&game],
        )
        .unwrap();
        conn.execute("INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds) VALUES('p1', 's1', '2026-01-01T10:00:00Z', '2026-01-01T11:00:00Z', 3600)", []).unwrap();
        conn.execute("UPDATE games SET favorite = 1, cover_path = 'Z:\\absent\\cover.jpg', images_checked = 1", []).unwrap();
        db::set_setting(&conn, "steam_id", "76561198000000001").unwrap();
        conn
    }

    #[test]
    fn export_then_import_restores_everything() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("sauvegarde.db");
        let conn = seeded();
        export(&conn, &file).unwrap();
        export(&conn, &file).unwrap(); // remplacer un fichier existant est permis

        let mut fresh = db::open_in_memory().unwrap();
        import(&mut fresh, &file).unwrap();
        let games = library::list_games(&fresh).unwrap();
        assert_eq!(games.len(), 1);
        assert!(games[0].favorite);
        assert_eq!(games[0].tracked_seconds, 3600);
        assert_eq!(games[0].cover_path, None, "image absente du disque : elle sera récupérée à nouveau");
        assert_eq!(db::get_setting(&fresh, "steam_id").as_deref(), Some("76561198000000001"));
    }

    #[test]
    fn import_rejects_foreign_files() {
        let tmp = tempfile::tempdir().unwrap();
        let text = tmp.path().join("notes.db");
        std::fs::write(&text, "pas une base").unwrap();
        let other = tmp.path().join("autre.db");
        Connection::open(&other).unwrap().execute_batch("CREATE TABLE t(x);").unwrap();
        let mut conn = seeded();
        for file in [&text, &other, &tmp.path().join("absent.db")] {
            let err: AppError = import(&mut conn, file).unwrap_err().into();
            assert_eq!(err.code, "BACKUP_INVALID");
        }
        assert_eq!(library::list_games(&conn).unwrap().len(), 1, "la base en place n'est pas touchée");
    }

    #[test]
    fn backup_never_contains_api_keys() {
        // Les secrets vivent dans le coffre Windows ; la base n'a aucune colonne ni réglage pour eux.
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("b.db");
        export(&seeded(), &file).unwrap();
        let conn = Connection::open(&file).unwrap();
        let keys: Vec<String> = conn.prepare("SELECT key FROM settings").unwrap().query_map([], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect();
        assert!(keys.iter().all(|k| !k.contains("key") && !k.contains("token") && !k.contains("secret")));
    }

    #[test]
    fn daily_backup_runs_once_a_day() {
        let tmp = tempfile::tempdir().unwrap();
        let conn = seeded();
        assert!(daily(&conn, tmp.path()).unwrap().is_some());
        assert!(daily(&conn, tmp.path()).unwrap().is_none());
        assert_eq!(std::fs::read_dir(backups_dir(tmp.path())).unwrap().count(), 1);
    }
}
