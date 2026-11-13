//! Accès HTTP minimal. Les messages d'erreur ne contiennent jamais l'URL (elle peut porter une clé API).
use crate::error::AppError;
use anyhow::Result;
use serde_json::Value;
use std::io::Read;
use std::time::Duration;

const USER_AGENT: &str = concat!("GameLauncher/", env!("CARGO_PKG_VERSION"));
const MAX_BODY: u64 = 20 * 1024 * 1024;

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(8)).timeout(Duration::from_secs(25)).user_agent(USER_AGENT).build()
}

/// Réponse JSON, y compris pour les statuts d'erreur qui en portent une (l'API Steam s'en sert).
pub struct JsonResponse {
    pub status: u16,
    pub body: Value,
}

fn transport_error(e: &ureq::Transport) -> AppError {
    AppError::new("OFFLINE", format!("{:?}", e.kind()))
}

pub fn get_json(url: &str, bearer: Option<&str>) -> Result<JsonResponse> {
    let mut req = agent().get(url);
    if let Some(token) = bearer {
        req = req.set("Authorization", &format!("Bearer {token}"));
    }
    match req.call() {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.into_json::<Value>().map_err(|e| AppError::new("BAD_RESPONSE", e.kind().to_string()))?;
            Ok(JsonResponse { status, body })
        }
        Err(ureq::Error::Status(status, resp)) => Ok(JsonResponse { status, body: resp.into_json::<Value>().unwrap_or(Value::Null) }),
        Err(ureq::Error::Transport(t)) => Err(transport_error(&t).into()),
    }
}

/// Télécharge un fichier binaire (image). `None` si la ressource n'existe pas.
pub fn get_bytes(url: &str) -> Result<Option<Vec<u8>>> {
    match agent().get(url).call() {
        Ok(resp) => {
            let mut buf = Vec::new();
            resp.into_reader().take(MAX_BODY).read_to_end(&mut buf).map_err(|e| AppError::new("OFFLINE", e.kind().to_string()))?;
            Ok(Some(buf))
        }
        Err(ureq::Error::Status(_, _)) => Ok(None),
        Err(ureq::Error::Transport(t)) => Err(transport_error(&t).into()),
    }
}

/// Vrai si l'erreur vient de l'absence de réseau (la synchronisation reprendra plus tard).
pub fn is_offline(e: &anyhow::Error) -> bool {
    e.downcast_ref::<AppError>().map(|a| a.code == "OFFLINE").unwrap_or(false)
}
