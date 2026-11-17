//! Mises à jour automatiques (tauri-plugin-updater) depuis les Releases GitHub.
//!
//! Tout se passe côté Rust, pour que la vérification continue quand la fenêtre est fermée (la vue web est alors
//! déchargée) : une vérification peu après le démarrage, puis toutes les 6 h. Une version trouvée déclenche une
//! notification Windows et une bannière dans l'application. L'installation n'a jamais lieu pendant une partie :
//! elle attend la fin de la session. Sans connexion, l'échec est silencieux (journal uniquement).
use crate::db;
use crate::state::Shared;
use serde::Serialize;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

/// Premier contrôle après le démarrage (laisse passer la détection des jeux).
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(30);
/// Intervalle entre deux contrôles tant que l'application tourne.
pub const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 3600);
/// Pendant une partie, l'installation attendue est retentée à cet intervalle.
const WAIT_FOR_GAME_POLL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Available {
    pub version: String,
    pub notes: Option<String>,
    pub date: Option<String>,
}

/// `idle`, `waiting-game` (installation demandée, partie en cours), `downloading`, `installing`, `error`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub available: Option<Available>,
    pub auto_check: bool,
    pub state: String,
    pub last_check: Option<String>,
    /// La dernière vérification a échoué (hors ligne, service indisponible) : rien n'est affiché comme erreur.
    pub last_check_failed: bool,
}

#[derive(Default)]
pub struct Updates {
    pending: Mutex<Option<Update>>,
    notified: Mutex<Option<String>>,
    state: Mutex<String>,
    last_check: Mutex<Option<String>>,
    last_failed: AtomicBool,
    installing: AtomicBool,
}

impl Updates {
    fn set_state(&self, app: &AppHandle, state: &str) {
        *self.state.lock().unwrap_or_else(|e| e.into_inner()) = state.to_string();
        let _ = app.emit("update-state", json!(state));
    }
}

fn available_of(u: &Update) -> Available {
    Available { version: u.version.clone(), notes: u.body.clone().filter(|b| !b.trim().is_empty()), date: u.date.map(|d| d.to_string()) }
}

pub fn auto_check_enabled(state: &Shared) -> bool {
    db::get_setting(&state.conn(), "auto_update_check").as_deref() != Some("0")
}

pub fn info(app: &AppHandle) -> UpdateInfo {
    let updates = app.state::<Updates>();
    let state = app.state::<Shared>();
    let available = updates.pending.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(available_of);
    let current = updates.state.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let last_check = updates.last_check.lock().unwrap_or_else(|e| e.into_inner()).clone();
    UpdateInfo {
        current_version: app.package_info().version.to_string(),
        available,
        auto_check: auto_check_enabled(state.inner()),
        state: if current.is_empty() { "idle".into() } else { current },
        last_check,
        last_check_failed: updates.last_failed.load(Ordering::SeqCst),
    }
}

/// Interroge l'adresse de mise à jour. `Err` = vérification impossible (hors ligne…) : jamais montré comme une erreur.
pub fn check(app: &AppHandle) -> Result<Option<Available>, String> {
    let updates = app.state::<Updates>();
    let handle = app.clone();
    let result = tauri::async_runtime::block_on(async {
        app.updater_builder()
            // Sous Windows, l'installeur est lancé puis l'application se ferme : dernier battement des sessions avant.
            .on_before_exit(move || before_exit(&handle))
            .build()
            .map_err(|e| e.to_string())?
            .check()
            .await
            .map_err(|e| e.to_string())
    });
    *updates.last_check.lock().unwrap_or_else(|e| e.into_inner()) = Some(crate::models::now_iso());
    match result {
        Ok(found) => {
            updates.last_failed.store(false, Ordering::SeqCst);
            let available = found.as_ref().map(available_of);
            *updates.pending.lock().unwrap_or_else(|e| e.into_inner()) = found;
            if let Some(a) = &available {
                log::info!("mise à jour disponible : {}", a.version);
                announce(app, a);
            }
            Ok(available)
        }
        Err(e) => {
            // Hors ligne ou service indisponible : échec silencieux.
            updates.last_failed.store(true, Ordering::SeqCst);
            log::info!("vérification des mises à jour impossible : {e}");
            Err(e)
        }
    }
}

/// Notification Windows (une fois par version) et bannière dans l'application.
fn announce(app: &AppHandle, a: &Available) {
    let updates = app.state::<Updates>();
    let mut notified = updates.notified.lock().unwrap_or_else(|e| e.into_inner());
    let _ = app.emit("update-available", json!(a));
    if notified.as_deref() != Some(a.version.as_str()) {
        *notified = Some(a.version.clone());
        crate::notify(app, &format!("GameLauncher {} est disponible", a.version), "Ouvrez GameLauncher pour l'installer.");
    }
}

/// Vérification au démarrage puis toutes les 6 h, fenêtre ouverte ou non (option « Vérifier automatiquement »).
pub fn run_loop(app: AppHandle) {
    std::thread::sleep(FIRST_CHECK_DELAY);
    loop {
        let state = app.state::<Shared>().inner().clone();
        if auto_check_enabled(&state) && !app.state::<Updates>().installing.load(Ordering::SeqCst) {
            let _ = check(&app);
        }
        std::thread::sleep(CHECK_INTERVAL);
    }
}

/// « Installer et redémarrer » : attend la fin de la partie en cours, télécharge, installe, redémarre.
pub fn install(app: &AppHandle) -> Result<(), crate::error::AppError> {
    let updates = app.state::<Updates>();
    if updates.pending.lock().unwrap_or_else(|e| e.into_inner()).is_none() {
        return Err(crate::error::AppError::new("NO_UPDATE", ""));
    }
    if updates.installing.swap(true, Ordering::SeqCst) {
        return Ok(()); // déjà en cours
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let updates = app.state::<Updates>();
        let state = app.state::<Shared>().inner().clone();
        // Jamais pendant une partie : l'installeur fermerait l'application et couperait le suivi.
        if !state.tracker.running_games().is_empty() {
            log::info!("mise à jour : installation reportée à la fin de la partie en cours");
            updates.set_state(&app, "waiting-game");
            while !state.tracker.running_games().is_empty() {
                std::thread::sleep(WAIT_FOR_GAME_POLL);
            }
        }
        let Some(update) = updates.pending.lock().unwrap_or_else(|e| e.into_inner()).clone() else { return };
        updates.set_state(&app, "downloading");
        log::info!("mise à jour : téléchargement de la version {}", update.version);
        let result = tauri::async_runtime::block_on(update.download_and_install(|_, _| {}, || {}));
        match result {
            // Sous Windows, l'installeur ferme l'application lui-même ; ailleurs, on redémarre.
            Ok(()) => {
                updates.set_state(&app, "installing");
                state.tracker.flush(&state);
                app.restart();
            }
            Err(e) => {
                log::warn!("mise à jour impossible : {e}");
                updates.installing.store(false, Ordering::SeqCst);
                updates.set_state(&app, "error");
            }
        }
    });
    Ok(())
}

/// Dernier battement des sessions avant que l'installeur ferme l'application (Windows).
pub fn before_exit(app: &AppHandle) {
    let state = app.state::<Shared>();
    state.tracker.flush(&state);
    log::info!("mise à jour : fermeture pour installation");
}
