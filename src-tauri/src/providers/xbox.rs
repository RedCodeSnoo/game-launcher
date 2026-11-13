//! Provider Xbox / Game Pass PC (§4.8) : dossiers `XboxGames` de chaque disque et leur `MicrosoftGame.config`.
//! Lancement : `shell:AppsFolder\<PackageFamilyName>!<AppId>`.
use super::GameProvider;
use crate::drives::{clean_path, list_drives};
use crate::models::{Capabilities, DetectedGame, LaunchMethod, SourceId};
use anyhow::Result;
use std::path::{Path, PathBuf};
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
use winreg::RegKey;

pub struct XboxProvider;

#[derive(Debug, Default, PartialEq)]
pub struct GameConfig {
    /// Nom du paquet (`Identity Name`), ex. « Microsoft.624F8B84B80 ».
    pub identity: String,
    pub display_name: Option<String>,
    pub executable: Option<String>,
    pub app_id: String,
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let mut from = 0;
    loop {
        let pos = from + tag[from..].find(&needle)?;
        // Le nom d'attribut doit être complet (« Name » ne doit pas correspondre à « DefaultDisplayName »).
        if pos == 0 || tag.as_bytes()[pos - 1].is_ascii_whitespace() {
            let rest = &tag[pos + needle.len()..];
            return rest.find('"').map(|end| rest[..end].to_string());
        }
        from = pos + needle.len();
    }
}

fn element<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}");
    let mut from = 0;
    loop {
        let start = from + xml[from..].find(&open)?;
        let rest = &xml[start + open.len()..];
        if rest.starts_with([' ', '\t', '\r', '\n', '>', '/']) {
            return rest.find('>').map(|end| &rest[..end]);
        }
        from = start + open.len();
    }
}

/// Analyse textuelle tolérante de `MicrosoftGame.config`.
pub fn parse_game_config(xml: &str) -> Option<GameConfig> {
    let identity = attr(element(xml, "Identity")?, "Name").filter(|s| !s.is_empty())?;
    let exe_tag = element(xml, "Executable");
    let display_name = element(xml, "ShellVisuals")
        .and_then(|t| attr(t, "DefaultDisplayName"))
        .map(|s| s.trim().to_string())
        // « ms-resource:… » est une référence à une ressource, pas un nom lisible.
        .filter(|s| !s.is_empty() && !s.starts_with("ms-resource:"));
    Some(GameConfig {
        identity,
        display_name,
        executable: exe_tag.and_then(|t| attr(t, "Name")).filter(|s| !s.is_empty()),
        app_id: exe_tag.and_then(|t| attr(t, "Id")).filter(|s| !s.is_empty()).unwrap_or_else(|| "Game".to_string()),
    })
}

/// `Nom_1.0.0.0_x64__8wekyb3d8bbwe` → `Nom_8wekyb3d8bbwe`.
pub fn family_from_full_name(full_name: &str) -> Option<String> {
    let name = full_name.split('_').next()?;
    let publisher = full_name.rsplit('_').next()?;
    (full_name.matches('_').count() >= 4 && !name.is_empty() && !publisher.is_empty()).then(|| format!("{name}_{publisher}"))
}

/// Nom de famille du paquet installé pour cet utilisateur, d'après le registre (lecture seule).
fn package_family(identity: &str) -> Option<String> {
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags("Software\\Classes\\Local Settings\\Software\\Microsoft\\Windows\\CurrentVersion\\AppModel\\Repository\\Packages", KEY_READ)
        .ok()?;
    let prefix = format!("{}_", identity.to_lowercase());
    key.enum_keys().flatten().find(|k| k.to_lowercase().starts_with(&prefix)).and_then(|k| family_from_full_name(&k))
}

/// Dossier des jeux Xbox d'un disque : indiqué par le fichier `.GamingRoot`, sinon `XboxGames`.
pub fn parse_gaming_root(bytes: &[u8]) -> Option<String> {
    // En-tête « RGBX » + 4 octets, puis chemin relatif en UTF-16LE terminé par un zéro.
    if bytes.len() < 10 || &bytes[..4] != b"RGBX" {
        return None;
    }
    let units: Vec<u16> = bytes[8..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|u| *u != 0).collect();
    let path = String::from_utf16(&units).ok()?;
    let path = path.trim_matches(['\\', '/']).to_string();
    (!path.is_empty()).then_some(path)
}

fn games_root(drive_root: &Path) -> PathBuf {
    let custom = std::fs::read(drive_root.join(".GamingRoot")).ok().and_then(|b| parse_gaming_root(&b));
    drive_root.join(custom.unwrap_or_else(|| "XboxGames".to_string()))
}

