//! Synchronisation en ligne : jeux possédés, heures, succès, puis métadonnées.
//! Tout est facultatif : hors ligne ou sans compte, l'application fonctionne sur ses données locales (§6.3).
use crate::error::AppError;
use crate::models::{GameSource, SourceId};
use crate::state::{AppState, Emitter};
use crate::{db, http, library, metadata, providers};
use anyhow::Result;
use rusqlite::params;
use serde::Serialize;
use serde_json::json;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::Ordering;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncError {
    pub source: SourceId,
    pub code: String,
    pub detail: String,
}

fn sync_error(source: SourceId, e: anyhow::Error) -> SyncError {
    let app: AppError = e.into();
    SyncError { source, code: app.code, detail: app.detail }
}

/// Erreurs après lesquelles il est inutile d'insister pendant cette synchronisation.
fn is_fatal(code: &str) -> bool {
    matches!(code, "OFFLINE" | "STEAM_PRIVATE" | "STEAM_BAD_KEY" | "STEAM_NOT_CONFIGURED" | "RATE_LIMITED" | "SERVICE_UNAVAILABLE")
}

/// Synchronisation complète. Renvoie `None` si une synchronisation est déjà en cours.
pub fn run(state: &AppState, emit: &Emitter) -> Option<Vec<SyncError>> {
    if state.syncing.swap(true, Ordering::SeqCst) {
        return None;
    }
    emit("sync-state", json!({ "running": true }));
    let mut errors = Vec::new();
    let disabled = state.disabled_sources();
    let cfg = state.provider_config();

    for provider in providers::all(&cfg) {
        let source = provider.id();
        let caps = provider.capabilities();
        if disabled.contains(source.as_str()) || !caps.owned {
            continue;
        }
        // 1. Jeux possédés et temps de jeu.
        match catch_unwind(AssertUnwindSafe(|| provider.fetch_owned())) {
            Ok(Ok(owned)) => {
                if !owned.is_empty() {
                    match library::apply_owned(&mut state.conn(), source, &owned) {
                        Ok(added) => log::info!("{} : {} jeu(x) possédé(s), {} nouveau(x)", source.as_str(), owned.len(), added),
                        Err(e) => errors.push(sync_error(source, e)),
                    }
                    emit("library-updated", json!(null));
                }
            }
            Ok(Err(e)) => {
                let err = sync_error(source, e);
                log::warn!("{} : jeux possédés indisponibles ({})", source.as_str(), err.code);
                errors.push(err);
                continue;
            }
            Err(_) => {
                errors.push(SyncError { source, code: "INTERNAL".into(), detail: "erreur interne du module".into() });
                continue;
            }
        }
        // 2. Succès des jeux joués ou installés qui ont changé depuis la dernière synchronisation.
        if caps.achievements {
            let candidates = achievement_candidates(state, source).unwrap_or_default();
            let mut done = 0;
            for g in candidates {
                match sync_source_achievements(state, provider.as_ref(), &g) {
                    Ok(()) => {
                        done += 1;
                        if done % 10 == 0 {
                            emit("library-updated", json!(null));
                        }
                        std::thread::sleep(Duration::from_millis(150));
                    }
                    Err(e) => {
                        let err = sync_error(source, e);
                        if is_fatal(&err.code) {
                            errors.push(err);
                            break;
                        }
                        log::warn!("{} : succès de {} indisponibles ({})", source.as_str(), g.external_id, err.code);
                    }
                }
            }
            if done > 0 {
                log::info!("{} : succès synchronisés pour {} jeu(x)", source.as_str(), done);
                emit("library-updated", json!(null));
            }
        }
    }

    state.syncing.store(false, Ordering::SeqCst);
    emit("sync-state", json!({ "running": false, "errors": errors }));
    // 3. Images et fiches manquantes.
    metadata::run_background(state, emit);
    Some(errors)
}

