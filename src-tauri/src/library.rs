//! LibraryService : fusion des résultats de scan dans la base, dédoublonnage,
//! disponibilité des disques et vue agrégée pour l'interface.
use crate::db::{self, new_id};
use crate::playtime::{game_total_minutes, ManualBase, SourcePart};
use crate::drives::{drive_letter, is_inside, norm_path, with_drive_letter, DriveSnapshot};
use crate::models::{now_iso, unix_to_iso, Achievement, DetectedGame, GameSource, LaunchMethod, OwnedGame, SourceId};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// Titre normalisé : minuscules, sans ponctuation ni symboles de marque.
pub fn normalize_title(title: &str) -> String {
    let mut out = String::new();
    for c in title.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if matches!(c, '™' | '®' | '©' | '\'' | '’') {
            // supprimé sans laisser d'espace
        } else {
            out.push(' ');
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn sort_title(title: &str) -> String {
    let n = normalize_title(title);
    n.strip_prefix("the ").map(|s| s.to_string()).unwrap_or(n)
}

pub fn insert_game(conn: &Connection, title: &str) -> Result<String> {
    let id = new_id();
    conn.execute(
        "INSERT INTO games(id, title, sort_title, added_at) VALUES(?1, ?2, ?3, ?4)",
        params![id, title, sort_title(title), now_iso()],
    )?;
    Ok(id)
}

fn insert_source(conn: &Connection, game_id: &str, d: &DetectedGame, serial: Option<String>) -> Result<String> {
    let id = new_id();
    conn.execute(
        "INSERT INTO game_sources(id, game_id, source, external_id, installed, install_path, install_path_norm, volume_serial,
            executable_path, launch_method, launch_target, launch_args, working_dir, last_played_source, available)
         VALUES(?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 1)",
        params![
            id,
            game_id,
            d.source.as_str(),
            d.external_id,
            d.install_path,
            d.install_path.as_deref().map(norm_path),
            serial,
            d.executable_path,
            d.launch_method.as_str(),
            d.launch_target,
            d.launch_args,
            d.working_dir,
            d.last_played.and_then(unix_to_iso),
        ],
    )?;
    Ok(id)
}

/// Source existante dont le dossier d'installation est le même (ou englobe / est englobé).
fn find_by_path(conn: &Connection, path: &str, exclude_source: SourceId) -> Result<Option<GameSource>> {
    let norm = norm_path(path);
    let exact = conn
        .query_row(
            &format!("SELECT {} FROM game_sources WHERE install_path_norm = ?1 AND source != ?2", db::SOURCE_COLS),
            params![norm, exclude_source.as_str()],
            db::row_to_source,
        )
        .optional()?;
    Ok(exact)
}

#[derive(Debug, Default, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScanStats {
    pub found: usize,
    pub added: usize,
    pub uninstalled: usize,
    pub unavailable: usize,
}

/// Intègre le résultat du scan d'une source (F-DET-02/06, F-LOC-08, F-EXT-03).
pub fn apply_scan(conn: &mut Connection, source: SourceId, detected: &[DetectedGame], drives: &DriveSnapshot) -> Result<ScanStats> {
    let tx = conn.transaction()?;
    let mut stats = ScanStats { found: detected.len(), ..Default::default() };
    let mut seen: HashSet<&str> = HashSet::new();

    for d in detected {
        if !seen.insert(d.external_id.as_str()) {
            continue;
        }
        if upsert_detected(&tx, d, drives)? {
            stats.added += 1;
        }
    }

    // Jeux connus de cette source qui n'ont pas été retrouvés.
    let missing: Vec<GameSource> = {
        let mut stmt = tx.prepare(&format!(
            "SELECT {} FROM game_sources WHERE source = ?1 AND installed = 1",
            db::SOURCE_COLS
        ))?;
        let rows = stmt.query_map(params![source.as_str()], db::row_to_source)?.collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter().filter(|s| !seen.contains(s.external_id.as_str())).collect()
    };
    for s in missing {
        if volume_missing(&s, drives) {
            tx.execute("UPDATE game_sources SET available = 0 WHERE id = ?1", params![s.id])?;
            stats.unavailable += 1;
        } else {
            tx.execute("UPDATE game_sources SET installed = 0, available = 1 WHERE id = ?1", params![s.id])?;
            stats.uninstalled += 1;
        }
    }
    tx.commit()?;
    Ok(stats)
}

/// Ajoute ou met à jour un jeu détecté. Renvoie vrai si un nouveau jeu a été créé.
pub fn upsert_detected(tx: &Connection, d: &DetectedGame, drives: &DriveSnapshot) -> Result<bool> {
    let source = d.source;
    let mut added = false;
    {
        let serial = d.install_path.as_deref().or(d.executable_path.as_deref()).and_then(|p| drives.serial_of_path(p));
        if let Some(existing) = db::find_source(tx, source, &d.external_id)? {
            tx.execute(
                "UPDATE game_sources SET installed = 1, available = 1, install_path = ?2, install_path_norm = ?3,
                    volume_serial = COALESCE(?4, volume_serial), executable_path = ?5, launch_method = ?6, launch_target = ?7,
                    last_played_source = COALESCE(?8, last_played_source)
                 WHERE id = ?1",
                params![
                    existing.id,
                    d.install_path,
                    d.install_path.as_deref().map(norm_path),
                    serial,
                    d.executable_path,
                    d.launch_method.as_str(),
                    d.launch_target,
                    d.last_played.and_then(unix_to_iso),
                ],
            )?;
            if source != SourceId::Local {
                tx.execute(
                    "UPDATE game_sources SET launch_args = ?2, working_dir = ?3 WHERE id = ?1",
                    params![existing.id, d.launch_args, d.working_dir],
                )?;
            }
            return Ok(false);
        }

        let same_path = match d.install_path.as_deref() {
            Some(p) => find_by_path(tx, p, source)?,
            None => None,
        };
        match same_path {
            // Le jeu avait été ajouté à la main : la source « launcher » le remplace (F-LOC-08).
            Some(local) if local.source == SourceId::Local => {
                let new_id = insert_source(tx, &local.game_id, d, serial)?;
                tx.execute("UPDATE play_sessions SET game_source_id = ?1 WHERE game_source_id = ?2", params![new_id, local.id])?;
                tx.execute("DELETE FROM game_sources WHERE id = ?1", params![local.id])?;
            }
            Some(other) => {
                insert_source(tx, &other.game_id, d, serial)?;
            }
            None => {
                let game_id = insert_game(tx, &d.title)?;
                insert_source(tx, &game_id, d, serial)?;
                added = true;
            }
        }
    }
    Ok(added)
}

