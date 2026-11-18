//! Sauvegarde et synchronisation vers un dossier choisi (V1.5, I-06) : OneDrive, Google Drive, clé USB…
//! Le dossier reçoit `GameLauncher/library.db` (heures, sessions, notes, thèmes perso, réglages ; jamais les clés,
//! qui restent dans le coffre Windows) et `GameLauncher/sync.json` (date, machine, nombre de jeux).
use crate::db;
use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SUBDIR: &str = "GameLauncher";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub app_version: String,
    pub exported_at: String,
    pub machine: String,
    pub games: i64,
    pub sessions: i64,
}

pub fn target(folder: &Path) -> PathBuf {
    folder.join(SUBDIR)
}

pub fn read_manifest(folder: &Path) -> Option<Manifest> {
    serde_json::from_str(&std::fs::read_to_string(target(folder).join("sync.json")).ok()?).ok()
}

/// Copie cohérente de la base dans le dossier, écrite à côté puis renommée (pas de fichier à moitié copié).
pub fn export(conn: &Connection, folder: &Path) -> Result<Manifest> {
    let dir = target(folder);
    std::fs::create_dir_all(&dir).context("création du dossier de synchronisation")?;
    let tmp = dir.join("library.db.tmp");
    let _ = std::fs::remove_file(&tmp);
    crate::backup::export(conn, &tmp)?;
    std::fs::rename(&tmp, dir.join("library.db")).context("remplacement de la sauvegarde")?;
    let count = |sql: &str| conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0);
    let manifest = Manifest {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: crate::models::now_iso(),
        machine: std::env::var("COMPUTERNAME").unwrap_or_else(|_| "ce PC".into()),
        games: count("SELECT COUNT(*) FROM games"),
        sessions: count("SELECT COUNT(*) FROM play_sessions"),
    };
    std::fs::write(dir.join("sync.json"), serde_json::to_string_pretty(&manifest)?)?;
    db::set_setting(conn, "sync_last", &manifest.exported_at)?;
    db::set_setting(conn, "sync_signature", &signature(conn))?;
    Ok(manifest)
}

/// Restaure la base depuis le dossier (remplace la bibliothèque actuelle). Le dossier choisi est conservé.
pub fn restore(conn: &mut Connection, folder: &Path) -> Result<Manifest> {
    let file = target(folder).join("library.db");
    if !file.is_file() {
        anyhow::bail!(crate::error::AppError::new("SYNC_EMPTY", folder.to_string_lossy().to_string()));
    }
    let manifest = read_manifest(folder);
    crate::backup::import(conn, &file)?;
    db::set_setting(conn, "sync_folder", &folder.to_string_lossy())?;
    db::set_setting(conn, "sync_last", &crate::models::now_iso())?;
    Ok(manifest.unwrap_or(Manifest { app_version: String::new(), exported_at: String::new(), machine: String::new(), games: 0, sessions: 0 }))
}

/// Empreinte des données : une synchronisation automatique n'a lieu que si elle a changé.
pub fn signature(conn: &Connection) -> String {
    conn.query_row(
        "SELECT (SELECT COUNT(*) FROM games) || '|' || (SELECT COALESCE(SUM(duration_seconds), 0) FROM play_sessions) || '|' ||
                (SELECT COUNT(*) FROM play_sessions) || '|' || (SELECT COALESCE(SUM(LENGTH(value)), 0) FROM settings WHERE key NOT LIKE 'sync_%' AND key NOT LIKE 'activity_%' AND key NOT LIKE 'epic_free_%') || '|' ||
                (SELECT COALESCE(SUM(LENGTH(COALESCE(notes, '')) + COALESCE(rating, 0) + LENGTH(COALESCE(status, '')) + favorite + hidden), 0) FROM games)",
        [],
        |r| r.get(0),
    )
    .unwrap_or_default()
}

/// Synchronisation automatique : à faire si l'option est active, un dossier choisi et les données modifiées.
pub fn auto_due(conn: &Connection) -> Option<PathBuf> {
    if db::get_setting(conn, "sync_auto").as_deref() != Some("1") {
        return None;
    }
    let folder = PathBuf::from(db::get_setting(conn, "sync_folder").filter(|f| !f.is_empty())?);
    (db::get_setting(conn, "sync_signature").as_deref() != Some(signature(conn).as_str())).then_some(folder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_then_restore_on_another_machine() {
        let tmp = tempfile::tempdir().unwrap();
        let cloud = tmp.path().join("OneDrive");
        let a = db::open_in_memory().unwrap();
        a.execute_batch(
            "INSERT INTO games(id, title, sort_title, added_at, notes, rating, status) VALUES('g', 'Hades', 'hades', '2026-01-01T00:00:00Z', 'Finir Zagreus', 9, 'playing');
             INSERT INTO game_sources(id, game_id, source, external_id, launch_method, launch_target) VALUES('s', 'g', 'steam', '1145360', 'uri', 'x');
             INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds) VALUES('p', 's', '2026-11-01T10:00:00Z', '2026-11-01T12:00:00Z', 7200);
             INSERT INTO settings(key, value) VALUES('custom_themes', '[{\"id\":\"perso-1\"}]');",
        )
        .unwrap();
        db::set_setting(&a, "sync_folder", &cloud.to_string_lossy()).unwrap();
        db::set_setting(&a, "sync_auto", "1").unwrap();
        assert!(auto_due(&a).is_some(), "jamais synchronisé");
        let m = export(&a, &cloud).unwrap();
        assert_eq!((m.games, m.sessions), (1, 1));
        assert!(auto_due(&a).is_none(), "rien n'a changé depuis");
        a.execute("UPDATE games SET rating = 10", []).unwrap();
        assert!(auto_due(&a).is_some(), "la note a changé");
        assert_eq!(read_manifest(&cloud).unwrap().games, 1);

        // Autre PC : restauration depuis le même dossier.
        let mut b = db::open_in_memory().unwrap();
        restore(&mut b, &cloud).unwrap();
        let (notes, minutes): (String, i64) = b.query_row("SELECT notes, (SELECT SUM(duration_seconds) / 60 FROM play_sessions) FROM games", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((notes.as_str(), minutes), ("Finir Zagreus", 120));
        assert!(db::get_setting(&b, "custom_themes").unwrap().contains("perso-1"), "les thèmes perso suivent");
        let empty = tmp.path().join("Vide");
        assert!(restore(&mut b, &empty).is_err());
    }
}
