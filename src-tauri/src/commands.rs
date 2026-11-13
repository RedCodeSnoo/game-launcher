//! Commandes Tauri appelées par l'interface.
use crate::drives::{norm_path, DriveSnapshot};
use crate::error::{AppError, AppResult};
use crate::library::{self, GameView};
use crate::models::{now_iso, LaunchMethod, SourceId};
use crate::providers::local::{self, Candidate};
use crate::state::{Emitter, Shared};
use crate::models::Achievement;
use crate::{db, metadata, online, providers, secrets, steam_api, sync};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{AppHandle, Emitter as _, Manager, State};

/// Envoie un événement à l'interface et tient la zone de notification à jour.
pub fn emit_event(app: &AppHandle, event: &str, payload: serde_json::Value) {
    if event == "session-detected" {
        // Notification facultative (E4-10).
        let title = payload.get("title").and_then(|t| t.as_str()).unwrap_or_default();
        crate::notify(app, "Session détectée", title);
        return;
    }
    let _ = app.emit(event, payload);
    if event == "library-updated" {
        // L'index des jeux connus suit la bibliothèque (E4-04).
        app.state::<Shared>().tracker.invalidate();
    }
    if matches!(event, "library-updated" | "running-changed" | "session-heartbeat") {
        crate::tray::refresh(app);
    }
}

pub fn emitter(app: &AppHandle) -> Emitter {
    let app = app.clone();
    std::sync::Arc::new(move |event: &str, payload: serde_json::Value| emit_event(&app, event, payload))
}

fn notify_library(app: &AppHandle) {
    emit_event(app, "library-updated", serde_json::Value::Null);
}

// ---------------------------------------------------------------------------
// Bibliothèque et détection
// ---------------------------------------------------------------------------

#[tauri::command(async)]
pub fn get_games(state: State<'_, Shared>) -> AppResult<Vec<GameView>> {
    let disabled = state.disabled_sources();
    Ok(library::list_visible_games(&state.conn(), &disabled)?)
}