pub fn to_detected(folder: &Path, config: &GameConfig, family: Option<&str>) -> Option<DetectedGame> {
    let install = clean_path(&folder.to_string_lossy());
    let content = folder.join("Content");
    let exe = config.executable.as_deref().map(|e| clean_path(&content.join(e).to_string_lossy()));
    let title = config.display_name.clone().or_else(|| folder.file_name().map(|s| s.to_string_lossy().to_string()))?;
    let (launch_method, launch_target) = match (family, &exe) {
        (Some(family), _) => (LaunchMethod::Uri, format!("shell:AppsFolder\\{family}!{}", config.app_id)),
        // Paquet non enregistré pour cet utilisateur : on tente l'exécutable direct.
        (None, Some(exe)) => (LaunchMethod::Exe, exe.clone()),
        (None, None) => return None,
    };
    Some(DetectedGame {
        source: SourceId::Xbox,
        external_id: config.identity.clone(),
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

/// Scanne un dossier `XboxGames` (testable avec un dossier temporaire).
pub fn scan_games_root(root: &Path, family_of: &dyn Fn(&str) -> Option<String>) -> Vec<DetectedGame> {
    let Ok(entries) = std::fs::read_dir(root) else { return Vec::new() };
    let mut games = Vec::new();
    for entry in entries.flatten() {
        let folder = entry.path();
        let Ok(xml) = std::fs::read_to_string(folder.join("Content").join("MicrosoftGame.config")) else { continue };
        let Some(config) = parse_game_config(&xml) else {
            log::warn!("xbox : MicrosoftGame.config illisible dans {}", folder.display());
            continue;
        };
        if let Some(game) = to_detected(&folder, &config, family_of(&config.identity).as_deref()) {
            games.push(game);
        }
    }
    games
}

impl GameProvider for XboxProvider {
    fn id(&self) -> SourceId {
        SourceId::Xbox
    }

    fn is_available(&self) -> bool {
        list_drives().iter().any(|d| games_root(Path::new(&format!("{}:\\", d.letter))).is_dir())
    }

    fn scan_installed(&self) -> Result<Vec<DetectedGame>> {
        let mut games: Vec<DetectedGame> = Vec::new();
        for drive in list_drives() {
            let root = games_root(Path::new(&format!("{}:\\", drive.letter)));
            for g in scan_games_root(&root, &package_family) {
                if !games.iter().any(|x| x.external_id == g.external_id) {
                    games.push(g);
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

    const CONFIG: &str = include_str!("../../tests/fixtures/MicrosoftGame.config");

    #[test]
    fn parses_game_config() {
        let c = parse_game_config(CONFIG).unwrap();
        assert_eq!(
            c,
            GameConfig { identity: "Microsoft.624F8B84B80".into(), display_name: Some("Halo Infinite".into()), executable: Some("HaloInfinite.exe".into()), app_id: "Game".into() }
        );
        let resource = CONFIG.replace("DefaultDisplayName=\"Halo Infinite\"", "DefaultDisplayName=\"ms-resource:AppName\"");
        assert_eq!(parse_game_config(&resource).unwrap().display_name, None);
        assert_eq!(parse_game_config("<Game></Game>"), None);
    }

    #[test]
    fn derives_package_family_name() {
        assert_eq!(family_from_full_name("Microsoft.624F8B84B80_1.3.2.0_x64__8wekyb3d8bbwe").as_deref(), Some("Microsoft.624F8B84B80_8wekyb3d8bbwe"));
        assert_eq!(family_from_full_name("PasUnPaquet"), None);
    }

    #[test]
    fn reads_custom_games_folder() {
        let mut bytes = b"RGBX\x01\x00\x00\x00".to_vec();
        bytes.extend("Jeux Xbox".encode_utf16().flat_map(|u| u.to_le_bytes()));
        bytes.extend([0, 0]);
        assert_eq!(parse_gaming_root(&bytes).as_deref(), Some("Jeux Xbox"));
        assert_eq!(parse_gaming_root(b"autre"), None);
    }

    #[test]
    fn scans_xbox_games_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let halo = tmp.path().join("Halo Infinite").join("Content");
        std::fs::create_dir_all(&halo).unwrap();
        std::fs::write(halo.join("MicrosoftGame.config"), CONFIG).unwrap();
        let other = tmp.path().join("Jeu Sans Nom").join("Content");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("MicrosoftGame.config"), CONFIG.replace("Microsoft.624F8B84B80", "Studio.Autre").replace("DefaultDisplayName=\"Halo Infinite\"", "DefaultDisplayName=\"ms-resource:x\"")).unwrap();
        std::fs::create_dir_all(tmp.path().join("Vide")).unwrap();

        let family = |identity: &str| (identity == "Microsoft.624F8B84B80").then(|| "Microsoft.624F8B84B80_8wekyb3d8bbwe".to_string());
        let mut games = scan_games_root(tmp.path(), &family);
        games.sort_by(|a, b| a.title.cmp(&b.title));
        assert_eq!(games.len(), 2);
        assert_eq!(games[0].title, "Halo Infinite");
        assert_eq!(games[0].launch_target, "shell:AppsFolder\\Microsoft.624F8B84B80_8wekyb3d8bbwe!Game");
        assert_eq!(games[0].launch_method, LaunchMethod::Uri);
        assert!(games[0].executable_path.as_ref().unwrap().ends_with("Content\\HaloInfinite.exe"));
        // Paquet non enregistré : titre du dossier, exécutable direct.
        assert_eq!((games[1].title.as_str(), games[1].launch_method), ("Jeu Sans Nom", LaunchMethod::Exe));
    }
}
