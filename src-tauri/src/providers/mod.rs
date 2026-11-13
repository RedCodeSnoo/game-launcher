//! Providers : une source de jeux = un module isolé implémentant `GameProvider` (§7.3).
use crate::launch;
use crate::models::{Achievement, Capabilities, DetectedGame, GameSource, OwnedGame, SourceId};
use anyhow::Result;

pub mod steam;

pub trait GameProvider: Send + Sync {
    fn id(&self) -> SourceId;
    /// Le launcher est-il installé sur cette machine ?
    fn is_available(&self) -> bool;
    fn scan_installed(&self) -> Result<Vec<DetectedGame>>;
    /// Jeux possédés (vide si la source ne le permet pas).
    fn fetch_owned(&self) -> Result<Vec<OwnedGame>> {
        Ok(Vec::new())
    }
    fn fetch_playtime(&self, _g: &GameSource) -> Result<Option<i64>> {
        Ok(None)
    }
    fn fetch_achievements(&self, _g: &GameSource) -> Result<Option<Vec<Achievement>>> {
        Ok(None)
    }
    fn launch(&self, g: &GameSource) -> Result<()> {
        launch::launch_source(g)
    }
    /// URI ouvrant le launcher d'origine sur l'installation du jeu (F-BIB-03).
    fn install_uri(&self, _g: &GameSource) -> Option<String> {
        None
    }
    fn capabilities(&self) -> Capabilities;
}

/// Paramètres non secrets dont les providers ont besoin.
#[derive(Debug, Clone, Default)]
pub struct ProviderConfig {
    pub steam_id: Option<String>,
}

/// Tous les providers, dans l'ordre de priorité du cahier des charges.
pub fn all(cfg: &ProviderConfig) -> Vec<Box<dyn GameProvider>> {
    vec![Box::new(steam::SteamProvider::new(cfg.steam_id.clone()))]
}

pub fn by_id(cfg: &ProviderConfig, id: SourceId) -> Option<Box<dyn GameProvider>> {
    all(cfg).into_iter().find(|p| p.id() == id)
}

/// Lecture d'une valeur chaîne du registre (HKLM ou HKCU), sans jamais écrire.
pub fn reg_string(hive: winreg::HKEY, key: &str, value: &str) -> Option<String> {
    use winreg::enums::KEY_READ;
    winreg::RegKey::predef(hive).open_subkey_with_flags(key, KEY_READ).ok()?.get_value::<String, _>(value).ok()
}
