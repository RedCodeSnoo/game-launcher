//! Provider Steam (§4.1) : bibliothèques multi-disques, manifestes ACF, lancement par URI.
use super::{reg_string, GameProvider};
use crate::drives::clean_path;
use crate::error::AppError;
use crate::models::{Achievement, Capabilities, DetectedGame, GameSource, LaunchMethod, OwnedGame, SourceId};
use crate::secrets;
use crate::steam_api::{self, SteamCreds};
use crate::vdf;
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};

/// Outils Steam qui ne sont pas des jeux.
const EXCLUDED_APPIDS: &[&str] = &[
    "228980",  // Steamworks Common Redistributables
    "250820",  // SteamVR
    "1070560", // Steam Linux Runtime
    "1391110", // Steam Linux Runtime - Soldier
    "1628350", // Steam Linux Runtime - Sniper
    "3810310", // Steam Linux Runtime 4.0
    "1493710", // Proton Experimental
    "1826330", // Proton EasyAntiCheat Runtime
    "1161040", // Proton BattlEye Runtime
    "2180100", // Proton Hotfix
    "858280", "930400", "961940", "1054830", "1113280", "1245040", "1420170", "1580130", "1887720", "2230260", "2348590", "2805730", "3658110", // Proton x.y
];

const EXCLUDED_NAME_PREFIXES: &[&str] = &["proton ", "steam linux runtime", "steamworks common", "steamvr"];

pub struct SteamProvider {
    steam_id: Option<String>,
}

impl SteamProvider {
    pub fn new(steam_id: Option<String>) -> Self {
        SteamProvider { steam_id }
    }

    /// Clé API (coffre Windows) et SteamID64 ; erreur explicite si le compte n'est pas configuré.
    pub fn creds(&self) -> Result<SteamCreds> {
        let steam_id = self.steam_id.clone().ok_or_else(|| AppError::new("STEAM_NOT_CONFIGURED", ""))?;
        let key = secrets::get(secrets::STEAM_API_KEY).ok_or_else(|| AppError::new("STEAM_NOT_CONFIGURED", ""))?;
        Ok(SteamCreds { key, steam_id })
    }

    pub fn is_configured(&self) -> bool {
        self.steam_id.is_some() && secrets::is_set(secrets::STEAM_API_KEY)
    }
}

/// SteamID64 du dernier compte connecté sur ce PC (`config/loginusers.vdf`), pour pré-remplir l'assistant.
pub fn parse_login_users(content: &str) -> Option<String> {
    let doc = vdf::parse(content).ok()?;
    let users = doc.get("users")?;
    let entries = users.entries();
    entries
        .iter()
        .find(|(_, v)| v.str("MostRecent") == Some("1"))
        .or_else(|| entries.iter().max_by_key(|(_, v)| v.str("timestamp").and_then(|t| t.parse::<i64>().ok()).unwrap_or(0)))
        .map(|(id, _)| id.clone())
        .filter(|id| steam_api::is_valid_steam_id(id))
}

pub fn local_steam_id() -> Option<String> {
    let path = steam_path()?.join("config").join("loginusers.vdf");
    parse_login_users(&std::fs::read_to_string(path).ok()?)
}

pub fn steam_path() -> Option<PathBuf> {
    let p = reg_string(winreg::enums::HKEY_CURRENT_USER, "Software\\Valve\\Steam", "SteamPath")
        .or_else(|| reg_string(winreg::enums::HKEY_LOCAL_MACHINE, "SOFTWARE\\WOW6432Node\\Valve\\Steam", "InstallPath"))?;
    let p = PathBuf::from(clean_path(&p));
    p.is_dir().then_some(p)
}

/// Dossiers de bibliothèque listés dans `libraryfolders.vdf` (formats ancien et récent).
pub fn parse_library_folders(content: &str) -> Result<Vec<String>> {
    let doc = vdf::parse(content)?;
    let root = doc
        .get("libraryfolders")
        .or_else(|| doc.get("LibraryFolders"))
        .ok_or_else(|| anyhow!("clé « libraryfolders » absente"))?;
    let mut out = Vec::new();
    for (key, value) in root.entries() {
        if key.parse::<u32>().is_err() {
            continue;
        }
        match value {
            vdf::Vdf::Obj(_) => {
                if let Some(p) = value.str("path") {
                    out.push(clean_path(p));
                }
            }
            // Ancien format : "1" "D:\\SteamLibrary"
            vdf::Vdf::Str(p) => out.push(clean_path(p)),
        }
    }
    Ok(out)
}

