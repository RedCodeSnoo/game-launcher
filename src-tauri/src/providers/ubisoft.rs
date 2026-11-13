//! Provider Ubisoft Connect (§4.6) : registre `Ubisoft\Launcher\Installs\<id>`, lancement `uplay://launch/<id>/0`.
use super::{reg_string, GameProvider};
use crate::drives::clean_path;
use crate::models::{Capabilities, DetectedGame, LaunchMethod, SourceId};
use anyhow::Result;
use std::path::Path;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
use winreg::RegKey;

pub struct UbisoftProvider;

const INSTALLS_KEYS: &[&str] = &["SOFTWARE\\WOW6432Node\\Ubisoft\\Launcher\\Installs", "SOFTWARE\\Ubisoft\\Launcher\\Installs"];
const UNINSTALL_KEYS: &[&str] = &["SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall", "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall"];

/// Construit le jeu à partir de son identifiant, de son dossier et du nom affiché (s'il est connu).
pub fn to_detected(id: &str, install_dir: &str, display_name: Option<&str>) -> Option<DetectedGame> {
    let install = clean_path(install_dir);
    if id.is_empty() || install.is_empty() {
        return None;
    }
    let title = display_name
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| Path::new(&install).file_name().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| format!("Ubisoft {id}"));
    Some(DetectedGame {
        source: SourceId::Ubisoft,
        external_id: id.to_string(),
        title,
        install_path: Some(install),
        executable_path: None,
        launch_method: LaunchMethod::Uri,
        launch_target: format!("uplay://launch/{id}/0"),
        launch_args: None,
        working_dir: None,
        last_played: None,
    })
}

fn display_name(id: &str) -> Option<String> {
    UNINSTALL_KEYS.iter().find_map(|k| reg_string(HKEY_LOCAL_MACHINE, &format!("{k}\\Uplay Install {id}"), "DisplayName"))
}

impl GameProvider for UbisoftProvider {
    fn id(&self) -> SourceId {
        SourceId::Ubisoft
    }

    fn is_available(&self) -> bool {
        reg_string(HKEY_LOCAL_MACHINE, "SOFTWARE\\WOW6432Node\\Ubisoft\\Launcher", "InstallDir").is_some()
            || INSTALLS_KEYS.iter().any(|k| RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(k, KEY_READ).is_ok())
    }

    fn scan_installed(&self) -> Result<Vec<DetectedGame>> {
        let mut games: Vec<DetectedGame> = Vec::new();
        for key_path in INSTALLS_KEYS {
            let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(key_path, KEY_READ) else { continue };
            for id in key.enum_keys().flatten() {
                let Some(dir) = reg_string(HKEY_LOCAL_MACHINE, &format!("{key_path}\\{id}"), "InstallDir") else { continue };
                if let Some(g) = to_detected(&id, &dir, display_name(&id).as_deref()) {
                    if Path::new(g.install_path.as_deref().unwrap_or_default()).is_dir() && !games.iter().any(|x| x.external_id == g.external_id) {
                        games.push(g);
                    }
                }
            }
        }
        Ok(games)
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { installed: true, owned: false, playtime: false, achievements: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_games_from_registry_values() {
        let g = to_detected("635", "D:/Ubisoft/Rainbow Six Siege/", Some(" Tom Clancy's Rainbow Six Siege ")).unwrap();
        assert_eq!(g.title, "Tom Clancy's Rainbow Six Siege");
        assert_eq!(g.install_path.as_deref(), Some("D:\\Ubisoft\\Rainbow Six Siege"));
        assert_eq!(g.launch_target, "uplay://launch/635/0");
        assert_eq!(g.launch_method, LaunchMethod::Uri);
        // Sans nom dans le registre : le nom du dossier.
        assert_eq!(to_detected("46", "C:\\Jeux\\Far Cry 3", None).unwrap().title, "Far Cry 3");
        assert!(to_detected("", "C:\\x", None).is_none());
        assert!(to_detected("1", "  ", None).is_none());
    }
}
