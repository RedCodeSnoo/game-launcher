//! Cœur applicatif du launcher (voir cahier-des-charges-launcher.md, §7).
pub mod backup;
pub mod commands;
pub mod db;
pub mod drives;
pub mod error;
pub mod http;
pub mod launch;
pub mod library;
pub mod logging;
pub mod metadata;
pub mod models;
pub mod online;
pub mod procs;
pub mod providers;
pub mod secrets;
pub mod state;
pub mod stats;
pub mod steam_api;
pub mod sync;
pub mod system;
pub mod tracker;
pub mod tray;
pub mod vdf;

use state::{AppState, Shared};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Affiche la fenêtre principale ; elle est recréée si elle avait été fermée vers la zone de notification.
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    // La création d'une fenêtre ne doit pas se faire depuis un gestionnaire d'événement du fil principal.
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = build_main_window(&app) {
            log::warn!("ouverture de la fenêtre : {e}");
        }
    });
}

fn build_main_window(app: &AppHandle) -> tauri::Result<()> {
    let Some(config) = app.config().app.windows.first().cloned() else { return Ok(()) };
    if app.get_webview_window(&config.label).is_some() {
        return Ok(());
    }
    tauri::WebviewWindowBuilder::from_config(app, &config)?.build()?;
    Ok(())
}

/// Notification Windows ; un échec (notifications désactivées) est sans conséquence.
pub fn notify(app: &AppHandle, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        log::info!("notification non affichée : {e}");
    }
}

/// Seul « Quitter » arrête vraiment l'application (E3-03).
pub fn quit(app: &AppHandle) {
    log::info!("arrêt demandé");
    app.exit(0);
}

/// Clic sur la croix : par défaut la fenêtre se ferme (la vue web est déchargée, E3-08) et l'application
/// continue dans la zone de notification.
fn on_close_requested(app: &AppHandle) {
    let state = app.state::<Shared>();
    let (quits, noticed) = {
        let conn = state.conn();
        (db::get_setting(&conn, "close_quits").as_deref() == Some("1"), db::get_setting(&conn, "tray_notice_shown").is_some())
    };
    if quits {
        quit(app);
        return;
    }
    if !noticed {
        let _ = db::set_setting(&state.conn(), "tray_notice_shown", "1");
        notify(app, "GameLauncher continue en arrière-plan", "Le suivi du temps de jeu reste actif. Pour quitter : clic droit sur l'icône près de l'horloge, puis Quitter.");
    }
}

pub fn run() {
    let autostarted = std::env::args().any(|a| a == system::AUTOSTART_ARG);
    let app = tauri::Builder::default()
        // Instance unique (E3-07) : relancer l'application ramène la fenêtre existante.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main_window(app)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec![system::AUTOSTART_ARG])))
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                on_close_requested(window.app_handle());
            }
        })
        .setup(move |app| {
            let data_dir = match std::env::var_os("GAMELAUNCHER_DATA_DIR") {
                Some(dir) => std::path::PathBuf::from(dir),
                None => app.path().app_data_dir()?,
            };
            std::fs::create_dir_all(&data_dir)?;
            logging::init(&data_dir);
            log::info!("démarrage, données dans {}", data_dir.display());
            let conn = db::open(&data_dir.join("library.db"))?;
            match backup::daily(&conn, &data_dir) {
                Ok(Some(path)) => log::info!("sauvegarde automatique : {}", path.display()),
                Ok(None) => {}
                Err(e) => log::warn!("sauvegarde automatique impossible : {e:#}"),
            }
            let start_minimized = autostarted && db::get_setting(&conn, "start_minimized").as_deref() != Some("0");
            let state: Shared = Arc::new(AppState::new(conn, data_dir));
            app.manage(state.clone());
            let emit = commands::emitter(app.handle());

            // Zone de notification (E3-01), puis fenêtre, sauf démarrage réduit avec Windows (E3-06).
            if let Err(e) = tray::create(app.handle()) {
                log::error!("icône de la zone de notification : {e}");
            }
            if start_minimized {
                log::info!("démarrage réduit dans la zone de notification");
            } else {
                build_main_window(app.handle())?;
            }
            // Entrée de démarrage d'une version précédente : réécrite avec l'argument `--autostart`.
            if system::legacy_autostart_entry() {
                use tauri_plugin_autostart::ManagerExt;
                if let Err(e) = app.autolaunch().enable() {
                    log::warn!("mise à jour de l'entrée de démarrage : {e}");
                }
            }

            // Suivi des heures : le ProcessWatcher observe les processus tant que l'application tourne (E4).
            // Sa première observation reprend ou clôt les sessions restées « en cours » (E1-06).
            {
                let (tracker, state, emit) = (state.tracker.clone(), state.clone(), emit.clone());
                std::thread::spawn(move || tracker.run_loop(state, emit));
            }

            // Branchement ou débranchement d'un disque (F-EXT-03/04).
            {
                let (state, emit) = (state.clone(), emit.clone());
                std::thread::spawn(move || system::watch_drives(state, emit));
            }

            // Détection automatique au démarrage, sans bloquer l'interface (F-DET-05).
            std::thread::spawn(move || {
                sync::run_scan(&state, &emit);
                // Puis synchronisation en ligne (sans effet hors ligne ou sans compte).
                online::run(&state, &emit);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_games,
            commands::refresh_library,
            commands::is_scanning,
            commands::get_running,
            commands::launch_game,
            commands::install_game,
            commands::open_path,
            commands::add_local_game,
            commands::get_watched_folders,
            commands::add_watched_folder,
            commands::remove_watched_folder,
            commands::scan_watched_folders,
            commands::add_candidates,
            commands::ignore_candidates,
            commands::get_sessions,
            commands::delete_session,
            commands::update_game,
            commands::update_source,
            commands::remove_game,
            commands::get_accounts,
            commands::set_steam_account,
            commands::clear_steam_account,
            commands::set_sgdb_key,
            commands::sync_online,
            commands::get_achievements,
            commands::refresh_game,
            commands::search_metadata,
            commands::apply_metadata,
            commands::set_cover,
            commands::get_stats,
            commands::get_sources,
            commands::set_source_enabled,
            commands::get_settings,
            commands::set_setting,
            commands::get_autostart,
            commands::set_autostart,
            commands::export_backup,
            commands::import_backup,
            commands::get_logs,
            commands::open_data_folder,
            commands::get_collections,
            commands::create_collection,
            commands::rename_collection,
            commands::delete_collection,
            commands::set_game_collection,
            commands::get_merge_suggestions,
            commands::merge_games,
            commands::dismiss_merge,
        ])
        .build(tauri::generate_context!())
        .expect("erreur au démarrage de l'application");
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() {
                // Plus aucune fenêtre : l'application reste active dans la zone de notification.
                api.prevent_exit();
            } else {
                // Arrêt réel : dernier battement des sessions en cours.
                let state = app.state::<Shared>();
                state.tracker.flush(&state);
            }
        }
    });
}