#[derive(Debug, PartialEq)]
pub struct AppManifest {
    pub appid: String,
    pub name: String,
    pub installdir: String,
    pub last_played: Option<i64>,
}

pub fn parse_app_manifest(content: &str) -> Result<AppManifest> {
    let doc = vdf::parse(content)?;
    let state = doc.get("AppState").ok_or_else(|| anyhow!("clé « AppState » absente"))?;
    Ok(AppManifest {
        appid: state.str("appid").ok_or_else(|| anyhow!("appid absent"))?.to_string(),
        name: state.str("name").ok_or_else(|| anyhow!("name absent"))?.to_string(),
        installdir: state.str("installdir").ok_or_else(|| anyhow!("installdir absent"))?.to_string(),
        last_played: state.str("LastPlayed").and_then(|s| s.parse::<i64>().ok()).filter(|v| *v > 0),
    })
}

pub fn is_excluded(appid: &str, name: &str) -> bool {
    let lower = name.to_lowercase();
    EXCLUDED_APPIDS.contains(&appid) || EXCLUDED_NAME_PREFIXES.iter().any(|p| lower.starts_with(p))
}

/// Scanne les bibliothèques d'une installation Steam (testable avec un dossier temporaire).
pub fn scan_steam_root(steam_root: &Path) -> Result<Vec<DetectedGame>> {
    let vdf_path = steam_root.join("steamapps").join("libraryfolders.vdf");
    let mut libraries = vec![clean_path(&steam_root.to_string_lossy())];
    match std::fs::read_to_string(&vdf_path) {
        Ok(content) => {
            let listed = parse_library_folders(&content).with_context(|| format!("lecture de {}", vdf_path.display()))?;
            for lib in listed {
                if !libraries.iter().any(|l| l.eq_ignore_ascii_case(&lib)) {
                    libraries.push(lib);
                }
            }
        }
        Err(e) => log::warn!("steam: libraryfolders.vdf illisible ({e}), seule la bibliothèque principale est scannée"),
    }

    let mut games: Vec<DetectedGame> = Vec::new();
    for lib in libraries {
        let apps_dir = Path::new(&lib).join("steamapps");
        let Ok(entries) = std::fs::read_dir(&apps_dir) else {
            continue; // disque débranché ou bibliothèque supprimée
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                continue;
            }
            // Un manifeste corrompu n'empêche pas de lire les autres.
            let manifest = match std::fs::read_to_string(entry.path()).map_err(anyhow::Error::from).and_then(|c| parse_app_manifest(&c)) {
                Ok(m) => m,
                Err(e) => {
                    log::warn!("steam: manifeste {name} ignoré : {e:#}");
                    continue;
                }
            };
            if is_excluded(&manifest.appid, &manifest.name) || games.iter().any(|g| g.external_id == manifest.appid) {
                continue;
            }
            let install = apps_dir.join("common").join(&manifest.installdir);
            if !install.is_dir() {
                continue; // téléchargement jamais commencé
            }
            games.push(DetectedGame {
                source: SourceId::Steam,
                launch_target: format!("steam://rungameid/{}", manifest.appid),
                external_id: manifest.appid,
                title: manifest.name,
                install_path: Some(clean_path(&install.to_string_lossy())),
                executable_path: None,
                launch_method: LaunchMethod::Uri,
                launch_args: None,
                working_dir: None,
                last_played: manifest.last_played,
            });
        }
    }
    Ok(games)
}

impl GameProvider for SteamProvider {
    fn id(&self) -> SourceId {
        SourceId::Steam
    }

    fn is_available(&self) -> bool {
        steam_path().is_some()
    }

    fn scan_installed(&self) -> Result<Vec<DetectedGame>> {
        let root = steam_path().ok_or_else(|| anyhow!("Steam n'est pas installé"))?;
        scan_steam_root(&root)
    }

    fn fetch_owned(&self) -> Result<Vec<OwnedGame>> {
        if !self.is_configured() {
            return Ok(Vec::new());
        }
        let games = steam_api::fetch_owned_games(&self.creds()?)?;
        Ok(games.into_iter().filter(|g| !is_excluded(&g.external_id, &g.title)).collect())
    }

    fn fetch_playtime(&self, g: &GameSource) -> Result<Option<i64>> {
        // Le temps de jeu arrive avec la liste des jeux possédés (un seul appel pour toute la bibliothèque).
        Ok(g.source_playtime_minutes)
    }

    fn fetch_achievements(&self, g: &GameSource) -> Result<Option<Vec<Achievement>>> {
        steam_api::fetch_achievements(&self.creds()?, &g.external_id)
    }

