//! Commandes Tauri appelées par l'interface.
use crate::drives::{norm_path, DriveSnapshot};
use crate::error::{AppError, AppResult};
use crate::library::{self, GameView};
use crate::models::{now_iso, LaunchMethod, SourceId};
use crate::providers::local::{self, Candidate};
use crate::state::{Emitter, Shared};
use crate::{db, providers, sync};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{AppHandle, Emitter as _, State};

pub fn emitter(app: &AppHandle) -> Emitter {
    let app = app.clone();
    std::sync::Arc::new(move |event: &str, payload: serde_json::Value| {
        let _ = app.emit(event, payload);
    })
}

fn notify_library(app: &AppHandle) {
    let _ = app.emit("library-updated", serde_json::Value::Null);
}

// ---------------------------------------------------------------------------
// Bibliothèque et détection
// ---------------------------------------------------------------------------

#[tauri::command(async)]
pub fn get_games(state: State<'_, Shared>) -> AppResult<Vec<GameView>> {
    Ok(library::list_games(&state.conn())?)
}

/// Relance la détection en arrière-plan (F-DET-04).
#[tauri::command]
pub fn refresh_library(app: AppHandle, state: State<'_, Shared>) {
    let state = state.inner().clone();
    let emit = emitter(&app);
    std::thread::spawn(move || {
        sync::run_scan(&state, &emit);
    });
}

#[tauri::command]
pub fn is_scanning(state: State<'_, Shared>) -> bool {
    state.scanning.load(std::sync::atomic::Ordering::SeqCst)
}

#[tauri::command]
pub fn get_running(state: State<'_, Shared>) -> Vec<String> {
    state.tracker.running_games()
}

// ---------------------------------------------------------------------------
// Lancement
// ---------------------------------------------------------------------------

/// Lance un jeu (F-LAN-01) avec des erreurs explicites (F-LAN-02), puis démarre le suivi (F-HEU-02).
#[tauri::command(async)]
pub fn launch_game(app: AppHandle, state: State<'_, Shared>, game_id: String, source_id: Option<String>) -> AppResult<()> {
    let sources = db::sources_of_game(&state.conn(), &game_id)?;
    let chosen = match source_id {
        Some(id) => sources.iter().find(|s| s.id == id),
        None => sources.iter().find(|s| s.installed && s.available).or_else(|| sources.iter().find(|s| s.installed)).or(sources.first()),
    }
    .ok_or_else(|| AppError::new("GAME_NOT_FOUND", game_id.clone()))?
    .clone();
    if !chosen.installed {
        return Err(AppError::new("NOT_INSTALLED", ""));
    }
    if !chosen.available {
        return Err(AppError::new("DISK_UNPLUGGED", chosen.install_path.clone().unwrap_or_default()));
    }
    let cfg = state.provider_config();
    match providers::by_id(&cfg, chosen.source) {
        Some(provider) => {
            if chosen.source != SourceId::Local && !provider.is_available() {
                return Err(AppError::new("LAUNCHER_MISSING", chosen.source.as_str()));
            }
            provider.launch(&chosen)?;
        }
        None => crate::launch::launch_source(&chosen)?,
    }
    log::info!("lancement de {} ({})", chosen.external_id, chosen.source.as_str());
    state.tracker.start(&state, &emitter(&app), &chosen, chosen.launch_method != LaunchMethod::Exe);
    Ok(())
}

/// Ouvre le launcher d'origine sur l'installation du jeu (F-BIB-03).
#[tauri::command(async)]
pub fn install_game(state: State<'_, Shared>, game_id: String) -> AppResult<()> {
    let sources = db::sources_of_game(&state.conn(), &game_id)?;
    let cfg = state.provider_config();
    for s in &sources {
        if let Some(uri) = providers::by_id(&cfg, s.source).and_then(|p| p.install_uri(s)) {
            crate::launch::shell_open(&uri, None, None)?;
            return Ok(());
        }
    }
    Err(AppError::new("INSTALL_UNSUPPORTED", ""))
}

/// Ouvre un dossier dans l'Explorateur.
#[tauri::command(async)]
pub fn open_path(path: String) -> AppResult<()> {
    if !Path::new(&path).exists() {
        return Err(AppError::new("FILE_NOT_FOUND", path));
    }
    crate::launch::shell_open(&path, None, None)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Jeux locaux et dossiers surveillés
// ---------------------------------------------------------------------------

fn watched_paths(conn: &rusqlite::Connection) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT path FROM watched_folders ORDER BY path")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn add_local(conn: &mut rusqlite::Connection, exe: &str, title: Option<&str>, folder: Option<&str>, args: Option<&str>, working_dir: Option<&str>) -> AppResult<()> {
    let path = Path::new(exe);
    let is_exe = path.extension().map(|e| e.eq_ignore_ascii_case("exe")).unwrap_or(false);
    if !is_exe {
        return Err(AppError::new("NOT_AN_EXE", exe));
    }
    if !path.is_file() {
        return Err(AppError::new("FILE_NOT_FOUND", exe));
    }
    if library::path_already_known(conn, exe)? {
        return Err(AppError::new("ALREADY_IN_LIBRARY", exe));
    }
    let watched = watched_paths(conn)?;
    let detected = local::detected_from_exe(exe, title, folder, args, working_dir, &watched);
    let tx = conn.transaction()?;
    library::upsert_detected(&tx, &detected, &DriveSnapshot::current())?;
    tx.execute("DELETE FROM ignored_candidates WHERE path_norm = ?1", params![norm_path(exe)])?;
    tx.commit()?;
    log::info!("jeu local ajouté : {}", detected.title);
    Ok(())
}

