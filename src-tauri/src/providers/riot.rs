//! Provider Riot Games (§4.5) : `RiotClientInstalls.json` et `Metadata/<produit>.<patchline>/*.product_settings.yaml`.
use super::GameProvider;
use crate::drives::clean_path;
use crate::models::{Capabilities, DetectedGame, LaunchMethod, SourceId};
use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct RiotProvider;

fn riot_data_dir() -> PathBuf {
    let program_data = std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData".into());
    Path::new(&program_data).join("Riot Games")
}

/// Chemin de `RiotClientServices.exe` d'après `RiotClientInstalls.json`.
pub fn parse_client_installs(content: &str) -> Option<String> {
    let json: Value = serde_json::from_str(content).ok()?;
    ["rc_live", "rc_default", "rc_beta"].iter().find_map(|k| json.get(k).and_then(Value::as_str)).filter(|s| !s.is_empty()).map(clean_path)
}

#[derive(Debug, PartialEq)]
pub struct ProductSettings {
    pub install_path: String,
    pub shortcut_name: Option<String>,
}

/// Lit `product_install_full_path` (et le nom du raccourci) d'un `product_settings.yaml`.
pub fn parse_product_settings(content: &str) -> Result<Option<ProductSettings>> {
    let doc: serde_yaml::Value = serde_yaml::from_str(content).context("product_settings.yaml illisible")?;
    let Some(path) = doc.get("product_install_full_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) else {
        return Ok(None); // produit connu du client mais pas installé
    };
    Ok(Some(ProductSettings {
        install_path: clean_path(path),
        shortcut_name: doc.get("shortcut_name").and_then(|v| v.as_str()).map(|s| s.trim_end_matches(".lnk").to_string()).filter(|s| !s.is_empty()),
    }))
}

fn product_title(product: &str, patchline: &str, shortcut: Option<&str>) -> String {
    let base = match product {
        "league_of_legends" => "League of Legends".to_string(),
        "valorant" => "VALORANT".to_string(),
        "bacon" => "Legends of Runeterra".to_string(),
        "lion" => "2XKO".to_string(),
        "teamfighttactics" => "Teamfight Tactics".to_string(),
        other => shortcut.map(|s| s.to_string()).unwrap_or_else(|| other.replace('_', " ")),
    };
    if patchline == "live" {
        base
    } else {
        format!("{base} ({})", patchline.to_uppercase())
    }
}

/// Scanne le dossier de données Riot (testable avec un dossier temporaire).
pub fn scan_riot(data_dir: &Path) -> Result<Vec<DetectedGame>> {
    let installs = std::fs::read_to_string(data_dir.join("RiotClientInstalls.json")).context("RiotClientInstalls.json introuvable")?;
    let client = parse_client_installs(&installs).ok_or_else(|| anyhow!("chemin du Riot Client absent de RiotClientInstalls.json"))?;
    let Ok(entries) = std::fs::read_dir(data_dir.join("Metadata")) else {
        return Ok(Vec::new());
    };
    let mut games = Vec::new();
    for entry in entries.flatten() {
        let dir_name = entry.file_name().to_string_lossy().to_string();
        // Dossiers attendus : « <produit>.<patchline> ».
        let Some((product, patchline)) = dir_name.split_once('.') else { continue };
        if patchline.contains('.') || product.is_empty() {
            continue; // ex. league_of_legends.live.game_patch
        }
        let settings_path = entry.path().join(format!("{dir_name}.product_settings.yaml"));
        let Ok(content) = std::fs::read_to_string(&settings_path) else { continue };
        let settings = match parse_product_settings(&content) {
            Ok(Some(s)) => s,
            Ok(None) => continue,
            Err(e) => {
                log::warn!("riot : {dir_name} ignoré : {e:#}");
                continue;
            }
        };
        if !Path::new(&settings.install_path).is_dir() {
            continue; // désinstallé, ou disque débranché (géré par le service de bibliothèque)
        }
        games.push(DetectedGame {
            source: SourceId::Riot,
            external_id: dir_name.clone(),
            title: product_title(product, patchline, settings.shortcut_name.as_deref()),
            install_path: Some(settings.install_path),
            executable_path: None,
            launch_method: LaunchMethod::LauncherCli,
            launch_target: client.clone(),
            launch_args: Some(format!("--launch-product={product} --launch-patchline={patchline}")),
            working_dir: None,
            last_played: None,
        });
    }
    games.sort_by(|a, b| a.external_id.cmp(&b.external_id));
    Ok(games)
}