    fn install_uri(&self, g: &GameSource) -> Option<String> {
        Some(format!("steam://install/{}", g.external_id))
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { installed: true, owned: true, playtime: true, achievements: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIBRARY_FOLDERS: &str = include_str!("../../tests/fixtures/libraryfolders.vdf");
    const MANIFEST: &str = include_str!("../../tests/fixtures/appmanifest_108600.acf");

    #[test]
    fn parses_library_folders_on_several_disks() {
        let libs = parse_library_folders(LIBRARY_FOLDERS).unwrap();
        assert_eq!(libs, vec!["C:\\Program Files (x86)\\Steam", "D:\\SteamLibrary", "E:\\Jeux\\Steam"]);
    }

    #[test]
    fn parses_legacy_library_folders() {
        let libs = parse_library_folders("\"LibraryFolders\"\n{\n\t\"TimeNextStatsReport\"\t\"1\"\n\t\"1\"\t\"D:\\\\Games\"\n}").unwrap();
        assert_eq!(libs, vec!["D:\\Games"]);
    }

    #[test]
    fn parses_app_manifest() {
        let m = parse_app_manifest(MANIFEST).unwrap();
        assert_eq!(m, AppManifest { appid: "108600".into(), name: "Project Zomboid".into(), installdir: "ProjectZomboid".into(), last_played: Some(1792771130) });
    }

    #[test]
    fn finds_most_recent_local_account() {
        let doc = r#""users" { "76561198000000001" { "AccountName" "a" "MostRecent" "0" "timestamp" "10" } "76561198000000002" { "AccountName" "b" "MostRecent" "1" "timestamp" "5" } }"#;
        assert_eq!(parse_login_users(doc).as_deref(), Some("76561198000000002"));
        let single = r#""users" { "76561198000000003" { "timestamp" "7" } }"#;
        assert_eq!(parse_login_users(single).as_deref(), Some("76561198000000003"));
        assert_eq!(parse_login_users(r#""users" { }"#), None);
    }

    #[test]
    fn excludes_tools() {
        assert!(is_excluded("228980", "Steamworks Common Redistributables"));
        assert!(is_excluded("9999999", "Proton 11.0"));
        assert!(!is_excluded("108600", "Project Zomboid"));
    }

    #[test]
    fn scans_libraries_and_survives_corrupted_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Steam");
        let second = tmp.path().join("Ext");
        for lib in [&root, &second] {
            std::fs::create_dir_all(lib.join("steamapps").join("common")).unwrap();
        }
        let esc = |p: &Path| p.to_string_lossy().replace('\\', "\\\\");
        std::fs::write(
            root.join("steamapps").join("libraryfolders.vdf"),
            format!("\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\"{}\"\n\t}}\n\t\"1\"\n\t{{\n\t\t\"path\"\t\"{}\"\n\t}}\n\t\"2\"\n\t{{\n\t\t\"path\"\t\"Z:\\\\Absent\"\n\t}}\n}}\n", esc(&root), esc(&second)),
        )
        .unwrap();
        let manifest = |id: &str, name: &str, dir: &str| format!("\"AppState\"\n{{\n\t\"appid\"\t\"{id}\"\n\t\"name\"\t\"{name}\"\n\t\"installdir\"\t\"{dir}\"\n}}\n");
        std::fs::create_dir_all(root.join("steamapps/common/Alpha")).unwrap();
        std::fs::write(root.join("steamapps/appmanifest_10.acf"), manifest("10", "Alpha", "Alpha")).unwrap();
        std::fs::write(root.join("steamapps/appmanifest_228980.acf"), manifest("228980", "Steamworks Common Redistributables", "Steamworks Shared")).unwrap();
        std::fs::write(root.join("steamapps/appmanifest_66.acf"), "ceci n'est pas du VDF {{{").unwrap();
        std::fs::create_dir_all(second.join("steamapps/common/Beta")).unwrap();
        std::fs::write(second.join("steamapps/appmanifest_20.acf"), manifest("20", "Beta", "Beta")).unwrap();
        // Manifeste sans dossier d'installation : ignoré.
        std::fs::write(second.join("steamapps/appmanifest_30.acf"), manifest("30", "Gamma", "Gamma")).unwrap();

        let mut games = scan_steam_root(&root).unwrap();
        games.sort_by(|a, b| a.external_id.cmp(&b.external_id));
        assert_eq!(games.iter().map(|g| g.title.as_str()).collect::<Vec<_>>(), vec!["Alpha", "Beta"]);
        assert_eq!(games[1].launch_target, "steam://rungameid/20");
        assert!(games[1].install_path.as_ref().unwrap().ends_with("Ext\\steamapps\\common\\Beta"));
    }
}