/// Ajout manuel d'un jeu par son exécutable (F-LOC-01, F-LOC-05).
#[tauri::command(async)]
pub fn add_local_game(app: AppHandle, state: State<'_, Shared>, exe_path: String, title: Option<String>, args: Option<String>, working_dir: Option<String>) -> AppResult<()> {
    add_local(&mut state.conn(), &exe_path, title.as_deref(), None, args.as_deref(), working_dir.as_deref())?;
    notify_library(&app);
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchedFolder {
    pub id: String,
    pub path: String,
    pub exists: bool,
}

#[tauri::command(async)]
pub fn get_watched_folders(state: State<'_, Shared>) -> AppResult<Vec<WatchedFolder>> {
    let conn = state.conn();
    let mut stmt = conn.prepare("SELECT id, path FROM watched_folders ORDER BY path")?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows.into_iter().map(|(id, path)| WatchedFolder { exists: Path::new(&path).is_dir(), id, path }).collect())
}

#[tauri::command(async)]
pub fn add_watched_folder(state: State<'_, Shared>, path: String) -> AppResult<()> {
    if !Path::new(&path).is_dir() {
        return Err(AppError::new("FOLDER_NOT_FOUND", path));
    }
    let clean = crate::drives::clean_path(&path);
    state.conn().execute("INSERT OR IGNORE INTO watched_folders(id, path, added_at) VALUES(?1, ?2, ?3)", params![db::new_id(), clean, now_iso()])?;
    Ok(())
}

#[tauri::command(async)]
pub fn remove_watched_folder(state: State<'_, Shared>, id: String) -> AppResult<()> {
    state.conn().execute("DELETE FROM watched_folders WHERE id = ?1", params![id])?;
    Ok(())
}

