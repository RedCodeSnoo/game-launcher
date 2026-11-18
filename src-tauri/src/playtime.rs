//! Calcul du temps de jeu total d'un jeu : la SEULE implémentation, utilisée par la bibliothèque (affichage et
//! tri), la fiche jeu, les statistiques et la zone de notification (via `GameView::playtime_minutes`).
//!
//! Règles, par ordre de priorité :
//! 1. Base manuelle (saisie par l'utilisateur) : base + sessions qui commencent après la saisie.
//! 2. Temps fourni par la source (Steam, GOG…) : ce temps + sessions jouées depuis sa dernière lecture.
//! 3. Sinon : somme de toutes les sessions (lancées depuis l'application ou détectées).
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Connection, OpenFlags};
use std::path::Path;

/// Temps d'une source du jeu.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SourcePart {
    /// Temps fourni par la source à sa dernière lecture (minutes), s'il existe.
    pub source_minutes: Option<i64>,
    /// Durée de toutes les sessions suivies pour cette source (secondes).
    pub session_seconds: i64,
    /// Durée des sessions postérieures à la dernière lecture du temps de la source (secondes).
    pub since_sync_seconds: i64,
}

impl SourcePart {
    /// Temps retenu pour cette source, en minutes (règles 2 et 3).
    pub fn minutes(&self) -> i64 {
        match self.source_minutes {
            Some(minutes) => minutes + self.since_sync_seconds / 60,
            None => self.session_seconds / 60,
        }
    }
}

/// Base manuelle : temps saisi par l'utilisateur et sessions jouées depuis la saisie.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ManualBase {
    pub minutes: i64,
    /// Durée des sessions qui commencent après la saisie (secondes).
    pub since_seconds: i64,
}

/// Temps total d'un jeu, en minutes.
pub fn game_total_minutes(parts: &[SourcePart], manual: Option<ManualBase>) -> i64 {
    match manual {
        Some(base) => base.minutes + base.since_seconds / 60,
        None => parts.iter().map(SourcePart::minutes).sum(),
    }
}

fn iso(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Saisies manuelles antérieures à la notion de base datée (migration v4) : leur date de saisie n'avait pas été
/// enregistrée. On la retrouve grâce aux sauvegardes automatiques quotidiennes : la saisie est datée de la
/// première sauvegarde qui contient déjà cette valeur (borne haute : on ne compte jamais une session qui aurait pu
/// être incluse dans le chiffre saisi). Sans sauvegarde correspondante, la date retenue est maintenant.
/// Renvoie le nombre de jeux datés.
pub fn date_legacy_manual_bases(conn: &Connection, backups_dir: &Path) -> anyhow::Result<usize> {
    let pending: Vec<(String, i64)> = {
        let mut stmt = conn.prepare("SELECT id, manual_baseline_minutes FROM games WHERE manual_baseline_minutes IS NOT NULL AND manual_baseline_at IS NULL")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    if pending.is_empty() {
        return Ok(0);
    }
    // Sauvegardes de la plus ancienne à la plus récente.
    let mut backups: Vec<(DateTime<Utc>, std::path::PathBuf)> = std::fs::read_dir(backups_dir)
        .map(|dir| {
            dir.flatten()
                .filter(|e| e.path().extension().map_or(false, |x| x == "db"))
                .filter_map(|e| Some((DateTime::<Utc>::from(e.metadata().ok()?.modified().ok()?), e.path())))
                .collect()
        })
        .unwrap_or_default();
    backups.sort();
    let snapshots: Vec<(DateTime<Utc>, Vec<(String, i64)>)> = backups.iter().filter_map(|(when, path)| Some((*when, manual_values(path)?))).collect();

    let now = iso(Utc::now());
    for (game_id, minutes) in &pending {
        let found = snapshots.iter().find(|(_, values)| values.iter().any(|(id, m)| id == game_id && m == minutes)).map(|(when, _)| iso(*when));
        let at = found.clone().unwrap_or_else(|| now.clone());
        conn.execute("UPDATE games SET manual_baseline_at = ?2 WHERE id = ?1", params![game_id, at])?;
        log::info!("base manuelle de {game_id} datée du {at}{}", if found.is_some() { " (d'après les sauvegardes)" } else { "" });
    }
    Ok(pending.len())
}

/// Valeurs manuelles d'une sauvegarde, quel que soit son schéma (avant ou après la migration v4).
fn manual_values(path: &Path) -> Option<Vec<(String, i64)>> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    let column = ["manual_baseline_minutes", "manual_playtime_minutes"]
        .into_iter()
        .find(|c| conn.prepare(&format!("SELECT {c} FROM games LIMIT 0")).is_ok())?;
    let mut stmt = conn.prepare(&format!("SELECT id, {column} FROM games WHERE {column} IS NOT NULL")).ok()?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).ok()?.collect::<rusqlite::Result<Vec<_>>>().ok()?;
    Some(rows)
}