/// Vrai si le disque qui portait ce jeu n'est pas branché.
fn volume_missing(s: &GameSource, drives: &DriveSnapshot) -> bool {
    let Some(path) = s.install_path.as_deref().or(s.executable_path.as_deref()) else { return false };
    let Some(letter) = drive_letter(path) else { return false };
    match s.volume_serial.as_deref() {
        Some(serial) => drives.letter_for_serial(serial).is_none(),
        None => !drives.has_letter(letter),
    }
}

/// Intègre la liste des jeux possédés d'une source (F-BIB-01, F-HEU-01).
///
/// Le temps de jeu lu chez la source remplace le précédent et la date de lecture est notée : le total affiché
/// est ce temps plus les sessions jouées depuis (voir `list_games`), sans double comptage.
pub fn apply_owned(conn: &mut Connection, source: SourceId, owned: &[OwnedGame]) -> Result<usize> {
    apply_owned_at(conn, source, owned, &now_iso())
}

pub fn apply_owned_at(conn: &mut Connection, source: SourceId, owned: &[OwnedGame], synced_at: &str) -> Result<usize> {
    let tx = conn.transaction()?;
    let mut added = 0;
    for o in owned {
        let last = o.last_played.and_then(unix_to_iso);
        if let Some(existing) = db::find_source(&tx, source, &o.external_id)? {
            tx.execute(
                "UPDATE game_sources SET owned = 1, source_playtime_minutes = COALESCE(?2, source_playtime_minutes),
                    playtime_synced_at = CASE WHEN ?2 IS NOT NULL THEN ?4 ELSE playtime_synced_at END,
                    last_played_source = COALESCE(?3, last_played_source) WHERE id = ?1",
                params![existing.id, o.playtime_minutes, last, synced_at],
            )?;
        } else {
            let game_id = insert_game(&tx, &o.title)?;
            tx.execute(
                "INSERT INTO game_sources(id, game_id, source, external_id, installed, launch_method, launch_target,
                    source_playtime_minutes, last_played_source, available, owned, playtime_synced_at)
                 VALUES(?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8, 1, 1, ?9)",
                params![new_id(), game_id, source.as_str(), o.external_id, o.launch_method.as_str(), o.launch_target, o.playtime_minutes, last, o.playtime_minutes.map(|_| synced_at)],
            )?;
            added += 1;
        }
    }
    tx.commit()?;
    Ok(added)
}

/// Met à jour `available` et retrouve les disques qui ont changé de lettre (F-EXT-02/03).
/// Renvoie le nombre de sources modifiées.
pub fn refresh_availability(conn: &mut Connection, drives: &DriveSnapshot) -> Result<usize> {
    let tx = conn.transaction()?;
    let sources: Vec<GameSource> = db::all_sources(&tx)?.into_iter().filter(|s| s.installed && (s.install_path.is_some() || s.executable_path.is_some())).collect();
    let mut changed = 0;
    for s in sources {
        let anchor = s.install_path.clone().or_else(|| s.executable_path.clone()).unwrap_or_default();
        let Some(letter) = drive_letter(&anchor) else { continue };
        let current = drives.serial_of(letter);
        match s.volume_serial.as_deref() {
            Some(serial) if current == Some(serial) => {
                if !s.available {
                    tx.execute("UPDATE game_sources SET available = 1 WHERE id = ?1", params![s.id])?;
                    changed += 1;
                }
            }
            Some(serial) => match drives.letter_for_serial(serial) {
                Some(new_letter) => {
                    // Même disque, nouvelle lettre : on réécrit les chemins.
                    let fix = |p: &Option<String>| p.as_deref().map(|p| with_drive_letter(p, new_letter));
                    let new_path = fix(&s.install_path);
                    let target = if s.launch_method == LaunchMethod::Exe { with_drive_letter(&s.launch_target, new_letter) } else { s.launch_target.clone() };
                    let workdir = match s.working_dir.as_deref() {
                        Some(w) if drive_letter(w) == Some(letter) => Some(with_drive_letter(w, new_letter)),
                        other => other.map(|w| w.to_string()),
                    };
                    // Les valeurs personnalisées situées sur ce disque suivent la nouvelle lettre.
                    let fix_same = |p: &Option<String>| p.as_deref().map(|p| if drive_letter(p) == Some(letter) { with_drive_letter(p, new_letter) } else { p.to_string() });
                    tx.execute(
                        "UPDATE game_sources SET available = 1, install_path = ?2, install_path_norm = ?3, executable_path = ?4,
                            launch_target = ?5, working_dir = ?6, install_path_override = ?7, executable_path_override = ?8, direct_working_dir = ?9
                         WHERE id = ?1",
                        params![
                            s.id,
                            new_path,
                            new_path.as_deref().map(norm_path),
                            fix(&s.executable_path),
                            target,
                            workdir,
                            fix_same(&s.install_path_override),
                            fix_same(&s.executable_path_override),
                            fix_same(&s.direct_working_dir)
                        ],
                    )?;
                    changed += 1;
                }
                None => {
                    if s.available {
                        tx.execute("UPDATE game_sources SET available = 0 WHERE id = ?1", params![s.id])?;
                        changed += 1;
                    }
                }
            },
            None => {
                let mounted = drives.has_letter(letter);
                if mounted != s.available || (mounted && current.is_some()) {
                    tx.execute(
                        "UPDATE game_sources SET available = ?2, volume_serial = COALESCE(?3, volume_serial) WHERE id = ?1",
                        params![s.id, mounted, current],
                    )?;
                    changed += 1;
                }
            }
        }
    }
    tx.commit()?;
    Ok(changed)
}