fn achievement_candidates(state: &AppState, source: SourceId) -> Result<Vec<GameSource>> {
    let conn = state.conn();
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM game_sources
         WHERE source = ?1 AND (
            (achievements_synced_at IS NULL AND (COALESCE(source_playtime_minutes, 0) > 0 OR installed = 1))
            OR (achievements_synced_at IS NOT NULL AND last_played_source > achievements_synced_at))
         ORDER BY last_played_source DESC",
        db::SOURCE_COLS
    ))?;
    let rows = stmt.query_map(params![source.as_str()], db::row_to_source)?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn sync_source_achievements(state: &AppState, provider: &dyn providers::GameProvider, g: &GameSource) -> Result<()> {
    let achievements = provider.fetch_achievements(g)?;
    library::store_achievements(&mut state.conn(), &g.id, achievements.as_deref())?;
    Ok(())
}

/// Rafraîchit un seul jeu à la demande (ouverture de sa fiche) : succès, images, description.
/// Renvoie vrai si quelque chose a changé.
pub fn refresh_game(state: &AppState, game_id: &str, force: bool) -> Result<bool> {
    // Durée de vie (IGDB, facultatif) : une recherche par jeu tous les 30 jours au plus.
    let ttb_changed = refresh_time_to_beat(state, game_id, force).unwrap_or_else(|e| {
        log::info!("durée de vie : {e:#}");
        false
    });
    let cfg = state.provider_config();
    let disabled = state.disabled_sources();
    let sources = db::sources_of_game(&state.conn(), game_id)?;
    let changed = false;
    let mut first_error: Option<anyhow::Error> = None;

    let mut changed = changed || ttb_changed;
    for s in &sources {
        if disabled.contains(s.source.as_str()) {
            continue;
        }
        let Some(provider) = providers::by_id(&cfg, s.source) else { continue };
        if !provider.capabilities().achievements {
            continue;
        }
        let synced_at: Option<String> = state.conn().query_row("SELECT achievements_synced_at FROM game_sources WHERE id = ?1", params![s.id], |r| r.get(0))?;
        let stale = match synced_at.as_deref().and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok()) {
            Some(t) => chrono::Utc::now().signed_duration_since(t) > chrono::Duration::minutes(10),
            None => true,
        };
        if !(force || stale) {
            continue;
        }
        match sync_source_achievements(state, provider.as_ref(), s) {
            Ok(()) => changed = true,
            Err(e) => {
                first_error.get_or_insert(e);
            }
        }
    }

    let (images_checked, details_checked): (bool, bool) =
        state.conn().query_row("SELECT images_checked, details_checked FROM games WHERE id = ?1", params![game_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    if force || !images_checked {
        if force {
            state.conn().execute("UPDATE games SET images_checked = 0 WHERE id = ?1", params![game_id])?;
        }
        match metadata::fetch_images(state, game_id) {
            Ok(c) => changed |= c,
            Err(e) => {
                first_error.get_or_insert(e);
            }
        }
    }
    if force || !details_checked {
        match metadata::fetch_details(state, game_id) {
            Ok(c) => changed |= c,
            Err(e) => {
                first_error.get_or_insert(e);
            }
        }
    }
    match first_error {
        // Hors ligne ou compte non configuré : rien d'anormal, on garde les données locales.
        Some(e) if force && !http::is_offline(&e) => {
            let app: AppError = e.into();
            if app.code == "STEAM_NOT_CONFIGURED" {
                Ok(changed)
            } else {
                Err(app.into())
            }
        }
        _ => Ok(changed),
    }
}

/// Durée de vie d'un jeu via IGDB (si les identifiants Twitch sont enregistrés).
pub fn refresh_time_to_beat(state: &AppState, game_id: &str, force: bool) -> Result<bool> {
    if !crate::igdb::configured() {
        return Ok(false);
    }
    let (title, checked): (String, Option<String>) = state.conn().query_row("SELECT title, time_to_beat_checked FROM games WHERE id = ?1", params![game_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    let recent = checked.as_deref().and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok()).map_or(false, |t| chrono::Utc::now().signed_duration_since(t) < chrono::Duration::days(30));
    if recent && !force {
        return Ok(false);
    }
    let found = crate::igdb::fetch(&title)?;
    state.conn().execute(
        "UPDATE games SET time_to_beat_main = COALESCE(?2, time_to_beat_main), time_to_beat_full = COALESCE(?3, time_to_beat_full), time_to_beat_checked = ?4 WHERE id = ?1",
        params![game_id, found.and_then(|f| f.main), found.and_then(|f| f.full), crate::models::now_iso()],
    )?;
    Ok(found.is_some())
}