/// Enregistre (ou efface avec `None`) la base manuelle d'un jeu ; la date de la base devient maintenant.
pub fn set_manual_base(conn: &Connection, game_id: &str, minutes: Option<i64>) -> rusqlite::Result<()> {
    match minutes {
        Some(m) => conn.execute(
            "UPDATE games SET manual_baseline_minutes = ?2, manual_baseline_at = ?3 WHERE id = ?1",
            params![game_id, m.max(0), iso(Utc::now())],
        )?,
        None => conn.execute("UPDATE games SET manual_baseline_minutes = NULL, manual_baseline_at = NULL WHERE id = ?1", params![game_id])?,
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_base_wins_and_grows_with_later_sessions() {
        let parts = [SourcePart { source_minutes: Some(600), session_seconds: 7200, since_sync_seconds: 3600 }];
        assert_eq!(game_total_minutes(&parts, None), 660, "temps de la source + sessions depuis sa lecture");
        assert_eq!(game_total_minutes(&parts, Some(ManualBase { minutes: 138_000, since_seconds: 3600 })), 138_060, "2300 h + 1 h");
        assert_eq!(game_total_minutes(&[SourcePart { source_minutes: None, session_seconds: 5400, since_sync_seconds: 0 }], None), 90);
        assert_eq!(game_total_minutes(&[], None), 0);
    }

    /// Vérification sur une COPIE de données réelles (dossier contenant `library.db` et `backups/`) :
    /// `GL_CHECK_DATA_DIR=<dossier> cargo test real_data_copy -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_data_copy() {
        let dir = std::path::PathBuf::from(std::env::var("GL_CHECK_DATA_DIR").expect("GL_CHECK_DATA_DIR"));
        let conn = crate::db::open(&dir.join("library.db")).unwrap();
        date_legacy_manual_bases(&conn, &dir.join("backups")).unwrap();
        let games = crate::library::list_games(&conn).unwrap();
        for g in games.iter().filter(|g| g.manual_baseline_minutes.is_some()) {
            println!(
                "{} : base {} h le {:?} + {} min depuis = {:.2} h",
                g.title,
                g.manual_baseline_minutes.unwrap() / 60,
                g.manual_baseline_at,
                g.since_manual_seconds / 60,
                g.playtime_minutes as f64 / 60.0
            );
        }
        let stats = crate::stats::compute(&conn, &std::collections::HashSet::new()).unwrap();
        for s in &stats.by_source {
            println!("stats {:?} : {:.1} h ({} jeux)", s.source, s.minutes as f64 / 60.0, s.games);
        }
        let t = std::time::Instant::now();
        let streaks = crate::activity::current_streaks(&conn).unwrap();
        let d = crate::dashboard::compute(&conn, &std::collections::HashSet::new(), crate::dashboard::Period::Month, &Default::default(), &chrono::Local, chrono::Utc::now()).unwrap();
        println!("série : {} jour(s), meilleure {} ({:?} → {:?}), en danger : {}", streaks.current, streaks.best, streaks.best_start, streaks.best_end, streaks.in_danger);
        println!("30 jours : {:.1} h de sessions, {} jeu(x), top : {:?}", d.session_seconds as f64 / 3600.0, d.games_played, d.top_games.iter().take(3).map(|g| (&g.title, g.minutes)).collect::<Vec<_>>());
        println!("records : {:?}", d.records.longest_session.as_ref().map(|r| (r.value, &r.game_title)));
        println!("badges obtenus : {:?} ; calcul en {:?}", d.badges.iter().filter(|b| b.earned).map(|b| b.name).collect::<Vec<_>>(), t.elapsed());
    }

    /// Temps de chaque calcul sur une copie des données réelles (GL_CHECK_DATA_DIR).
    #[test]
    #[ignore]
    fn real_data_timings() {
        let dir = std::path::PathBuf::from(std::env::var("GL_CHECK_DATA_DIR").expect("GL_CHECK_DATA_DIR"));
        let t = std::time::Instant::now();
        let conn = crate::db::open(&dir.join("library.db")).unwrap();
        println!("ouverture + migrations : {:?}", t.elapsed());
        let none = std::collections::HashSet::new();
        let t = std::time::Instant::now();
        crate::activity::ensure_fresh(&conn, &chrono::Local).unwrap();
        println!("ensure_fresh : {:?}", t.elapsed());
        let t = std::time::Instant::now();
        let games = crate::library::list_visible_games(&conn, &none).unwrap();
        println!("list_visible_games ({}) : {:?}", games.len(), t.elapsed());
        let t = std::time::Instant::now();
        let json = serde_json::to_string(&games).unwrap();
        println!("sérialisation : {} octets en {:?}", json.len(), t.elapsed());
        let t = std::time::Instant::now();
        crate::stats::compute(&conn, &none).unwrap();
        println!("stats : {:?}", t.elapsed());
        for p in [crate::dashboard::Period::Month, crate::dashboard::Period::All] {
            let t = std::time::Instant::now();
            crate::dashboard::compute(&conn, &none, p, &Default::default(), &chrono::Local, chrono::Utc::now()).unwrap();
            println!("dashboard : {:?}", t.elapsed());
        }
        let t = std::time::Instant::now();
        crate::activity::current_streaks(&conn).unwrap();
        println!("streaks : {:?}", t.elapsed());
        let t = std::time::Instant::now();
        crate::recap::compute(&conn, 2026, &chrono::Local, chrono::Local::now().date_naive()).unwrap();
        println!("recap : {:?}", t.elapsed());
        let t = std::time::Instant::now();
        crate::library::recent_games(&conn, &none, 5).unwrap();
        println!("recent_games : {:?}", t.elapsed());
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM play_sessions", [], |r| r.get(0)).unwrap();
        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        println!("sessions : {n}, version : {v}");
    }

    /// Base d'avant la migration v4 (sans date) : datée par la première sauvegarde qui contient la valeur.
    #[test]
    fn legacy_manual_entries_are_dated_from_the_backups() {
        let tmp = tempfile::tempdir().unwrap();
        let backups = tmp.path().join("backups");
        std::fs::create_dir_all(&backups).unwrap();
        // Sauvegardes au schéma d'avant (colonne manual_playtime_minutes) : la valeur apparaît dans la 2e.
        let make = |name: &str, value: Option<i64>, when: DateTime<Utc>| {
            let path = backups.join(name);
            let c = Connection::open(&path).unwrap();
            c.execute_batch("CREATE TABLE games (id TEXT PRIMARY KEY, manual_playtime_minutes INTEGER);").unwrap();
            c.execute("INSERT INTO games VALUES('lol', ?1), ('autre', NULL)", params![value]).unwrap();
            drop(c);
            let f = std::fs::File::options().write(true).open(&path).unwrap();
            f.set_modified(when.into()).unwrap();
        };
        let t = |s: &str| DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc);
        make("library-20261114.db", None, t("2026-11-14T12:37:18Z"));
        make("library-20261115.db", Some(138_000), t("2026-11-15T10:01:57Z"));
        make("library-20261116.db", Some(138_000), t("2026-11-16T16:50:12Z"));

        // Base de données réelle, migrée depuis le schéma v3 : la valeur est conservée, sans date.
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrate_to(&conn, 3).unwrap();
        conn.execute_batch(
            "INSERT INTO games(id, title, sort_title, added_at, manual_playtime_minutes) VALUES
               ('lol', 'League of Legends', 'lol', '2026-11-01T00:00:00Z', 138000),
               ('wow', 'World of Warcraft', 'wow', '2026-11-01T00:00:00Z', 6618);
             INSERT INTO game_sources(id, game_id, source, external_id, installed, launch_method, launch_target)
               VALUES('lol-r', 'lol', 'riot', 'lol', 1, 'launcher_cli', 'x');
             INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds) VALUES
               ('avant', 'lol-r', '2026-11-13T20:51:31Z', '2026-11-13T21:05:19Z', 828),
               ('apres', 'lol-r', '2026-11-15T21:38:25Z', '2026-11-15T21:58:29Z', 1204);",
        )
        .unwrap();
        crate::db::migrate(&conn).unwrap();
        assert_eq!(date_legacy_manual_bases(&conn, &backups).unwrap(), 2);
        let at = |id: &str| conn.query_row("SELECT manual_baseline_at FROM games WHERE id = ?1", params![id], |r| r.get::<_, String>(0)).unwrap();
        assert_eq!(at("lol"), "2026-11-15T10:01:57Z", "première sauvegarde qui contient la saisie");
        assert!(at("wow") > "2026-11-17".to_string(), "aucune sauvegarde ne la contient : datée de maintenant");
        let lol = crate::library::list_games(&conn).unwrap().into_iter().find(|g| g.id == "lol").unwrap();
        assert_eq!(lol.playtime_minutes, 138_000 + 1204 / 60, "LoL garde ses 2300 h, plus les sessions jouées depuis la saisie");
        assert_eq!(date_legacy_manual_bases(&conn, &backups).unwrap(), 0, "rien à refaire au démarrage suivant");
    }
}
