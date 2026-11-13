//! Intégration à Windows : démarrage automatique (F-PAR-04) et surveillance des disques (F-EXT-04).
use crate::drives::DriveSnapshot;
use crate::library;
use crate::state::{AppState, Emitter};
use anyhow::{Context, Result};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
use winreg::RegKey;

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_VALUE: &str = "GameLauncher";

/// L'application démarre-t-elle avec la session Windows ? (clé `Run` de l'utilisateur, sans droits administrateur)
pub fn autostart_enabled() -> bool {
    RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_READ).and_then(|k| k.get_value::<String, _>(RUN_VALUE)).is_ok()
}

pub fn set_autostart(enabled: bool) -> Result<()> {
    let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE).context("ouverture de la clé de démarrage")?;
    if enabled {
        let exe = std::env::current_exe().context("chemin de l'application")?;
        key.set_value(RUN_VALUE, &format!("\"{}\"", exe.display())).context("écriture de la clé de démarrage")?;
    } else {
        match key.delete_value(RUN_VALUE) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e).context("suppression de la clé de démarrage"),
        }
    }
    Ok(())
}

/// Surveille les lecteurs montés (interrogation légère toutes les 5 s). Au branchement ou au débranchement
/// d'un disque : la disponibilité des jeux est mise à jour tout de suite, puis une détection complète est lancée.
pub fn watch_drives(state: Arc<AppState>, emit: Emitter) {
    let signature = |snap: &DriveSnapshot| snap.drives.iter().map(|d| (d.letter, d.serial.clone())).collect::<Vec<_>>();
    let mut last = signature(&DriveSnapshot::current());
    loop {
        std::thread::sleep(Duration::from_secs(5));
        let snap = DriveSnapshot::current();
        let now = signature(&snap);
        if now == last {
            continue;
        }
        log::info!("changement de disques détecté : {} lecteur(s) monté(s)", now.len());
        last = now;
        match library::refresh_availability(&mut state.conn(), &snap) {
            Ok(changed) if changed > 0 => emit("library-updated", json!(null)),
            Ok(_) => {}
            Err(e) => log::error!("mise à jour de la disponibilité des disques : {e:#}"),
        }
        crate::sync::run_scan(&state, &emit);
    }
}
