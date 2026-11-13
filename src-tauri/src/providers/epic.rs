//! Provider Epic Games (§4.2) : manifestes `.item` (JSON) et cache du catalogue du launcher.
use super::{reg_string, GameProvider};
use crate::drives::clean_path;
use crate::models::{Capabilities, DetectedGame, LaunchMethod, OwnedGame, SourceId};
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

pub struct EpicProvider;

fn data_dir() -> PathBuf {
    let from_registry = reg_string(winreg::enums::HKEY_LOCAL_MACHINE, "SOFTWARE\\WOW6432Node\\Epic Games\\EpicGamesLauncher", "AppDataPath");
    match from_registry {
        Some(p) => PathBuf::from(clean_path(&p)),
        None => {
            let program_data = std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData".into());
            Path::new(&program_data).join("Epic").join("EpicGamesLauncher").join("Data")
        }
    }
}

pub fn manifests_dir() -> PathBuf {
    data_dir().join("Manifests")
}

#[derive(Debug, Deserialize)]
struct Item {
    #[serde(rename = "DisplayName", default)]
    display_name: String,
    #[serde(rename = "InstallLocation", default)]
    install_location: String,
    #[serde(rename = "LaunchExecutable", default)]
    launch_executable: String,
    #[serde(rename = "AppName", default)]
    app_name: String,
    #[serde(rename = "CatalogNamespace", default)]
    catalog_namespace: String,
    #[serde(rename = "CatalogItemId", default)]
    catalog_item_id: String,
    #[serde(rename = "MainGameAppName", default)]
    main_game_app_name: String,
    #[serde(rename = "bIsApplication", default = "yes")]
    is_application: bool,
    #[serde(rename = "bIsIncompleteInstall", default)]
    incomplete: bool,
    #[serde(rename = "AppCategories", default)]
    app_categories: Vec<String>,
}

fn yes() -> bool {
    true
}

pub fn launch_uri(namespace: &str, item_id: &str, app_name: &str) -> String {
    if namespace.is_empty() || item_id.is_empty() {
        format!("com.epicgames.launcher://apps/{app_name}?action=launch&silent=true")
    } else {
        format!("com.epicgames.launcher://apps/{namespace}%3A{item_id}%3A{app_name}?action=launch&silent=true")
    }
}

/// Analyse un manifeste `.item`. `Ok(None)` pour ce qui n'est pas un jeu jouable (DLC, moteur, greffon, installation incomplète).
pub fn parse_item(content: &str) -> Result<Option<DetectedGame>> {
    let item: Item = serde_json::from_str(content).context("manifeste Epic illisible")?;
    if item.app_name.is_empty() || item.install_location.is_empty() {
        anyhow::bail!("manifeste Epic incomplet (AppName ou InstallLocation absent)");
    }
    let is_dlc = !item.main_game_app_name.is_empty() && item.main_game_app_name != item.app_name;
    let is_tool = item.app_name.starts_with("UE_") || item.app_categories.iter().any(|c| c == "engines" || c == "plugins" || c.starts_with("plugins/"));
    let not_a_game = !item.app_categories.is_empty() && !item.app_categories.iter().any(|c| c == "games");
    if !item.is_application || item.incomplete || is_dlc || is_tool || not_a_game {
        return Ok(None);
    }
    let install = clean_path(&item.install_location);
    let exe = (!item.launch_executable.is_empty()).then(|| clean_path(&format!("{install}\\{}", item.launch_executable)));
    Ok(Some(DetectedGame {
        source: SourceId::Epic,
        launch_target: launch_uri(&item.catalog_namespace, &item.catalog_item_id, &item.app_name),
        external_id: item.app_name.clone(),
        title: if item.display_name.is_empty() { item.app_name } else { item.display_name },
        install_path: Some(install),
        executable_path: exe,
        launch_method: LaunchMethod::Uri,
        launch_args: None,
        working_dir: None,
        last_played: None,
    }))
}

/// Lit tous les manifestes d'un dossier. Un fichier corrompu est ignoré sans bloquer les autres.
pub fn scan_manifests(dir: &Path) -> Result<Vec<DetectedGame>> {
    let entries = std::fs::read_dir(dir).with_context(|| format!("lecture de {}", dir.display()))?;
    let mut games: Vec<DetectedGame> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.extension().map(|e| e.eq_ignore_ascii_case("item")).unwrap_or(false) {
            continue;
        }
        match std::fs::read_to_string(&path).map_err(anyhow::Error::from).and_then(|c| parse_item(&c)) {
            Ok(Some(game)) => {
                if Path::new(game.install_path.as_deref().unwrap_or_default()).is_dir() && !games.iter().any(|g| g.external_id == game.external_id) {
                    games.push(game);
                }
            }
            Ok(None) => {}
            Err(e) => log::warn!("epic : manifeste {} ignoré : {e:#}", path.display()),
        }
    }
    Ok(games)
}