impl GameProvider for RiotProvider {
    fn id(&self) -> SourceId {
        SourceId::Riot
    }

    fn is_available(&self) -> bool {
        riot_data_dir().join("RiotClientInstalls.json").is_file()
    }

    fn scan_installed(&self) -> Result<Vec<DetectedGame>> {
        scan_riot(&riot_data_dir())
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { installed: true, owned: false, playtime: false, achievements: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INSTALLS: &str = include_str!("../../tests/fixtures/RiotClientInstalls.json");
    const SETTINGS: &str = include_str!("../../tests/fixtures/valorant.live.product_settings.yaml");

    #[test]
    fn parses_client_installs() {
        assert_eq!(parse_client_installs(INSTALLS).as_deref(), Some("D:\\Riot Games\\Riot Client\\RiotClientServices.exe"));
        assert_eq!(parse_client_installs("{\"rc_default\": \"C:/RC/RiotClientServices.exe\"}").as_deref(), Some("C:\\RC\\RiotClientServices.exe"));
        assert_eq!(parse_client_installs("{}"), None);
        assert_eq!(parse_client_installs("pas du json"), None);
    }

    #[test]
    fn parses_product_settings() {
        let s = parse_product_settings(SETTINGS).unwrap().unwrap();
        assert_eq!(s, ProductSettings { install_path: "D:\\Riot Games\\VALORANT\\live".into(), shortcut_name: Some("VALORANT".into()) });
        assert_eq!(parse_product_settings("auto_patching_enabled_by_player: false\n").unwrap(), None);
        assert!(parse_product_settings("a: [non fermé").is_err());
    }

    #[test]
    fn titles_follow_products_and_patchlines() {
        assert_eq!(product_title("league_of_legends", "live", None), "League of Legends");
        assert_eq!(product_title("league_of_legends", "pbe", None), "League of Legends (PBE)");
        assert_eq!(product_title("nouveau_jeu", "live", Some("Nouveau Jeu")), "Nouveau Jeu");
    }

    #[test]
    fn scans_installed_products_only() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let install = root.join("VALORANT").join("live");
        std::fs::create_dir_all(&install).unwrap();
        std::fs::write(root.join("RiotClientInstalls.json"), INSTALLS).unwrap();
        let meta = root.join("Metadata");
        let write = |name: &str, body: String| {
            std::fs::create_dir_all(meta.join(name)).unwrap();
            std::fs::write(meta.join(name).join(format!("{name}.product_settings.yaml")), body).unwrap();
        };
        let yaml_path = |p: &Path| p.to_string_lossy().replace('\\', "/");
        write("valorant.live", format!("product_install_full_path: \"{}\"\nshortcut_name: \"VALORANT.lnk\"\n", yaml_path(&install)));
        // Produit dont le dossier n'existe pas (ex. Teamfight Tactics, inclus dans LoL).
        write("teamfighttactics.live", format!("product_install_full_path: \"{}\"\n", yaml_path(&root.join("TFT"))));
        // Produit connu mais jamais installé, et fichier corrompu.
        write("bacon.live", "patching_policy: \"manual\"\n".into());
        write("lion.live", "x: [".into());
        std::fs::create_dir_all(meta.join("league_of_legends.live.game_patch")).unwrap();
        std::fs::create_dir_all(meta.join("Riot Client")).unwrap();

        let games = scan_riot(root).unwrap();
        assert_eq!(games.len(), 1);
        let g = &games[0];
        assert_eq!((g.external_id.as_str(), g.title.as_str()), ("valorant.live", "VALORANT"));
        assert_eq!(g.launch_method, LaunchMethod::LauncherCli);
        assert_eq!(g.launch_target, "D:\\Riot Games\\Riot Client\\RiotClientServices.exe");
        assert_eq!(g.launch_args.as_deref(), Some("--launch-product=valorant --launch-patchline=live"));
        assert!(scan_riot(&root.join("absent")).is_err());
    }
}