/// Les jeux locaux dont l'exécutable a disparu (disque présent) passent en « non installé ».
pub fn check_local_games(conn: &mut Connection, drives: &DriveSnapshot) -> Result<()> {
    let locals: Vec<GameSource> = db::all_sources(conn)?.into_iter().filter(|s| s.source == SourceId::Local).collect();
    for s in locals {
        let Some(exe) = s.effective_exe() else { continue };
        if volume_missing(&s, drives) || !s.available {
            continue;
        }
        let exists = std::path::Path::new(exe).is_file();
        if exists != s.installed {
            conn.execute("UPDATE game_sources SET installed = ?2 WHERE id = ?1", params![s.id, exists])?;
        }
    }
    Ok(())
}

/// Une source (autre que `exclude`) couvre-t-elle déjà ce chemin ? (dédoublonnage du scan local)
pub fn path_already_known(conn: &Connection, path: &str) -> Result<bool> {
    let norm = norm_path(path);
    let mut stmt = conn.prepare("SELECT install_path_norm, executable_path FROM game_sources WHERE install_path_norm IS NOT NULL OR executable_path IS NOT NULL")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?)))?;
    for row in rows {
        let (dir, exe) = row?;
        if let Some(exe) = exe {
            if norm_path(&exe) == norm {
                return Ok(true);
            }
        }
        if let Some(dir) = dir {
            // Racine de disque ou dossier trop générique : on ne compare que l'égalité.
            if dir.len() > 3 && (is_inside(&norm, &dir) || is_inside(&dir, &norm)) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Remplace les succès d'une source (F-SUC-01). `None` = le jeu n'a pas de succès.
pub fn store_achievements(conn: &mut Connection, source_id: &str, achievements: Option<&[Achievement]>) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM achievements WHERE game_source_id = ?1", params![source_id])?;
    for a in achievements.unwrap_or_default() {
        tx.execute(
            "INSERT OR REPLACE INTO achievements(id, game_source_id, api_name, name, description, icon_unlocked, icon_locked, hidden, unlocked, unlocked_at, global_percent)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![new_id(), source_id, a.api_name, a.name, a.description, a.icon_unlocked, a.icon_locked, a.hidden, a.unlocked, a.unlocked_at, a.global_percent],
        )?;
    }
    tx.execute("UPDATE game_sources SET achievements_synced_at = ?2 WHERE id = ?1", params![source_id, now_iso()])?;
    tx.commit()?;
    Ok(())
}

/// Succès d'un jeu, débloqués d'abord (du plus récent au plus ancien), puis par rareté décroissante.
pub fn list_achievements(conn: &Connection, game_id: &str) -> Result<Vec<Achievement>> {
    let mut stmt = conn.prepare(
        "SELECT a.api_name, a.name, a.description, a.icon_unlocked, a.icon_locked, a.hidden, a.unlocked, a.unlocked_at, a.global_percent
         FROM achievements a JOIN game_sources gs ON gs.id = a.game_source_id WHERE gs.game_id = ?1
         ORDER BY a.unlocked DESC, a.unlocked_at DESC, a.global_percent DESC, a.name",
    )?;
    let rows = stmt
        .query_map(params![game_id], |r| {
            Ok(Achievement {
                api_name: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
                icon_unlocked: r.get(3)?,
                icon_locked: r.get(4)?,
                hidden: r.get(5)?,
                unlocked: r.get(6)?,
                unlocked_at: r.get(7)?,
                global_percent: r.get(8)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Vue pour l'interface
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceView {
    #[serde(flatten)]
    pub source: GameSource,
    pub tracked_seconds: i64,
    /// Temps joué depuis la dernière lecture du temps de la source (secondes) ; 0 si la source ne fournit pas de temps.
    pub since_sync_seconds: i64,
    /// Part de cette source dans le temps du jeu (minutes), hors base manuelle : temps de la source + sessions
    /// depuis sa lecture, ou somme des sessions. Le total du jeu est `GameView::playtime_minutes`.
    pub playtime_minutes: i64,
    pub drive: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameView {
    pub id: String,
    pub title: String,
    pub sort_title: String,
    pub cover_path: Option<String>,
    pub hero_path: Option<String>,
    pub logo_path: Option<String>,
    pub description: Option<String>,
    pub genres: Vec<String>,
    pub developer: Option<String>,
    pub release_date: Option<String>,
    pub favorite: bool,
    pub hidden: bool,
    pub added_at: String,
    pub sources: Vec<SourceView>,
    pub installed: bool,
    pub available: bool,
    /// Temps total du jeu en minutes, calculé par `playtime::game_total_minutes`. C'est la seule valeur à
    /// utiliser pour afficher, trier ou additionner le temps d'un jeu.
    pub playtime_minutes: i64,
    /// Temps suivi par l'application, en secondes (affiché à part, F-HEU-05).
    pub tracked_seconds: i64,
    /// Base manuelle saisie par l'utilisateur (minutes) et date de la saisie.
    pub manual_baseline_minutes: Option<i64>,
    pub manual_baseline_at: Option<String>,
    /// Sessions qui commencent après la saisie manuelle (secondes) ; 0 sans base manuelle.
    pub since_manual_seconds: i64,
    /// Part du temps affiché jouée depuis la dernière synchronisation avec la source (secondes).
    pub since_sync_seconds: i64,
    /// Le temps de ce jeu n'est pas suivi (logiciel qui tourne en permanence, par exemple).
    pub no_tracking: bool,
    /// Statut dans le backlog : `backlog`, `playing`, `finished`, `completed`, `abandoned`, `paused` (O-02).
    pub status: Option<String>,
    pub status_changed_at: Option<String>,
    /// Note personnelle sur 10 (O-03).
    pub rating: Option<i64>,
    pub notes: Option<String>,
    /// Durée de vie (minutes) : histoire principale et 100 %, d'après IGDB (V1.5).
    pub time_to_beat_main: Option<i64>,
    pub time_to_beat_full: Option<i64>,
    pub last_played: Option<String>,
    pub last_session_seconds: Option<i64>,
    pub ach_total: i64,
    pub ach_unlocked: i64,
    pub collections: Vec<String>,
}

pub fn list_games(conn: &Connection) -> Result<Vec<GameView>> {
    // Sessions agrégées par source.
    // Dernière colonne : temps joué après la dernière lecture du temps de la source (une session à cheval
    // sur cette date ne compte que pour sa partie postérieure).
    let mut tracked: HashMap<String, (i64, Option<String>, i64)> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT ps.game_source_id, SUM(ps.duration_seconds), MAX(ps.started_at),
                    SUM(CASE WHEN gs.playtime_synced_at IS NULL OR ps.ended_at <= gs.playtime_synced_at THEN 0
                             WHEN ps.started_at >= gs.playtime_synced_at THEN ps.duration_seconds
                             ELSE MIN(ps.duration_seconds, CAST(strftime('%s', ps.ended_at) AS INTEGER) - CAST(strftime('%s', gs.playtime_synced_at) AS INTEGER)) END)
             FROM play_sessions ps JOIN game_sources gs ON gs.id = ps.game_source_id GROUP BY ps.game_source_id",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, i64>(3)?)))?;
        for row in rows {
            let (id, total, last, since) = row?;
            tracked.insert(id, (total, last, since.max(0)));
        }
    }
    // Dernière session par jeu.
    let mut last_session: HashMap<String, i64> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT gs.game_id, ps.duration_seconds FROM play_sessions ps JOIN game_sources gs ON gs.id = ps.game_source_id
             ORDER BY ps.started_at ASC",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for row in rows {
            let (game, dur) = row?;
            last_session.insert(game, dur);
        }
    }
    let mut ach: HashMap<String, (i64, i64)> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT gs.game_id, COUNT(*), SUM(a.unlocked) FROM achievements a JOIN game_sources gs ON gs.id = a.game_source_id GROUP BY gs.game_id",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)))?;
        for row in rows {
            let (game, total, unlocked) = row?;
            ach.insert(game, (total, unlocked));
        }
    }
    let mut colls: HashMap<String, Vec<String>> = HashMap::new();
    {
        let mut stmt = conn.prepare("SELECT game_id, collection_id FROM collection_games")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (game, coll) = row?;
            colls.entry(game).or_default().push(coll);
        }
    }
    // Sessions postérieures à la base manuelle de chaque jeu.
    let mut since_manual: HashMap<String, i64> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT gs.game_id, SUM(ps.duration_seconds) FROM play_sessions ps
             JOIN game_sources gs ON gs.id = ps.game_source_id JOIN games g ON g.id = gs.game_id
             WHERE g.manual_baseline_at IS NOT NULL AND ps.started_at >= g.manual_baseline_at GROUP BY gs.game_id",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for row in rows {
            let (game, secs) = row?;
            since_manual.insert(game, secs);
        }
    }
    let mut by_game: HashMap<String, Vec<GameSource>> = HashMap::new();
    for s in db::all_sources(conn)? {
        by_game.entry(s.game_id.clone()).or_default().push(s);
    }

    let mut stmt = conn.prepare(
        "SELECT id, title, sort_title, cover_path, hero_path, logo_path, description, genres, developer, release_date,
                favorite, hidden, added_at, manual_baseline_minutes, no_tracking, manual_baseline_at, status, status_changed_at, rating, notes, time_to_beat_main, time_to_beat_full FROM games",
    )?;
    let mut games = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(r) = rows.next()? {
        let id: String = r.get(0)?;
        let sources = by_game.remove(&id).unwrap_or_default();
        if sources.is_empty() {
            continue;
        }
        let manual_minutes: Option<i64> = r.get(13)?;
        let manual_at: Option<String> = r.get(15)?;
        let manual = manual_minutes.map(|minutes| ManualBase { minutes, since_seconds: since_manual.get(&id).copied().unwrap_or(0) });
        let mut parts: Vec<SourcePart> = Vec::new();
        let mut tracked_total = 0i64;
        let mut since_sync_total = 0i64;
        let mut last_played: Option<String> = None;
        let mut views = Vec::new();
        for s in sources {
            let (secs, last_tracked, since) = tracked.get(&s.id).cloned().unwrap_or((0, None, 0));
            tracked_total += secs;
            let part = SourcePart { source_minutes: s.source_playtime_minutes, session_seconds: secs, since_sync_seconds: since };
            let since = if s.source_playtime_minutes.is_some() { since } else { 0 };
            let source_minutes = part.minutes();
            parts.push(part);
            since_sync_total += since;
            for candidate in [s.last_played_source.clone(), last_tracked].into_iter().flatten() {
                if last_played.as_deref().map_or(true, |cur| candidate.as_str() > cur) {
                    last_played = Some(candidate);
                }
            }
            let drive = s.effective_dir().and_then(drive_letter).map(|l| format!("{l}:"));
            views.push(SourceView { source: s, tracked_seconds: secs, since_sync_seconds: since, playtime_minutes: source_minutes, drive });
        }
        // Source installée et disponible en premier (c'est elle qu'on lance).
        views.sort_by_key(|v| (!(v.source.installed && v.source.available), !v.source.installed));
        let genres: Option<String> = r.get(7)?;
        let (ach_total, ach_unlocked) = ach.get(&id).cloned().unwrap_or((0, 0));
        games.push(GameView {
            title: r.get(1)?,
            sort_title: r.get(2)?,
            cover_path: r.get(3)?,
            hero_path: r.get(4)?,
            logo_path: r.get(5)?,
            description: r.get(6)?,
            genres: genres.map(|g| g.split(", ").filter(|x| !x.is_empty()).map(|x| x.to_string()).collect()).unwrap_or_default(),
            developer: r.get(8)?,
            release_date: r.get(9)?,
            favorite: r.get(10)?,
            hidden: r.get(11)?,
            added_at: r.get(12)?,
            installed: views.iter().any(|v| v.source.installed),
            available: views.iter().any(|v| v.source.installed && v.source.available) || !views.iter().any(|v| v.source.installed),
            playtime_minutes: game_total_minutes(&parts, manual),
            tracked_seconds: tracked_total,
            manual_baseline_minutes: manual_minutes,
            manual_baseline_at: manual_at,
            since_manual_seconds: manual.map_or(0, |m| m.since_seconds),
            since_sync_seconds: if manual.is_some() { 0 } else { since_sync_total },
            no_tracking: r.get(14)?,
            status: r.get(16)?,
            status_changed_at: r.get(17)?,
            rating: r.get(18)?,
            notes: r.get(19)?,
            time_to_beat_main: r.get(20)?,
            time_to_beat_full: r.get(21)?,
            last_played,
            last_session_seconds: last_session.get(&id).copied(),
            ach_total,
            ach_unlocked,
            collections: colls.remove(&id).unwrap_or_default(),
            sources: views,
            id,
        });
    }
    games.sort_by(|a, b| a.sort_title.cmp(&b.sort_title));
    Ok(games)
}

