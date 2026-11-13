//! Provider Battle.net (§4.4) : `Agent/product.db` (protobuf) et lancement par `Battle.net.exe --exec="launch <CODE>"`.
use super::{reg_string, GameProvider};
use crate::drives::clean_path;
use crate::models::{Capabilities, DetectedGame, LaunchMethod, SourceId};
use anyhow::{Context, Result};
use prost::Message;
use std::path::{Path, PathBuf};

pub struct BattleNetProvider;

// Sous-ensemble du schéma de product.db (numéros de champs vérifiés sur un fichier réel).
#[derive(Clone, PartialEq, Message)]
pub struct UserSettings {
    #[prost(string, tag = "1")]
    pub install_path: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct BaseProductState {
    #[prost(bool, tag = "1")]
    pub installed: bool,
    #[prost(bool, tag = "2")]
    pub playable: bool,
}

#[derive(Clone, PartialEq, Message)]
pub struct CachedProductState {
    #[prost(message, optional, tag = "1")]
    pub base_product_state: Option<BaseProductState>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProductInstall {
    #[prost(string, tag = "1")]
    pub uid: String,
    #[prost(string, tag = "2")]
    pub product_code: String,
    #[prost(message, optional, tag = "3")]
    pub settings: Option<UserSettings>,
    #[prost(message, optional, tag = "4")]
    pub cached_product_state: Option<CachedProductState>,
    #[prost(string, tag = "6")]
    pub product_family: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct Database {
    #[prost(message, repeated, tag = "1")]
    pub product_installs: Vec<ProductInstall>,
}

/// Code produit → (titre, code de lancement). À compléter au fil des sorties.
const PRODUCTS: &[(&str, &str, &str)] = &[
    ("wow", "World of Warcraft", "WoW"),
    ("wow_classic", "World of Warcraft Classic", "WoWC"),
    ("wow_classic_era", "World of Warcraft Classic Era", "WoWC"),
    ("pro", "Overwatch", "Pro"),
    ("fenris", "Diablo IV", "Fen"),
    ("osi", "Diablo II: Resurrected", "OSI"),
    ("d3", "Diablo III", "D3"),
    ("anbs", "Diablo Immortal", "ANBS"),
    ("hero", "Heroes of the Storm", "Hero"),
    ("wtcg", "Hearthstone", "WTCG"),
    ("hsb", "Hearthstone", "WTCG"),
    ("s1", "StarCraft", "S1"),
    ("s2", "StarCraft II", "S2"),
    ("w3", "Warcraft III: Reforged", "W3"),
    ("gryphon", "Warcraft Rumble", "GRY"),
    ("rtro", "Blizzard Arcade Collection", "RTRO"),
    ("wlby", "Crash Bandicoot 4: It's About Time", "WLBY"),
    ("viper", "Call of Duty: Black Ops 4", "VIPR"),
    ("odin", "Call of Duty: Modern Warfare", "ODIN"),
    ("lazarus", "Call of Duty: Modern Warfare 2 Campaign Remastered", "LAZR"),
    ("zeus", "Call of Duty: Black Ops Cold War", "ZEUS"),
    ("fore", "Call of Duty: Vanguard", "FORE"),
    ("auks", "Call of Duty", "AUKS"),
];

/// Entrées de product.db qui ne sont pas des jeux.
fn is_launcher_entry(p: &ProductInstall) -> bool {
    matches!(p.uid.as_str(), "agent" | "battle.net") || matches!(p.product_code.as_str(), "agent" | "bna")
}

#[derive(Debug, PartialEq)]
pub struct ParsedDb {
    /// Dossier d'installation de Battle.net lui-même, s'il figure dans la base.
    pub launcher_dir: Option<String>,
    pub games: Vec<BnetGame>,
}

#[derive(Debug, PartialEq)]
pub struct BnetGame {
    pub uid: String,
    pub title: String,
    pub launch_code: String,
    pub install_path: String,
}

fn title_from_folder(path: &str) -> Option<String> {
    Path::new(path).file_name().map(|s| s.to_string_lossy().to_string()).filter(|s| !s.is_empty())
}

pub fn parse_product_db(bytes: &[u8]) -> Result<ParsedDb> {
    let db = Database::decode(bytes).context("product.db : protobuf invalide")?;
    let mut launcher_dir = None;
    let mut games = Vec::new();
    for p in db.product_installs {
        let install_path = p.settings.as_ref().map(|s| clean_path(&s.install_path)).unwrap_or_default();
        if is_launcher_entry(&p) {
            if p.uid == "battle.net" && !install_path.is_empty() {
                launcher_dir = Some(install_path);
            }
            continue;
        }
        if install_path.is_empty() {
            continue;
        }
        // Installation en cours ou cassée : pas encore jouable.
        if let Some(state) = p.cached_product_state.as_ref().and_then(|c| c.base_product_state.as_ref()) {
            if !state.installed && !state.playable {
                continue;
            }
        }
        let known = PRODUCTS.iter().find(|(code, _, _)| *code == p.product_code || *code == p.uid);
        // Variante inconnue d'une famille connue (ex. « wowf ») : titre et code de la famille.
        let family = PRODUCTS.iter().find(|(code, _, _)| !p.product_family.is_empty() && *code == p.product_family);
        let (title, launch_code) = match (known, family) {
            (Some((_, title, code)), _) => (title.to_string(), code.to_string()),
            (None, Some((_, title, code))) => (title_from_folder(&install_path).unwrap_or_else(|| title.to_string()), code.to_string()),
            (None, None) => (title_from_folder(&install_path).unwrap_or_else(|| p.product_code.clone()), p.product_code.to_uppercase()),
        };
        games.push(BnetGame { uid: p.uid, title, launch_code, install_path });
    }
    Ok(ParsedDb { launcher_dir, games })
}

fn product_db_path() -> PathBuf {
    let program_data = std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData".into());
    Path::new(&program_data).join("Battle.net").join("Agent").join("product.db")
}

/// Emplacement de `Battle.net.exe` : product.db, puis registre de désinstallation, puis emplacement par défaut.
fn launcher_exe(from_db: Option<&str>) -> Option<String> {
    let candidates = [
        from_db.map(|d| d.to_string()),
        reg_string(winreg::enums::HKEY_LOCAL_MACHINE, "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Battle.net", "InstallLocation"),
        Some("C:\\Program Files (x86)\\Battle.net".to_string()),
    ];
    candidates.into_iter().flatten().map(|dir| format!("{}\\Battle.net.exe", clean_path(&dir))).find(|exe| Path::new(exe).is_file())
}

pub fn to_detected(parsed: ParsedDb, launcher: &str, require_dir: bool) -> Vec<DetectedGame> {
    parsed
        .games
        .into_iter()
        .filter(|g| !require_dir || Path::new(&g.install_path).is_dir())
        .map(|g| DetectedGame {
            source: SourceId::Battlenet,
            external_id: g.uid,
            title: g.title,
            install_path: Some(g.install_path),
            executable_path: None,
            launch_method: LaunchMethod::LauncherCli,
            launch_target: launcher.to_string(),
            launch_args: Some(format!("--exec=\"launch {}\"", g.launch_code)),
            working_dir: None,
            last_played: None,
        })
        .collect()
}

impl GameProvider for BattleNetProvider {
    fn id(&self) -> SourceId {
        SourceId::Battlenet
    }

    fn is_available(&self) -> bool {
        product_db_path().is_file()
    }

    fn scan_installed(&self) -> Result<Vec<DetectedGame>> {
        let bytes = std::fs::read(product_db_path()).context("lecture de product.db")?;
        let parsed = parse_product_db(&bytes)?;
        let launcher = launcher_exe(parsed.launcher_dir.as_deref()).unwrap_or_else(|| "C:\\Program Files (x86)\\Battle.net\\Battle.net.exe".to_string());
        Ok(to_detected(parsed, &launcher, true))
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { installed: true, owned: false, playtime: false, achievements: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fichier produit indépendamment de `prost` (encodage protobuf écrit à la main, cf. scripts/make-bnet-fixture.py).
    const FIXTURE: &[u8] = include_bytes!("../../tests/fixtures/battlenet_product.db");

    #[test]
    fn parses_product_db() {
        let parsed = parse_product_db(FIXTURE).unwrap();
        assert_eq!(parsed.launcher_dir.as_deref(), Some("C:\\Program Files (x86)\\Battle.net"));
        let summary: Vec<(&str, &str, &str, &str)> = parsed.games.iter().map(|g| (g.uid.as_str(), g.title.as_str(), g.launch_code.as_str(), g.install_path.as_str())).collect();
        assert_eq!(
            summary,
            vec![
                ("wow", "World of Warcraft", "WoW", "D:\\Jeux\\World of Warcraft"),
                ("pro", "Overwatch", "Pro", "D:\\Jeux\\Overwatch"),
                // Variante inconnue de la famille « wow » : titre du dossier, code de la famille.
                ("wowf", "WoW Forever", "WoW", "E:\\WoW Forever"),
                // Produit totalement inconnu : titre du dossier, code en majuscules.
                ("newgame", "Nouveau Jeu", "NEWGAME", "D:\\Jeux\\Nouveau Jeu"),
            ],
            "l'agent, Battle.net et l'installation non jouable sont exclus"
        );
    }

    #[test]
    fn builds_launch_command() {
        let parsed = parse_product_db(FIXTURE).unwrap();
        let games = to_detected(parsed, "C:\\Program Files (x86)\\Battle.net\\Battle.net.exe", false);
        assert_eq!(games[0].launch_args.as_deref(), Some("--exec=\"launch WoW\""));
        assert_eq!(crate::launch::split_args(games[0].launch_args.as_deref().unwrap()), vec!["--exec=launch WoW"]);
        assert_eq!(games[0].launch_method, LaunchMethod::LauncherCli);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_product_db(&[0xFF, 0xFF, 0xFF, 0xFF, 0x01]).is_err());
        assert_eq!(parse_product_db(&[]).unwrap(), ParsedDb { launcher_dir: None, games: vec![] });
    }
}
