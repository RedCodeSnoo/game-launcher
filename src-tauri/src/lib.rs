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
pub mod providers;
pub mod secrets;
pub mod state;
pub mod stats;
pub mod steam_api;
pub mod sync;
pub mod system;
pub mod tracker;
pub mod vdf;

use state::{AppState, Shared};
use std::sync::Arc;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = match std::env::var_os("GAMELAUNCHER_DATA_DIR") {
                Some(dir) => std::path::PathBuf::from(dir),
                None => app.path().app_data_dir()?,
            };
            std::fs::create_dir_all(&data_dir)?;
            logging::init(&data_dir);
            log::info!("démarrage, données dans {}", data_dir.display());
            let conn = db::open(&data_dir.join("library.db"))?;
            if let Err(e) = tracker::close_orphan_sessions(&conn) {
                log::error!("clôture des sessions orphelines : {e}");
            }
            match backup::daily(&conn, &data_dir) {
                Ok(Some(path)) => log::info!("sauvegarde automatique : {}", path.display()),
                Ok(None) => {}
                Err(e) => log::warn!("sauvegarde automatique impossible : {e:#}"),
            }
            let state: Shared = Arc::new(AppState::new(conn, data_dir));
            app.manage(state.clone());
            let emit = commands::emitter(app.handle());

            // Suivi des heures : un fil qui dort tant qu'aucun jeu n'est lancé.
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
        .run(tauri::generate_context!())
        .expect("erreur au démarrage de l'application");
}