/// Relance la détection en arrière-plan (F-DET-04).
#[tauri::command]
pub fn refresh_library(app: AppHandle, state: State<'_, Shared>) {
    let state = state.inner().clone();
    let emit = emitter(&app);
    std::thread::spawn(move || {
        sync::run_scan(&state, &emit);
        online::run(&state, &emit);
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
    do_launch(&app, state.inner(), &game_id, source_id.as_deref())?;
    // Option « réduire au lancement » : la fenêtre est fermée, l'application reste dans la zone de notification.
    if db::get_setting(&state.conn(), "minimize_on_launch").as_deref() == Some("1") {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.destroy();
        }
    }
    Ok(())
}

/// Lancement partagé par l'interface et le menu de la zone de notification.
pub fn do_launch(app: &AppHandle, state: &Shared, game_id: &str, source_id: Option<&str>) -> AppResult<()> {
    let sources = db::sources_of_game(&state.conn(), game_id)?;
    let chosen = match source_id {
        Some(id) => sources.iter().find(|s| s.id == id),
        None => sources.iter().find(|s| s.installed && s.available).or_else(|| sources.iter().find(|s| s.installed)).or(sources.first()),
    }
    .ok_or_else(|| AppError::new("GAME_NOT_FOUND", game_id.to_string()))?
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
    state.tracker.start(state, &emitter(app), &chosen, chosen.launch_method != LaunchMethod::Exe);
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
    /// `"app"` (lancée depuis l'application) ou `"detected"` (E4-12).
    pub origin: String,
}

#[tauri::command(async)]
pub fn get_sessions(state: State<'_, Shared>, game_id: String) -> AppResult<Vec<SessionView>> {
    let conn = state.conn();
    let mut stmt = conn.prepare(
        "SELECT ps.id, gs.source, ps.started_at, ps.ended_at, ps.duration_seconds, ps.active, ps.origin
         FROM play_sessions ps JOIN game_sources gs ON gs.id = ps.game_source_id
         WHERE gs.game_id = ?1 ORDER BY ps.started_at DESC LIMIT 500",
    )?;
    let rows = stmt
        .query_map(params![game_id], |r| {
            Ok(SessionView { id: r.get(0)?, source: r.get(1)?, started_at: r.get(2)?, ended_at: r.get(3)?, duration_seconds: r.get(4)?, active: r.get(5)?, origin: r.get(6)? })
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
    pub no_tracking: Option<bool>,
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
    if let Some(no_tracking) = patch.no_tracking {
        conn.execute("UPDATE games SET no_tracking = ?2 WHERE id = ?1", params![game_id, no_tracking])?;
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

// ---------------------------------------------------------------------------
// Comptes en ligne (F-PAR-01) et synchronisation
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Accounts {
    pub steam_id: Option<String>,
    pub steam_key_set: bool,
    /// SteamID du compte connecté sur ce PC, pour pré-remplir le formulaire.
    pub local_steam_id: Option<String>,
    pub sgdb_key_set: bool,
}

#[tauri::command(async)]
pub fn get_accounts(state: State<'_, Shared>) -> Accounts {
    Accounts {
        steam_id: state.provider_config().steam_id,
        steam_key_set: secrets::is_set(secrets::STEAM_API_KEY),
        local_steam_id: providers::steam::local_steam_id(),
        sgdb_key_set: secrets::is_set(secrets::STEAMGRIDDB_API_KEY),
    }
}

fn spawn_online_sync(app: &AppHandle, state: &Shared) {
    let state = state.clone();
    let emit = emitter(app);
    std::thread::spawn(move || {
        online::run(&state, &emit);
    });
}

/// Enregistre le compte Steam après l'avoir vérifié ; renvoie le nombre de jeux possédés.
/// `api_key` absent = conserver la clé déjà enregistrée. La clé va dans le coffre Windows, jamais dans la base.
#[tauri::command(async)]
pub fn set_steam_account(app: AppHandle, state: State<'_, Shared>, steam_id: String, api_key: Option<String>) -> AppResult<usize> {
    let key = match api_key.as_deref().map(str::trim).filter(|k| !k.is_empty()) {
        Some(k) => k.to_string(),
        None => secrets::get(secrets::STEAM_API_KEY).ok_or_else(|| AppError::new("STEAM_KEY_REQUIRED", ""))?,
    };
    let mut id = steam_id.trim().trim_end_matches('/').rsplit('/').next().unwrap_or("").to_string();
    if !steam_api::is_valid_steam_id(&id) {
        // Peut-être une URL personnalisée (steamcommunity.com/id/<nom>).
        id = steam_api::resolve_vanity(&key, &id)?.ok_or_else(|| AppError::new("STEAM_BAD_ID", steam_id.clone()))?;
    }
    let creds = steam_api::SteamCreds { key: key.clone(), steam_id: id.clone() };
    let owned = steam_api::fetch_owned_games(&creds)?;
    secrets::set(secrets::STEAM_API_KEY, &key)?;
    db::set_setting(&state.conn(), "steam_id", &id)?;
    log::info!("compte Steam enregistré ({} jeux possédés)", owned.len());
    spawn_online_sync(&app, state.inner());
    Ok(owned.len())
}

#[tauri::command(async)]
pub fn clear_steam_account(state: State<'_, Shared>) -> AppResult<()> {
    secrets::delete(secrets::STEAM_API_KEY)?;
    db::delete_setting(&state.conn(), "steam_id")?;
    Ok(())
}

/// Clé SteamGridDB (facultative) : `None` ou vide pour la supprimer.
#[tauri::command(async)]
pub fn set_sgdb_key(app: AppHandle, state: State<'_, Shared>, api_key: Option<String>) -> AppResult<()> {
    match api_key.as_deref().map(str::trim).filter(|k| !k.is_empty()) {
        Some(key) => {
            // Vérification de la clé par une recherche simple.
            metadata::sgdb_search(key, "Portal")?;
            secrets::set(secrets::STEAMGRIDDB_API_KEY, key)?;
            metadata::reset_missing(&state.conn())?;
            spawn_online_sync(&app, state.inner());
        }
        None => secrets::delete(secrets::STEAMGRIDDB_API_KEY)?,
    }
    Ok(())
}

#[tauri::command]
pub fn sync_online(app: AppHandle, state: State<'_, Shared>) {
    spawn_online_sync(&app, state.inner());
}

#[tauri::command(async)]
pub fn get_achievements(state: State<'_, Shared>, game_id: String) -> AppResult<Vec<Achievement>> {
    Ok(library::list_achievements(&state.conn(), &game_id)?)
}

/// Met à jour un jeu à l'ouverture de sa fiche (succès, images, description).
#[tauri::command(async)]
pub fn refresh_game(app: AppHandle, state: State<'_, Shared>, game_id: String, force: bool) -> AppResult<bool> {
    let changed = online::refresh_game(&state, &game_id, force)?;
    if changed {
        notify_library(&app);
    }
    Ok(changed)
}

// ---------------------------------------------------------------------------
// Métadonnées (F-MET-03, F-MET-04)
// ---------------------------------------------------------------------------

#[tauri::command(async)]
pub fn search_metadata(term: String) -> AppResult<Vec<metadata::StoreMatch>> {
    Ok(metadata::store_search(term.trim())?)
}

/// Associe le jeu à la fiche confirmée par l'utilisateur.
#[tauri::command(async)]
pub fn apply_metadata(app: AppHandle, state: State<'_, Shared>, game_id: String, appid: i64) -> AppResult<()> {
    metadata::apply_store_match(&state, &game_id, appid)?;
    notify_library(&app);
    Ok(())
}

/// Remplace la jaquette par une image locale ; `path` absent = revenir à la jaquette automatique.
#[tauri::command(async)]
pub fn set_cover(app: AppHandle, state: State<'_, Shared>, game_id: String, path: Option<String>) -> AppResult<()> {
    match path {
        Some(p) => metadata::set_custom_cover(&state, &game_id, Path::new(&p))?,
        None => {
            state.conn().execute("UPDATE games SET custom_cover = 0, cover_path = NULL, images_checked = 0 WHERE id = ?1", params![game_id])?;
            let _ = metadata::fetch_images(&state, &game_id);
        }
    }
    notify_library(&app);
    Ok(())
}

// ---------------------------------------------------------------------------
// Statistiques, sources, réglages (V1.0)
// ---------------------------------------------------------------------------

#[tauri::command(async)]
pub fn get_stats(state: State<'_, Shared>) -> AppResult<crate::stats::Stats> {
    let disabled = state.disabled_sources();
    Ok(crate::stats::compute(&state.conn(), &disabled)?)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    pub source: SourceId,
    /// Launcher détecté sur ce PC (F-DET-01).
    pub available: bool,
    pub enabled: bool,
    pub capabilities: crate::models::Capabilities,
    pub installed_games: i64,
    pub total_games: i64,
}

/// État de chaque source : launcher présent, source activée, nombre de jeux.
#[tauri::command(async)]
pub fn get_sources(state: State<'_, Shared>) -> AppResult<Vec<SourceStatus>> {
    let cfg = state.provider_config();
    let disabled = state.disabled_sources();
    let mut out = Vec::new();
    for provider in providers::all(&cfg) {
        let source = provider.id();
        let available = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| provider.is_available())).unwrap_or(false);
        let (installed_games, total_games): (i64, i64) = state.conn().query_row(
            "SELECT COALESCE(SUM(installed), 0), COUNT(*) FROM game_sources WHERE source = ?1",
            params![source.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        out.push(SourceStatus { source, available, enabled: !disabled.contains(source.as_str()), capabilities: provider.capabilities(), installed_games, total_games });
    }
    Ok(out)
}

/// Active ou désactive une source (F-PAR-02). Les données d'une source désactivée sont conservées mais masquées.
#[tauri::command(async)]
pub fn set_source_enabled(app: AppHandle, state: State<'_, Shared>, source: SourceId, enabled: bool) -> AppResult<()> {
    let mut disabled = state.disabled_sources();
    if enabled {
        disabled.remove(source.as_str());
    } else {
        disabled.insert(source.as_str().to_string());
    }
    let mut list: Vec<String> = disabled.into_iter().collect();
    list.sort();
    db::set_setting(&state.conn(), "disabled_sources", &serde_json::to_string(&list).map_err(anyhow::Error::from)?)?;
    notify_library(&app);
    if enabled {
        refresh_library(app, state);
    }
    Ok(())
}

/// Réglages d'interface modifiables depuis le front (liste fermée : pas d'écriture arbitraire dans la base).
const UI_SETTINGS: &[&str] = &[
    "theme",
    "view",
    "sort",
    "sort_desc",
    "wizard_done",
    "minimize_on_launch",
    "reveal_hidden_achievements",
    "close_quits",
    "start_minimized",
    "track_external",
    "min_session_secs",
    "notify_detected",
    "tracking_exclusions",
];

#[tauri::command(async)]
pub fn get_settings(state: State<'_, Shared>) -> std::collections::HashMap<String, String> {
    let conn = state.conn();
    UI_SETTINGS.iter().filter_map(|k| db::get_setting(&conn, k).map(|v| (k.to_string(), v))).collect()
}

#[tauri::command(async)]
pub fn set_setting(state: State<'_, Shared>, key: String, value: String) -> AppResult<()> {
    if !UI_SETTINGS.contains(&key.as_str()) {
        return Err(AppError::new("INTERNAL", format!("réglage inconnu : {key}")));
    }
    db::set_setting(&state.conn(), &key, &value)?;
    // Les réglages du suivi sont relus par le ProcessWatcher.
    state.tracker.invalidate();
    Ok(())
}

/// Démarrage avec Windows (E3-05), géré par `tauri-plugin-autostart` : entrée de l'utilisateur, sans droits administrateur.
#[tauri::command(async)]
pub fn get_autostart(app: AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command(async)]
pub fn set_autostart(app: AppHandle, enabled: bool) -> AppResult<()> {
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|e| AppError::new("AUTOSTART_FAILED", e.to_string()))?;
    log::info!("démarrage avec Windows : {}", if enabled { "activé" } else { "désactivé" });
    Ok(())
}

// ---------------------------------------------------------------------------
// Sauvegarde et journaux
// ---------------------------------------------------------------------------

#[tauri::command(async)]
pub fn export_backup(state: State<'_, Shared>, path: String) -> AppResult<()> {
    crate::backup::export(&state.conn(), Path::new(&path))?;
    log::info!("sauvegarde exportée");
    Ok(())
}

#[tauri::command(async)]
pub fn import_backup(app: AppHandle, state: State<'_, Shared>, path: String) -> AppResult<()> {
    if !state.tracker.running_games().is_empty() {
        return Err(AppError::new("BACKUP_WHILE_PLAYING", ""));
    }
    crate::backup::import(&mut state.conn(), Path::new(&path))?;
    log::info!("sauvegarde restaurée");
    notify_library(&app);
    refresh_library(app, state);
    Ok(())
}

#[tauri::command(async)]
pub fn get_logs(state: State<'_, Shared>) -> String {
    crate::logging::tail(&state.data_dir, 400)
}

/// Ouvre le dossier des données (base, images, journaux, sauvegardes automatiques).
#[tauri::command(async)]
pub fn open_data_folder(state: State<'_, Shared>, sub: Option<String>) -> AppResult<()> {
    let dir = match sub.as_deref() {
        Some("logs") => state.data_dir.join("logs"),
        Some("backups") => crate::backup::backups_dir(&state.data_dir),
        _ => state.data_dir.clone(),
    };
    crate::launch::shell_open(&dir.to_string_lossy(), None, None)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Collections (F-NAV-06) et fusion de doublons (§5.2)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionView {
    pub id: String,
    pub name: String,
    pub count: i64,
}

#[tauri::command(async)]
pub fn get_collections(state: State<'_, Shared>) -> AppResult<Vec<CollectionView>> {
    let conn = state.conn();
    let mut stmt = conn.prepare(
        "SELECT c.id, c.name, (SELECT COUNT(*) FROM collection_games cg JOIN games g ON g.id = cg.game_id WHERE cg.collection_id = c.id AND g.hidden = 0)
         FROM collections c ORDER BY c.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |r| Ok(CollectionView { id: r.get(0)?, name: r.get(1)?, count: r.get(2)? }))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn clean_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 60 {
        return Err(AppError::new("BAD_NAME", name));
    }
    Ok(name.to_string())
}

#[tauri::command(async)]
pub fn create_collection(app: AppHandle, state: State<'_, Shared>, name: String) -> AppResult<String> {
    let name = clean_name(&name)?;
    let id = db::new_id();
    state.conn().execute("INSERT INTO collections(id, name) VALUES(?1, ?2)", params![id, name]).map_err(|_| AppError::new("NAME_TAKEN", name.clone()))?;
    notify_library(&app);
    Ok(id)
}

#[tauri::command(async)]
pub fn rename_collection(app: AppHandle, state: State<'_, Shared>, id: String, name: String) -> AppResult<()> {
    let name = clean_name(&name)?;
    state.conn().execute("UPDATE collections SET name = ?2 WHERE id = ?1", params![id, name]).map_err(|_| AppError::new("NAME_TAKEN", name.clone()))?;
    notify_library(&app);
    Ok(())
}

#[tauri::command(async)]
pub fn delete_collection(app: AppHandle, state: State<'_, Shared>, id: String) -> AppResult<()> {
    state.conn().execute("DELETE FROM collections WHERE id = ?1", params![id])?;
    notify_library(&app);
    Ok(())
}

#[tauri::command(async)]
pub fn set_game_collection(app: AppHandle, state: State<'_, Shared>, game_id: String, collection_id: String, member: bool) -> AppResult<()> {
    let conn = state.conn();
    if member {
        conn.execute("INSERT OR IGNORE INTO collection_games(collection_id, game_id) VALUES(?1, ?2)", params![collection_id, game_id])?;
    } else {
        conn.execute("DELETE FROM collection_games WHERE collection_id = ?1 AND game_id = ?2", params![collection_id, game_id])?;
    }
    drop(conn);
    notify_library(&app);
    Ok(())
}

#[tauri::command(async)]
pub fn get_merge_suggestions(state: State<'_, Shared>) -> AppResult<Vec<library::MergeSuggestion>> {
    Ok(library::merge_suggestions(&state.conn())?)
}

#[tauri::command(async)]
pub fn merge_games(app: AppHandle, state: State<'_, Shared>, keep: String, others: Vec<String>) -> AppResult<()> {
    library::merge_games(&mut state.conn(), &keep, &others)?;
    notify_library(&app);
    Ok(())
}

#[tauri::command(async)]
pub fn dismiss_merge(state: State<'_, Shared>, title: String) -> AppResult<()> {
    library::dismiss_merge(&state.conn(), &title)?;
    Ok(())
}
