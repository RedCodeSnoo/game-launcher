//! Provider EA app (§4.7) : registre `EA Games\<jeu>` et `__Installer\installerdata.xml` de chaque jeu.
//! Le lancement se fait par l'exécutable du jeu (qui démarre l'EA app si nécessaire) ; à défaut par URI `origin2://`.
use super::GameProvider;
use crate::drives::clean_path;
use crate::models::{Capabilities, DetectedGame, LaunchMethod, SourceId};
use anyhow::Result;
use std::path::Path;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
use winreg::RegKey;

pub struct EaProvider;

const GAMES_KEYS: &[&str] = &["SOFTWARE\\WOW6432Node\\EA Games", "SOFTWARE\\EA Games", "SOFTWARE\\WOW6432Node\\Electronic Arts", "SOFTWARE\\Electronic Arts"];

#[derive(Debug, Default, PartialEq)]
pub struct InstallerData {
    pub content_id: Option<String>,
    pub title: Option<String>,
    /// Chemin de l'exécutable relatif au dossier du jeu.
    pub exe: Option<String>,
}

fn tag_text<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}");
    // La balise doit être exactement `tag` (« contentID » ne doit pas correspondre à « contentIDs »).
    let mut from = 0;
    let after = loop {
        let start = from + xml[from..].find(&open)?;
        let rest = &xml[start + open.len()..];
        if rest.starts_with(['>', ' ', '\t', '\r', '\n', '/']) {
            break rest;
        }
        from = start + open.len();
    };
    let body_start = after.find('>')? + 1;
    let end = after.find(&format!("</{tag}>"))?;
    (end >= body_start).then(|| after[body_start..end].trim())
}

/// Extrait l'identifiant, le titre et l'exécutable d'un `installerdata.xml` (analyse textuelle tolérante).
pub fn parse_installer_data(xml: &str) -> InstallerData {
    let non_empty = |s: &str| (!s.is_empty()).then(|| s.to_string());
    // <launcher> … <filePath>[HKEY_LOCAL_MACHINE\SOFTWARE\...\Install Dir]Bin\Game.exe</filePath>
    let exe = tag_text(xml, "launcher")
        .and_then(|l| tag_text(l, "filePath"))
        .map(|p| p.rsplit(']').next().unwrap_or(p).trim().trim_start_matches(['\\', '/']).to_string())
        .filter(|p| p.to_lowercase().ends_with(".exe"));
    let title = tag_text(xml, "gameTitles").and_then(|t| tag_text(t, "gameTitle")).or_else(|| tag_text(xml, "gameTitle")).and_then(non_empty);
    InstallerData { content_id: tag_text(xml, "contentID").and_then(non_empty), title, exe }
}

/// Construit le jeu à partir de la clé de registre et, s'il existe, de son `installerdata.xml`.
pub fn to_detected(key_name: &str, install_dir: &str, display_name: Option<&str>, data: &InstallerData) -> Option<DetectedGame> {
    let install = clean_path(install_dir);
    if install.is_empty() {
        return None;
    }
    let exe = data.exe.as_deref().map(|rel| clean_path(&format!("{install}\\{rel}")));
    let title = display_name.map(str::trim).filter(|s| !s.is_empty()).map(|s| s.to_string()).or_else(|| data.title.clone()).unwrap_or_else(|| key_name.to_string());
    let (launch_method, launch_target) = match (&exe, &data.content_id) {
        (Some(exe), _) => (LaunchMethod::Exe, exe.clone()),
        (None, Some(id)) => (LaunchMethod::Uri, format!("origin2://game/launch?offerIds={id}")),
        (None, None) => return None, // ni exécutable ni identifiant : impossible à lancer
    };
    Some(DetectedGame {
        source: SourceId::Ea,
        external_id: data.content_id.clone().unwrap_or_else(|| key_name.to_string()),
        title,
        install_path: Some(install),
        executable_path: exe,
        launch_method,
        launch_target,
        launch_args: None,
        working_dir: None,
        last_played: None,
    })
}

fn ea_app_installed() -> bool {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    ["SOFTWARE\\WOW6432Node\\Electronic Arts\\EA Desktop", "SOFTWARE\\Electronic Arts\\EA Desktop", "SOFTWARE\\WOW6432Node\\Origin"].iter().any(|k| hklm.open_subkey_with_flags(k, KEY_READ).is_ok())
}

impl GameProvider for EaProvider {
    fn id(&self) -> SourceId {
        SourceId::Ea
    }

    fn is_available(&self) -> bool {
        ea_app_installed()
    }

    fn scan_installed(&self) -> Result<Vec<DetectedGame>> {
        let mut games: Vec<DetectedGame> = Vec::new();
        for key_path in GAMES_KEYS {
            let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(key_path, KEY_READ) else { continue };
            for name in key.enum_keys().flatten() {
                let Ok(sub) = key.open_subkey_with_flags(&name, KEY_READ) else { continue };
                let Ok(dir) = sub.get_value::<String, _>("Install Dir") else { continue };
                if !Path::new(&dir).is_dir() {
                    continue;
                }
                let display: Option<String> = sub.get_value("DisplayName").ok();
                let data = std::fs::read_to_string(Path::new(&dir).join("__Installer").join("installerdata.xml")).map(|xml| parse_installer_data(&xml)).unwrap_or_default();
                if let Some(g) = to_detected(&name, &dir, display.as_deref(), &data) {
                    let known = games.iter().any(|x| x.external_id == g.external_id || x.install_path == g.install_path);
                    let launchable = g.launch_method != LaunchMethod::Exe || Path::new(&g.launch_target).is_file();
                    if !known && launchable {
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

    const INSTALLER_DATA: &str = include_str!("../../tests/fixtures/ea_installerdata.xml");

    #[test]
    fn parses_installer_data() {
        let d = parse_installer_data(INSTALLER_DATA);
        assert_eq!(d, InstallerData { content_id: Some("1026023".into()), title: Some("Titanfall 2".into()), exe: Some("Titanfall2.exe".into()) });
        assert_eq!(parse_installer_data("<DiPManifest></DiPManifest>"), InstallerData::default());
        assert_eq!(parse_installer_data("pas du xml"), InstallerData::default());
    }

    #[test]
    fn prefers_executable_and_falls_back_to_uri() {
        let d = parse_installer_data(INSTALLER_DATA);
        let g = to_detected("Titanfall2", "D:/EA Games/Titanfall2/", Some("Titanfall™ 2"), &d).unwrap();
        assert_eq!(g.title, "Titanfall™ 2");
        assert_eq!(g.external_id, "1026023");
        assert_eq!((g.launch_method, g.launch_target.as_str()), (LaunchMethod::Exe, "D:\\EA Games\\Titanfall2\\Titanfall2.exe"));

        let no_exe = InstallerData { content_id: Some("42".into()), title: Some("Jeu".into()), exe: None };
        let g = to_detected("Cle", "D:\\EA\\Jeu", None, &no_exe).unwrap();
        assert_eq!((g.launch_method, g.launch_target.as_str(), g.title.as_str()), (LaunchMethod::Uri, "origin2://game/launch?offerIds=42", "Jeu"));
        assert!(to_detected("Cle", "D:\\EA\\Jeu", None, &InstallerData::default()).is_none());
    }
}