/// Derniers jeux joués (installés, visibles), du plus récent au plus ancien, avec leur temps total.
pub fn recent_games(conn: &Connection, disabled: &HashSet<String>, limit: usize) -> Result<Vec<GameView>> {
    let mut games: Vec<GameView> = list_visible_games(conn, disabled)?.into_iter().filter(|g| g.installed && !g.hidden && g.last_played.is_some()).collect();
    games.sort_by(|a, b| b.last_played.cmp(&a.last_played));
    games.truncate(limit);
    Ok(games)
}

/// Bibliothèque sans les sources désactivées par l'utilisateur (F-PAR-02).
pub fn list_visible_games(conn: &Connection, disabled: &HashSet<String>) -> Result<Vec<GameView>> {
    let mut games = list_games(conn)?;
    if disabled.is_empty() {
        return Ok(games);
    }
    for g in &mut games {
        g.sources.retain(|s| !disabled.contains(s.source.source.as_str()));
        g.installed = g.sources.iter().any(|v| v.source.installed);
        g.available = g.sources.iter().any(|v| v.source.installed && v.source.available) || !g.installed;
    }
    games.retain(|g| !g.sources.is_empty());
    Ok(games)
}

/// Paires de jeux au même titre normalisé sur des sources différentes (§5.2 : proposer de fusionner).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MergeSuggestion {
    pub title: String,
    pub game_ids: Vec<String>,
}