// ---------------------------------------------------------------------------
// Cache du catalogue (`Catalog/catcache.bin`, JSON encodé en base64) : visuels et bibliothèque possédée.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CatalogEntry {
    pub app_name: String,
    pub namespace: String,
    pub item_id: String,
    pub title: String,
    pub description: Option<String>,
    pub developer: Option<String>,
    pub cover_url: Option<String>,
    pub hero_url: Option<String>,
    pub logo_url: Option<String>,
    pub is_game: bool,
}

fn base64_decode(input: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    let (mut buf, mut bits) = (0u32, 0u8);
    for &c in input {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\r' | b'\n' | b' ' => continue,
            _ => return None,
        };
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// Analyse le contenu de `catcache.bin`.
pub fn parse_catalog(raw: &[u8]) -> Result<Vec<CatalogEntry>> {
    let decoded = base64_decode(raw).context("catcache.bin : base64 invalide")?;
    let items: Vec<Value> = serde_json::from_slice(&decoded).context("catcache.bin : JSON invalide")?;
    let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(|s| s.to_string());
    let mut out = Vec::new();
    for item in &items {
        let Some(app_name) = item.pointer("/releaseInfo/0/appId").and_then(Value::as_str) else { continue };
        let categories: Vec<&str> = item.get("categories").and_then(Value::as_array).map(|c| c.iter().filter_map(|x| x.get("path").and_then(Value::as_str)).collect()).unwrap_or_default();
        let image = |kind: &str| {
            item.get("keyImages")
                .and_then(Value::as_array)
                .and_then(|imgs| imgs.iter().find(|i| i.get("type").and_then(Value::as_str) == Some(kind)))
                .and_then(|i| i.get("url").and_then(Value::as_str))
                .map(|s| s.to_string())
        };
        let is_game = categories.contains(&"games") && !categories.iter().any(|c| c.starts_with("addons") || c.starts_with("digitalextras") || *c == "engines" || c.starts_with("plugins"));
        out.push(CatalogEntry {
            app_name: app_name.to_string(),
            namespace: text(item, "namespace").unwrap_or_default(),
            item_id: text(item, "id").unwrap_or_default(),
            title: text(item, "title").unwrap_or_else(|| app_name.to_string()),
            description: text(item, "description"),
            developer: text(item, "developer"),
            cover_url: image("DieselGameBoxTall"),
            hero_url: image("DieselGameBox"),
            logo_url: image("DieselGameBoxLogo"),
            is_game,
        });
    }
    Ok(out)
}

type CatalogCache = Option<(SystemTime, HashMap<String, CatalogEntry>)>;
static CATALOG: Mutex<CatalogCache> = Mutex::new(None);

/// Catalogue indexé par AppName, relu seulement si le fichier a changé.
pub fn catalog() -> HashMap<String, CatalogEntry> {
    let path = data_dir().join("Catalog").join("catcache.bin");
    let Ok(modified) = std::fs::metadata(&path).and_then(|m| m.modified()) else { return HashMap::new() };
    let mut cache = CATALOG.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((stamp, map)) = cache.as_ref() {
        if *stamp == modified {
            return map.clone();
        }
    }
    let map: HashMap<String, CatalogEntry> = match std::fs::read(&path).map_err(anyhow::Error::from).and_then(|raw| parse_catalog(&raw)) {
        Ok(entries) => entries.into_iter().map(|e| (e.app_name.clone(), e)).collect(),
        Err(e) => {
            log::warn!("epic : cache du catalogue illisible : {e:#}");
            HashMap::new()
        }
    };
    *cache = Some((modified, map.clone()));
    map
}

impl GameProvider for EpicProvider {
    fn id(&self) -> SourceId {
        SourceId::Epic
    }

    fn is_available(&self) -> bool {
        manifests_dir().is_dir()
    }

    fn scan_installed(&self) -> Result<Vec<DetectedGame>> {
        scan_manifests(&manifests_dir())
    }

    /// Bibliothèque possédée lue dans le cache local du launcher (pas d'appel à l'API non officielle).
    fn fetch_owned(&self) -> Result<Vec<OwnedGame>> {
        let mut games: Vec<OwnedGame> = catalog()
            .into_values()
            .filter(|e| e.is_game)
            .map(|e| OwnedGame {
                launch_target: launch_uri(&e.namespace, &e.item_id, &e.app_name),
                external_id: e.app_name,
                title: e.title,
                playtime_minutes: None,
                last_played: None,
                launch_method: LaunchMethod::Uri,
            })
            .collect();
        games.sort_by(|a, b| a.title.cmp(&b.title));
        Ok(games)
    }

    fn install_uri(&self, g: &crate::models::GameSource) -> Option<String> {
        Some(g.launch_target.replace("action=launch&silent=true", "action=install"))
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { installed: true, owned: true, playtime: false, achievements: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME: &str = include_str!("../../tests/fixtures/epic_game.item");
    const DLC: &str = include_str!("../../tests/fixtures/epic_dlc.item");
    const PLUGIN: &str = include_str!("../../tests/fixtures/epic_plugin.item");

    #[test]
    fn parses_game_manifest() {
        let g = parse_item(GAME).unwrap().unwrap();
        assert_eq!(g.title, "Fortnite");
        assert_eq!(g.external_id, "Fortnite");
        assert_eq!(g.install_path.as_deref(), Some("C:\\Program Files\\Epic Games\\Fortnite"));
        assert_eq!(g.executable_path.as_deref(), Some("C:\\Program Files\\Epic Games\\Fortnite\\FortniteGame\\Binaries\\Win64\\FortniteLauncher.exe"));
        assert_eq!(g.launch_target, "com.epicgames.launcher://apps/fn%3A4fe75bbc5a674f4f9b356b5c90567da5%3AFortnite?action=launch&silent=true");
    }

    #[test]
    fn skips_dlc_plugins_and_incomplete_installs() {
        assert_eq!(parse_item(DLC).unwrap(), None);
        assert_eq!(parse_item(PLUGIN).unwrap(), None);
        let incomplete = GAME.replace("\"bIsIncompleteInstall\": false", "\"bIsIncompleteInstall\": true");
        assert_eq!(parse_item(&incomplete).unwrap(), None);
    }

    #[test]
    fn rejects_corrupted_manifest() {
        assert!(parse_item("{ \"DisplayName\": \"Tronqué").is_err());
        assert!(parse_item("{}").is_err());
    }

    #[test]
    fn corrupted_file_does_not_hide_other_games() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("Jeu");
        std::fs::create_dir_all(&install).unwrap();
        let manifest = GAME.replace("C:\\\\Program Files\\\\Epic Games\\\\Fortnite", &install.to_string_lossy().replace('\\', "\\\\"));
        std::fs::write(tmp.path().join("A.item"), manifest).unwrap();
        std::fs::write(tmp.path().join("B.item"), "pas du JSON").unwrap();
        std::fs::write(tmp.path().join("C.item"), DLC).unwrap();
        std::fs::write(tmp.path().join("notes.txt"), "ignoré").unwrap();
        let games = scan_manifests(tmp.path()).unwrap();
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].title, "Fortnite");
        assert!(scan_manifests(&tmp.path().join("absent")).is_err());
    }

    #[test]
    fn parses_catalog_cache() {
        // « [{"id":"i1","namespace":"ns","title":"Jeu Un",...}] » encodé en base64.
        let json = r#"[{"id":"i1","namespace":"ns","title":"Jeu Un","description":"Desc","developer":"Studio","keyImages":[{"type":"DieselGameBox","url":"https://example.invalid/wide"},{"type":"DieselGameBoxTall","url":"https://example.invalid/tall"}],"categories":[{"path":"games"},{"path":"applications"}],"releaseInfo":[{"appId":"AppUn"}]},{"id":"i2","namespace":"ns","title":"Extension","categories":[{"path":"addons"},{"path":"games"}],"releaseInfo":[{"appId":"Dlc"}]},{"id":"i3","title":"Audience","categories":[{"path":"audience"}],"releaseInfo":[]}]"#;
        let b64 = {
            const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let mut out = String::new();
            for chunk in json.as_bytes().chunks(3) {
                let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
                out.push(T[(n >> 18) as usize & 63] as char);
                out.push(T[(n >> 12) as usize & 63] as char);
                out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
                out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
            }
            out
        };
        let entries = parse_catalog(b64.as_bytes()).unwrap();
        assert_eq!(entries.len(), 2, "l'entrée sans appId est ignorée");
        assert_eq!(entries[0].title, "Jeu Un");
        assert_eq!(entries[0].cover_url.as_deref(), Some("https://example.invalid/tall"));
        assert_eq!(entries[0].hero_url.as_deref(), Some("https://example.invalid/wide"));
        assert!(entries[0].is_game);
        assert!(!entries[1].is_game, "les extensions ne sont pas des jeux");
        assert!(parse_catalog(b"@@@").is_err());
    }
}