/// Scanne les dossiers surveillés et renvoie les candidats à valider (F-LOC-03/04).
#[tauri::command(async)]
pub fn scan_watched_folders(state: State<'_, Shared>) -> AppResult<Vec<Candidate>> {
    let folders = watched_paths(&state.conn())?;
    // Le parcours des disques se fait sans tenir le verrou de la base.
    let mut candidates: Vec<Candidate> = folders.iter().flat_map(|f| local::scan_watched_folder(Path::new(f))).collect();
    let conn = state.conn();
    let ignored: std::collections::HashSet<String> = {
        let mut stmt = conn.prepare("SELECT path_norm FROM ignored_candidates")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter().collect()
    };
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for c in candidates.drain(..) {
        let key = norm_path(&c.exe);
        let folder_known = match &c.folder {
            Some(f) => library::path_already_known(&conn, f)?,
            None => false,
        };
        if ignored.contains(&key) || folder_known || library::path_already_known(&conn, &c.exe)? || !seen.insert(key) {
            continue;
        }
        out.push(c);
    }
    Ok(out)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateChoice {
    pub name: String,
    pub folder: Option<String>,
    pub exe: String,
}

/// Ajoute les candidats cochés ; renvoie le nombre de jeux ajoutés.
#[tauri::command(async)]
pub fn add_candidates(app: AppHandle, state: State<'_, Shared>, candidates: Vec<CandidateChoice>) -> AppResult<usize> {
    let mut added = 0;
    for c in candidates {
        match add_local(&mut state.conn(), &c.exe, Some(&c.name), c.folder.as_deref(), None, None) {
            Ok(()) => added += 1,
            Err(e) => log::warn!("candidat {} non ajouté : {e}", c.exe),
        }
    }
    notify_library(&app);
    Ok(added)
}

/// Candidats refusés : ils ne seront plus proposés.
#[tauri::command(async)]
pub fn ignore_candidates(state: State<'_, Shared>, exes: Vec<String>) -> AppResult<()> {
    let conn = state.conn();
    for exe in exes {
        conn.execute("INSERT OR IGNORE INTO ignored_candidates(path_norm) VALUES(?1)", params![norm_path(&exe)])?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Fiche jeu
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub id: String,
    pub source: String,
    pub started_at: String,
    pub ended_at: String,
    pub duration_seconds: i64,
    pub active: bool,
}

#[tauri::command(async)]
pub fn get_sessions(state: State<'_, Shared>, game_id: String) -> AppResult<Vec<SessionView>> {
    let conn = state.conn();
    let mut stmt = conn.prepare(
        "SELECT ps.id, gs.source, ps.started_at, ps.ended_at, ps.duration_seconds, ps.active
         FROM play_sessions ps JOIN game_sources gs ON gs.id = ps.game_source_id
         WHERE gs.game_id = ?1 ORDER BY ps.started_at DESC LIMIT 500",
    )?;
    let rows = stmt
        .query_map(params![game_id], |r| {
            Ok(SessionView { id: r.get(0)?, source: r.get(1)?, started_at: r.get(2)?, ended_at: r.get(3)?, duration_seconds: r.get(4)?, active: r.get(5)? })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

#[tauri::command(async)]
pub fn delete_session(app: AppHandle, state: State<'_, Shared>, session_id: String) -> AppResult<()> {
    state.conn().execute("DELETE FROM play_sessions WHERE id = ?1 AND active = 0", params![session_id])?;
    notify_library(&app);
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GamePatch {
    pub title: Option<String>,
    pub favorite: Option<bool>,
    pub hidden: Option<bool>,
    /// `Some(None)` efface la correction manuelle (F-HEU-07).
    #[serde(default, deserialize_with = "double_option")]
    pub manual_playtime_minutes: Option<Option<i64>>,
}

fn double_option<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<i64>>, D::Error> {
    Ok(Some(Option::<i64>::deserialize(d)?))
}

/// Renommer, mettre en favori, masquer, corriger le temps de jeu (F-MET-04, F-NAV-05/07, F-HEU-07).
#[tauri::command(async)]
pub fn update_game(app: AppHandle, state: State<'_, Shared>, game_id: String, patch: GamePatch) -> AppResult<()> {
    let conn = state.conn();
    if let Some(title) = patch.title.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        conn.execute(
            "UPDATE games SET title = ?2, sort_title = ?3, custom_title = 1 WHERE id = ?1",
            params![game_id, title, library::sort_title(title)],
        )?;
    }
    if let Some(fav) = patch.favorite {
        conn.execute("UPDATE games SET favorite = ?2 WHERE id = ?1", params![game_id, fav])?;
    }
    if let Some(hidden) = patch.hidden {
        conn.execute("UPDATE games SET hidden = ?2 WHERE id = ?1", params![game_id, hidden])?;
    }
    if let Some(manual) = patch.manual_playtime_minutes {
        conn.execute("UPDATE games SET manual_playtime_minutes = ?2 WHERE id = ?1", params![game_id, manual.map(|m| m.max(0))])?;
    }
    drop(conn);
    notify_library(&app);
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePatch {
    pub executable_path: Option<String>,
    pub launch_args: Option<String>,
    pub working_dir: Option<String>,
}

/// Paramètres de lancement d'un jeu local : exécutable, arguments, dossier de travail (F-LOC-05).
#[tauri::command(async)]
pub fn update_source(app: AppHandle, state: State<'_, Shared>, source_id: String, patch: SourcePatch) -> AppResult<()> {
    let conn = state.conn();
    let source = db::get_source(&conn, &source_id)?.ok_or_else(|| AppError::new("GAME_NOT_FOUND", source_id.clone()))?;
    if source.source != SourceId::Local {
        return Err(AppError::new("NOT_EDITABLE", source.source.as_str()));
    }
    if let Some(exe) = patch.executable_path.as_deref().map(str::trim).filter(|e| !e.is_empty()) {
        if !Path::new(exe).is_file() {
            return Err(AppError::new("FILE_NOT_FOUND", exe));
        }
        let exe = crate::drives::clean_path(exe);
        conn.execute(
            "UPDATE game_sources SET executable_path = ?2, launch_target = ?2, external_id = ?3, installed = 1 WHERE id = ?1",
            params![source_id, exe, norm_path(&exe)],
        )
        .map_err(|_| AppError::new("ALREADY_IN_LIBRARY", exe.clone()))?;
    }
    let clean = |v: &Option<String>| v.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(|s| s.to_string());
    if patch.launch_args.is_some() {
        conn.execute("UPDATE game_sources SET launch_args = ?2 WHERE id = ?1", params![source_id, clean(&patch.launch_args)])?;
    }
    if patch.working_dir.is_some() {
        conn.execute("UPDATE game_sources SET working_dir = ?2 WHERE id = ?1", params![source_id, clean(&patch.working_dir)])?;
    }
    drop(conn);
    notify_library(&app);
    Ok(())
}

/// Retire un jeu local de la bibliothèque (ses sessions sont supprimées avec lui).
#[tauri::command(async)]
pub fn remove_game(app: AppHandle, state: State<'_, Shared>, game_id: String) -> AppResult<()> {
    let conn = state.conn();
    let sources = db::sources_of_game(&conn, &game_id)?;
    if sources.iter().any(|s| s.source != SourceId::Local && (s.installed || s.owned)) {
        // Un jeu d'un launcher réapparaîtrait au prochain scan : il faut le masquer.
        return Err(AppError::new("REMOVE_LAUNCHER_GAME", ""));
    }
    conn.execute("DELETE FROM games WHERE id = ?1", params![game_id])?;
    drop(conn);
    notify_library(&app);
    Ok(())
}