/// Ne plus proposer la fusion des jeux portant ce titre.
pub fn dismiss_merge(conn: &Connection, title: &str) -> Result<()> {
    let mut list: Vec<String> = db::get_setting(conn, "merge_dismissed").and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default();
    let key = normalize_title(title);
    if !list.contains(&key) {
        list.push(key);
    }
    db::set_setting(conn, "merge_dismissed", &serde_json::to_string(&list)?)
}

pub fn merge_suggestions(conn: &Connection) -> Result<Vec<MergeSuggestion>> {
    let mut stmt = conn.prepare("SELECT id, title FROM games WHERE hidden = 0 ORDER BY added_at")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut groups: HashMap<String, (String, Vec<String>)> = HashMap::new();
    for row in rows {
        let (id, title) = row?;
        let key = normalize_title(&title);
        if key.is_empty() {
            continue;
        }
        groups.entry(key).or_insert_with(|| (title, Vec::new())).1.push(id);
    }
    let dismissed: HashSet<String> = db::get_setting(conn, "merge_dismissed")
        .and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok())
        .unwrap_or_default()
        .into_iter()
        .collect();
    let mut out: Vec<MergeSuggestion> = groups
        .into_iter()
        .filter(|(key, (_, ids))| ids.len() > 1 && !dismissed.contains(key))
        .map(|(_, (title, game_ids))| MergeSuggestion { title, game_ids })
        .collect();
    out.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(out)
}

