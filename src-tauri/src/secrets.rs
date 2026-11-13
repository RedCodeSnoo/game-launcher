//! Clés API et jetons : stockés dans le Gestionnaire d'identification Windows (§6.2), jamais en clair.
use anyhow::{Context, Result};

const SERVICE: &str = "GameLauncher";

pub const STEAM_API_KEY: &str = "steam_api_key";
pub const STEAMGRIDDB_API_KEY: &str = "steamgriddb_api_key";

fn entry(name: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, name).context("accès au coffre Windows")
}

pub fn get(name: &str) -> Option<String> {
    entry(name).ok()?.get_password().ok().filter(|s| !s.is_empty())
}

pub fn set(name: &str, value: &str) -> Result<()> {
    entry(name)?.set_password(value).context("écriture dans le coffre Windows")
}

pub fn delete(name: &str) -> Result<()> {
    match entry(name)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e).context("suppression dans le coffre Windows"),
    }
}

pub fn is_set(name: &str) -> bool {
    get(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_through_windows_vault() {
        let name = "test_secret_roundtrip";
        set(name, "valeur-secrète").unwrap();
        assert_eq!(get(name).as_deref(), Some("valeur-secrète"));
        assert!(is_set(name));
        delete(name).unwrap();
        assert_eq!(get(name), None);
        delete(name).unwrap(); // supprimer une entrée absente n'est pas une erreur
    }
}
