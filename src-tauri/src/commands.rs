//! Commandes Tauri appelées par l'interface.
use crate::error::{AppError, AppResult};
use crate::library::{self, GameView};
use crate::models::SourceId;
use crate::state::{Emitter, Shared};
use crate::{db, providers, sync};
use tauri::{AppHandle, Emitter as _, State};

pub fn emitter(app: &AppHandle) -> Emitter {
    let app = app.clone();
    std::sync::Arc::new(move |event: &str, payload: serde_json::Value| {
        let _ = app.emit(event, payload);
    })
}

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

/// Lance un jeu (F-LAN-01) avec des erreurs explicites (F-LAN-02).
#[tauri::command(async)]
pub fn launch_game(state: State<'_, Shared>, game_id: String, source_id: Option<String>) -> AppResult<()> {
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
    Ok(())
}