/// Fusionne `others` dans `keep` : les sources, collections et préférences sont regroupées.
pub fn merge_games(conn: &mut Connection, keep: &str, others: &[String]) -> Result<()> {
    let tx = conn.transaction()?;
    for other in others {
        if other == keep {
            continue;
        }
        tx.execute("UPDATE game_sources SET game_id = ?1 WHERE game_id = ?2", params![keep, other])?;
        tx.execute("UPDATE OR IGNORE collection_games SET game_id = ?1 WHERE game_id = ?2", params![keep, other])?;
        tx.execute(
            "UPDATE games SET favorite = MAX(favorite, (SELECT favorite FROM games WHERE id = ?2)),
                cover_path = COALESCE(cover_path, (SELECT cover_path FROM games WHERE id = ?2)),
                hero_path = COALESCE(hero_path, (SELECT hero_path FROM games WHERE id = ?2)),
                logo_path = COALESCE(logo_path, (SELECT logo_path FROM games WHERE id = ?2)),
                description = COALESCE(description, (SELECT description FROM games WHERE id = ?2)),
                genres = COALESCE(genres, (SELECT genres FROM games WHERE id = ?2)),
                developer = COALESCE(developer, (SELECT developer FROM games WHERE id = ?2)),
                release_date = COALESCE(release_date, (SELECT release_date FROM games WHERE id = ?2))
             WHERE id = ?1",
            params![keep, other],
        )?;
        tx.execute("DELETE FROM games WHERE id = ?1", params![other])?;
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drives::DriveInfo;

    fn drives(list: &[(char, &str)]) -> DriveSnapshot {
        DriveSnapshot { drives: list.iter().map(|(l, s)| DriveInfo { letter: *l, serial: Some(s.to_string()), removable: false }).collect() }
    }

    fn steam_game(id: &str, title: &str, path: &str) -> DetectedGame {
        DetectedGame {
            source: SourceId::Steam,
            external_id: id.into(),
            title: title.into(),
            install_path: Some(path.into()),
            executable_path: None,
            launch_method: LaunchMethod::Uri,
            launch_target: format!("steam://rungameid/{id}"),
            launch_args: None,
            working_dir: None,
            last_played: None,
        }
    }

    fn local_game(path: &str, exe: &str, title: &str) -> DetectedGame {
        DetectedGame {
            source: SourceId::Local,
            external_id: norm_path(exe),
            title: title.into(),
            install_path: Some(path.into()),
            executable_path: Some(exe.into()),
            launch_method: LaunchMethod::Exe,
            launch_target: exe.into(),
            launch_args: None,
            working_dir: None,
            last_played: None,
        }
    }

    #[test]
    fn titles_are_normalized() {
        assert_eq!(normalize_title("Tom Clancy's Rainbow Six® Siege"), "tom clancys rainbow six siege");
        assert_eq!(normalize_title("  DOOM: Eternal "), "doom eternal");
        assert_eq!(sort_title("The Witcher 3"), "witcher 3");
    }

    #[test]
    fn scan_adds_updates_and_marks_uninstalled() {
        let mut conn = db::open_in_memory().unwrap();
        let d = drives(&[('C', "AAAA"), ('D', "BBBB")]);
        let games = vec![steam_game("10", "Alpha", "C:\\Steam\\common\\Alpha"), steam_game("20", "Beta", "D:\\Lib\\common\\Beta")];
        let s = apply_scan(&mut conn, SourceId::Steam, &games, &d).unwrap();
        assert_eq!((s.found, s.added), (2, 2));
        // Deuxième scan identique : rien de nouveau.
        let s = apply_scan(&mut conn, SourceId::Steam, &games, &d).unwrap();
        assert_eq!(s.added, 0);
        assert_eq!(list_games(&conn).unwrap().len(), 2);
        // Beta désinstallé, disque présent : reste dans l'historique (F-DET-06).
        let s = apply_scan(&mut conn, SourceId::Steam, &games[..1], &d).unwrap();
        assert_eq!(s.uninstalled, 1);
        let list = list_games(&conn).unwrap();
        let beta = list.iter().find(|g| g.title == "Beta").unwrap();
        assert!(!beta.installed);
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn unplugged_drive_greys_games_without_removing(){
        let mut conn = db::open_in_memory().unwrap();
        let both = drives(&[('C', "AAAA"), ('E', "EXT1")]);
        let games = vec![steam_game("10", "Alpha", "C:\\Steam\\common\\Alpha"), steam_game("20", "Beta", "E:\\Lib\\common\\Beta")];
        apply_scan(&mut conn, SourceId::Steam, &games, &both).unwrap();
        // Disque E: débranché : le scan ne trouve plus Beta.
        let only_c = drives(&[('C', "AAAA")]);
        refresh_availability(&mut conn, &only_c).unwrap();
        let s = apply_scan(&mut conn, SourceId::Steam, &games[..1], &only_c).unwrap();
        assert_eq!((s.unavailable, s.uninstalled), (1, 0));
        let beta = list_games(&conn).unwrap().into_iter().find(|g| g.title == "Beta").unwrap();
        assert!(beta.installed && !beta.available);
        // Rebranché sous une autre lettre : les chemins suivent le numéro de série (F-EXT-02).
        let moved = drives(&[('C', "AAAA"), ('F', "EXT1")]);
        refresh_availability(&mut conn, &moved).unwrap();
        let beta = list_games(&conn).unwrap().into_iter().find(|g| g.title == "Beta").unwrap();
        assert!(beta.available);
        assert_eq!(beta.sources[0].source.install_path.as_deref(), Some("F:\\Lib\\common\\Beta"));
    }

    #[test]
    fn local_exe_follows_drive_letter_change() {
        let mut conn = db::open_in_memory().unwrap();
        let d = drives(&[('C', "AAAA"), ('E', "EXT1")]);
        apply_scan(&mut conn, SourceId::Local, &[local_game("E:\\Jeux\\Foo", "E:\\Jeux\\Foo\\foo.exe", "Foo")], &d).unwrap();
        refresh_availability(&mut conn, &drives(&[('C', "AAAA")])).unwrap();
        assert!(!list_games(&conn).unwrap()[0].available);
        refresh_availability(&mut conn, &drives(&[('C', "AAAA"), ('G', "EXT1")])).unwrap();
        let g = &list_games(&conn).unwrap()[0];
        assert!(g.available);
        assert_eq!(g.sources[0].source.launch_target, "G:\\Jeux\\Foo\\foo.exe");
        assert_eq!(g.sources[0].source.executable_path.as_deref(), Some("G:\\Jeux\\Foo\\foo.exe"));
    }

    #[test]
    fn letter_reused_by_another_disk_is_not_confused() {
        let mut conn = db::open_in_memory().unwrap();
        apply_scan(&mut conn, SourceId::Steam, &[steam_game("20", "Beta", "E:\\Lib\\common\\Beta")], &drives(&[('E', "EXT1")])).unwrap();
        // Un autre disque prend la lettre E:.
        refresh_availability(&mut conn, &drives(&[('E', "OTHER")])).unwrap();
        assert!(!list_games(&conn).unwrap()[0].available);
    }

    #[test]
    fn launcher_scan_replaces_local_entry_with_same_path() {
        let mut conn = db::open_in_memory().unwrap();
        let d = drives(&[('C', "AAAA")]);
        apply_scan(&mut conn, SourceId::Local, &[local_game("C:\\Steam\\common\\Alpha", "C:\\Steam\\common\\Alpha\\alpha.exe", "Alpha")], &d).unwrap();
        let local_id = list_games(&conn).unwrap()[0].sources[0].source.id.clone();
        conn.execute(
            "INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds) VALUES('s1', ?1, '2026-01-01T10:00:00Z', '2026-01-01T11:00:00Z', 3600)",
            params![local_id],
        )
        .unwrap();
        apply_scan(&mut conn, SourceId::Steam, &[steam_game("10", "Alpha", "C:/Steam/common/Alpha/")], &d).unwrap();
        let list = list_games(&conn).unwrap();
        assert_eq!(list.len(), 1, "le jeu ne doit apparaître qu'une fois (F-LOC-08)");
        assert_eq!(list[0].sources.len(), 1);
        assert_eq!(list[0].sources[0].source.source, SourceId::Steam);
        assert_eq!(list[0].tracked_seconds, 3600, "les sessions sont conservées");
        assert!(path_already_known(&conn, "C:\\Steam\\common\\Alpha\\alpha.exe").unwrap());
        assert!(!path_already_known(&conn, "C:\\Steam\\common\\AlphaTwo\\a.exe").unwrap());
    }

    /// Jeu Riot (sans temps de la source) avec une base manuelle saisie le 15/11 à 10 h.
    fn riot_game_with_manual_base(conn: &Connection) -> (String, String) {
        conn.execute_batch(
            "INSERT INTO games(id, title, sort_title, added_at, manual_baseline_minutes, manual_baseline_at)
               VALUES('lol', 'League of Legends', 'league of legends', '2026-11-01T00:00:00Z', 138000, '2026-11-15T10:00:00Z');
             INSERT INTO game_sources(id, game_id, source, external_id, installed, launch_method, launch_target)
               VALUES('lol-riot', 'lol', 'riot', 'league_of_legends.live', 1, 'launcher_cli', 'x');",
        )
        .unwrap();
        ("lol".into(), "lol-riot".into())
    }

    fn add_session(conn: &Connection, id: &str, source: &str, start: &str, end: &str, secs: i64) {
        conn.execute(
            "INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds, origin) VALUES(?1, ?2, ?3, ?4, ?5, 'detected')",
            params![id, source, start, end, secs],
        )
        .unwrap();
    }

    #[test]
    fn manual_base_plus_one_hour_after_is_2301_hours() {
        let conn = db::open_in_memory().unwrap();
        let (_, source) = riot_game_with_manual_base(&conn);
        add_session(&conn, "apres", &source, "2026-11-16T20:00:00Z", "2026-11-16T21:00:00Z", 3600);
        let g = &list_games(&conn).unwrap()[0];
        assert_eq!(g.playtime_minutes, 2301 * 60);
        assert_eq!(g.since_manual_seconds, 3600);
        assert_eq!(g.manual_baseline_at.as_deref(), Some("2026-11-15T10:00:00Z"));
    }

    #[test]
    fn session_before_the_manual_entry_is_not_counted() {
        let conn = db::open_in_memory().unwrap();
        let (_, source) = riot_game_with_manual_base(&conn);
        add_session(&conn, "avant", &source, "2026-11-13T20:00:00Z", "2026-11-13T22:00:00Z", 7200);
        let g = &list_games(&conn).unwrap()[0];
        assert_eq!(g.playtime_minutes, 2300 * 60, "la session antérieure est déjà comprise dans le chiffre saisi");
        assert_eq!(g.tracked_seconds, 7200, "elle reste dans l'historique");
    }

    #[test]
    fn new_manual_entry_replaces_the_base_and_restarts_the_clock() {
        let conn = db::open_in_memory().unwrap();
        let (game, source) = riot_game_with_manual_base(&conn);
        add_session(&conn, "s1", &source, "2026-11-16T20:00:00Z", "2026-11-16T21:00:00Z", 3600);
        crate::playtime::set_manual_base(&conn, &game, Some(100 * 60)).unwrap();
        let g = &list_games(&conn).unwrap()[0];
        assert_eq!(g.playtime_minutes, 100 * 60, "nouvelle base, les sessions passées ne s'y ajoutent pas");
        crate::playtime::set_manual_base(&conn, &game, None).unwrap();
        assert_eq!(list_games(&conn).unwrap()[0].playtime_minutes, 60, "sans base : somme de toutes les sessions");
    }

    #[test]
    fn owned_games_and_playtime_rule() {
        let mut conn = db::open_in_memory().unwrap();
        let d = drives(&[('C', "AAAA")]);
        apply_scan(&mut conn, SourceId::Steam, &[steam_game("10", "Alpha", "C:\\Steam\\common\\Alpha")], &d).unwrap();
        let owned = vec![
            OwnedGame { external_id: "10".into(), title: "Alpha".into(), playtime_minutes: Some(120), last_played: Some(1_700_000_000), launch_method: LaunchMethod::Uri, launch_target: "steam://rungameid/10".into() },
            OwnedGame { external_id: "30".into(), title: "Gamma".into(), playtime_minutes: Some(5), last_played: None, launch_method: LaunchMethod::Uri, launch_target: "steam://rungameid/30".into() },
        ];
        assert_eq!(apply_owned(&mut conn, SourceId::Steam, &owned).unwrap(), 1);
        let sid = list_games(&conn).unwrap().into_iter().find(|g| g.title == "Alpha").unwrap().sources[0].source.id.clone();
        conn.execute(
            "INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds) VALUES('s1', ?1, '2026-01-01T10:00:00Z', '2026-01-01T10:30:00Z', 1800)",
            params![sid],
        )
        .unwrap();
        let list = list_games(&conn).unwrap();
        let alpha = list.iter().find(|g| g.title == "Alpha").unwrap();
        // Temps de la source affiché, temps suivi à part : pas de double comptage (F-HEU-05).
        assert_eq!(alpha.playtime_minutes, 120);
        assert_eq!(alpha.tracked_seconds, 1800);
        assert_eq!(alpha.last_played.as_deref(), Some("2026-01-01T10:00:00Z"));
        let gamma = list.iter().find(|g| g.title == "Gamma").unwrap();
        assert!(!gamma.installed && gamma.available);
        assert_eq!(gamma.playtime_minutes, 5);
        // Une partie jouée APRÈS l'import s'ajoute au temps de la source, sans attendre la synchronisation…
        apply_owned_at(&mut conn, SourceId::Steam, &owned, "2026-02-01T00:00:00Z").unwrap();
        conn.execute(
            "INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds) VALUES
               ('s2', ?1, '2026-02-02T10:00:00Z', '2026-02-02T11:00:00Z', 3600),
               ('s3', ?1, '2026-01-31T23:30:00Z', '2026-02-01T00:15:00Z', 2700)",
            params![sid],
        )
        .unwrap();
        let alpha = list_games(&conn).unwrap().into_iter().find(|g| g.title == "Alpha").unwrap();
        assert_eq!(alpha.playtime_minutes, 120 + 60 + 15, "temps Steam + session d'après + partie postérieure de la session à cheval");
        assert_eq!(alpha.since_sync_seconds, 4500);
        // … puis une synchronisation remplace le tout par le chiffre de Steam : rien n'est compté deux fois
        // et l'historique des sessions est intact.
        let mut synced = owned.clone();
        synced[0].playtime_minutes = Some(200);
        apply_owned_at(&mut conn, SourceId::Steam, &synced, "2026-02-03T00:00:00Z").unwrap();
        let alpha = list_games(&conn).unwrap().into_iter().find(|g| g.title == "Alpha").unwrap();
        assert_eq!((alpha.playtime_minutes, alpha.since_sync_seconds), (200, 0));
        assert_eq!(alpha.tracked_seconds, 1800 + 3600 + 2700, "les sessions sont conservées");
        // Un scan ultérieur ne supprime pas le jeu possédé non installé.
        apply_scan(&mut conn, SourceId::Steam, &[steam_game("10", "Alpha", "C:\\Steam\\common\\Alpha")], &d).unwrap();
        assert_eq!(list_games(&conn).unwrap().len(), 2);
    }

    #[test]
    fn achievements_are_stored_counted_and_replaced() {
        let mut conn = db::open_in_memory().unwrap();
        let d = drives(&[('C', "AAAA")]);
        apply_scan(&mut conn, SourceId::Steam, &[steam_game("10", "Alpha", r"C:\Steam\common\Alpha")], &d).unwrap();
        let game = list_games(&conn).unwrap().remove(0);
        assert_eq!((game.ach_total, game.ach_unlocked), (0, 0), "aucun succès par défaut");
        let ach = |name: &str, unlocked: bool| Achievement {
            api_name: name.into(),
            name: name.to_lowercase(),
            description: String::new(),
            icon_unlocked: None,
            icon_locked: None,
            hidden: false,
            unlocked,
            unlocked_at: unlocked.then(|| "2026-02-01T00:00:00Z".to_string()),
            global_percent: Some(50.0),
        };
        let sid = game.sources[0].source.id.clone();
        store_achievements(&mut conn, &sid, Some(&[ach("A", true), ach("B", false), ach("C", false)])).unwrap();
        let game = list_games(&conn).unwrap().remove(0);
        assert_eq!((game.ach_total, game.ach_unlocked), (3, 1));
        let listed = list_achievements(&conn, &game.id).unwrap();
        assert_eq!(listed[0].api_name, "A", "débloqués en premier");
        // Nouvelle synchronisation : remplacement complet, pas de doublons.
        store_achievements(&mut conn, &sid, Some(&[ach("A", true), ach("B", true)])).unwrap();
        let game = list_games(&conn).unwrap().remove(0);
        assert_eq!((game.ach_total, game.ach_unlocked), (2, 2));
        store_achievements(&mut conn, &sid, None).unwrap();
        assert!(list_achievements(&conn, &game.id).unwrap().is_empty());
    }

    #[test]
    fn handles_a_thousand_games_quickly() {
        let mut conn = db::open_in_memory().unwrap();
        let d = drives(&[('C', "AAAA")]);
        let games: Vec<DetectedGame> = (0..1000).map(|i| steam_game(&i.to_string(), &format!("Jeu {i}"), &format!("C:\\Steam\\common\\Jeu{i}"))).collect();
        let started = std::time::Instant::now();
        apply_scan(&mut conn, SourceId::Steam, &games, &d).unwrap();
        apply_scan(&mut conn, SourceId::Steam, &games, &d).unwrap();
        refresh_availability(&mut conn, &d).unwrap();
        let listed = list_games(&conn).unwrap();
        assert_eq!(listed.len(), 1000);
        assert!(started.elapsed().as_secs() < 5, "1 000 jeux intégrés et relus en {:?}", started.elapsed());
    }

    #[test]
    fn disabled_sources_are_hidden() {
        let mut conn = db::open_in_memory().unwrap();
        let d = drives(&[('C', "AAAA")]);
        apply_scan(&mut conn, SourceId::Steam, &[steam_game("10", "Alpha", "C:\\Steam\\common\\Alpha")], &d).unwrap();
        apply_scan(&mut conn, SourceId::Local, &[local_game("C:\\Jeux\\Foo", "C:\\Jeux\\Foo\\foo.exe", "Foo")], &d).unwrap();
        let disabled: HashSet<String> = ["steam".to_string()].into_iter().collect();
        let visible = list_visible_games(&conn, &disabled).unwrap();
        assert_eq!(visible.iter().map(|g| g.title.as_str()).collect::<Vec<_>>(), vec!["Foo"]);
        assert_eq!(list_visible_games(&conn, &HashSet::new()).unwrap().len(), 2);
    }

    #[test]
    fn merge_suggestions_and_merge() {
        let mut conn = db::open_in_memory().unwrap();
        let d = drives(&[('C', "AAAA")]);
        apply_scan(&mut conn, SourceId::Steam, &[steam_game("10", "Rocket League®", "C:\\Steam\\common\\rl")], &d).unwrap();
        let mut epic = steam_game("Sugar", "Rocket League", "C:\\Epic\\rocketleague");
        epic.source = SourceId::Epic;
        apply_scan(&mut conn, SourceId::Epic, &[epic], &d).unwrap();
        let sugg = merge_suggestions(&conn).unwrap();
        assert_eq!(sugg.len(), 1);
        merge_games(&mut conn, &sugg[0].game_ids[0], &sugg[0].game_ids[1..]).unwrap();
        let list = list_games(&conn).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].sources.len(), 2);
        assert!(merge_suggestions(&conn).unwrap().is_empty());
    }
}
