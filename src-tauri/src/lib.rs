//! Cœur applicatif du launcher (voir cahier-des-charges-launcher.md, §7).
pub mod commands;
pub mod db;
pub mod drives;
pub mod error;
pub mod launch;
pub mod library;
pub mod logging;
pub mod models;
pub mod providers;
pub mod state;
pub mod sync;
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
            let state: Shared = Arc::new(AppState::new(conn, data_dir));
            app.manage(state.clone());

            // Détection automatique au démarrage, sans bloquer l'interface (F-DET-05).
            let emit = commands::emitter(app.handle());
            std::thread::spawn(move || {
                sync::run_scan(&state, &emit);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_games,
            commands::refresh_library,
            commands::is_scanning,
            commands::launch_game,
        ])
        .run(tauri::generate_context!())
        .expect("erreur au démarrage de l'application");
}
