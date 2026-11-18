//! Durée de vie des jeux (V1.5, I-05) via IGDB (`game_time_to_beats`), source officielle de Twitch.
//! Facultatif : il faut un identifiant et un secret d'application Twitch (gratuits), rangés dans le coffre Windows.
//! Aucune source non officielle n'est utilisée.
use crate::error::AppError;
use crate::secrets;
use anyhow::Result;
use serde_json::Value;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const CLIENT_ID: &str = "igdb_client_id";
pub const CLIENT_SECRET: &str = "igdb_client_secret";

static TOKEN: Mutex<Option<(String, Instant)>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeToBeat {
    /// Histoire principale (minutes).
    pub main: Option<i64>,
    /// Complétion à 100 % (minutes).
    pub full: Option<i64>,
}

pub fn configured() -> bool {
    secrets::is_set(CLIENT_ID) && secrets::is_set(CLIENT_SECRET)
}

fn post(url: &str, body: &str, client_id: &str, token: &str) -> Result<Value> {
    let resp = ureq::post(url)
        .timeout(Duration::from_secs(20))
        .set("Client-ID", client_id)
        .set("Authorization", &format!("Bearer {token}"))
        .set("Accept", "application/json")
        .send_string(body);
    match resp {
        Ok(r) => Ok(r.into_json()?),
        Err(ureq::Error::Status(401, _)) | Err(ureq::Error::Status(403, _)) => Err(AppError::new("IGDB_BAD_KEY", "").into()),
        Err(ureq::Error::Status(429, _)) => Err(AppError::new("RATE_LIMITED", "").into()),
        Err(ureq::Error::Status(code, _)) => Err(AppError::new("SERVICE_UNAVAILABLE", code.to_string()).into()),
        Err(ureq::Error::Transport(t)) => Err(AppError::new("OFFLINE", format!("{:?}", t.kind())).into()),
    }
}

/// Jeton d'application Twitch (« client credentials »), gardé en mémoire jusqu'à son expiration.
fn token(client_id: &str, secret: &str) -> Result<String> {
    if let Some((t, until)) = TOKEN.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        if Instant::now() < *until {
            return Ok(t.clone());
        }
    }
    let resp = ureq::post("https://id.twitch.tv/oauth2/token")
        .timeout(Duration::from_secs(20))
        .send_form(&[("client_id", client_id), ("client_secret", secret), ("grant_type", "client_credentials")]);
    let body: Value = match resp {
        Ok(r) => r.into_json()?,
        Err(ureq::Error::Status(400 | 401 | 403, _)) => return Err(AppError::new("IGDB_BAD_KEY", "").into()),
        Err(ureq::Error::Status(code, _)) => return Err(AppError::new("SERVICE_UNAVAILABLE", code.to_string()).into()),
        Err(ureq::Error::Transport(t)) => return Err(AppError::new("OFFLINE", format!("{:?}", t.kind())).into()),
    };
    let t = body.get("access_token").and_then(|v| v.as_str()).ok_or_else(|| AppError::new("BAD_RESPONSE", "token"))?.to_string();
    let ttl = body.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(3600).saturating_sub(300);
    *TOKEN.lock().unwrap_or_else(|e| e.into_inner()) = Some((t.clone(), Instant::now() + Duration::from_secs(ttl)));
    Ok(t)
}

/// Échappe un titre pour la requête Apicalypse d'IGDB.
pub fn quote(title: &str) -> String {
    title.replace('\\', " ").replace('"', " ")
}

/// Identifiant IGDB du jeu le plus proche du titre (correspondance exacte du nom normalisé préférée).
pub fn pick_game(results: &Value, title: &str) -> Option<i64> {
    let items = results.as_array()?;
    let wanted = crate::library::normalize_title(title);
    items
        .iter()
        .find(|g| g.get("name").and_then(|n| n.as_str()).map(crate::library::normalize_title).as_deref() == Some(wanted.as_str()))
        .or_else(|| items.first())
        .and_then(|g| g.get("id").and_then(|v| v.as_i64()))
}

/// Durées (secondes chez IGDB) converties en minutes : « normally » = histoire principale, « completely » = 100 %.
pub fn parse_time_to_beat(results: &Value) -> Option<TimeToBeat> {
    let t = results.as_array()?.first()?;
    let minutes = |key: &str| t.get(key).and_then(|v| v.as_i64()).filter(|s| *s > 0).map(|s| (s + 30) / 60);
    let main = minutes("normally").or_else(|| minutes("hastily"));
    let full = minutes("completely");
    (main.is_some() || full.is_some()).then_some(TimeToBeat { main, full })
}

pub fn fetch(title: &str) -> Result<Option<TimeToBeat>> {
    let (Some(id), Some(secret)) = (secrets::get(CLIENT_ID), secrets::get(CLIENT_SECRET)) else { return Ok(None) };
    let token = token(&id, &secret)?;
    let games = post("https://api.igdb.com/v4/games", &format!("search \"{}\"; fields id,name; limit 5;", quote(title)), &id, &token)?;
    let Some(game_id) = pick_game(&games, title) else { return Ok(None) };
    let ttb = post("https://api.igdb.com/v4/game_time_to_beats", &format!("fields hastily,normally,completely; where game_id = {game_id};"), &id, &token)?;
    Ok(parse_time_to_beat(&ttb))
}

/// Vérifie les identifiants (obtention d'un jeton) avant de les enregistrer.
pub fn check_credentials(client_id: &str, secret: &str) -> Result<()> {
    *TOKEN.lock().unwrap_or_else(|e| e.into_inner()) = None;
    token(client_id, secret).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn picks_exact_title_and_converts_durations() {
        let games = json!([{ "id": 1, "name": "Hades II" }, { "id": 2, "name": "HADES" }]);
        assert_eq!(pick_game(&games, "Hades"), Some(2));
        assert_eq!(pick_game(&json!([{ "id": 7, "name": "Autre" }]), "Hades"), Some(7));
        assert_eq!(pick_game(&json!([]), "Hades"), None);
        let ttb = json!([{ "hastily": 54000, "normally": 79200, "completely": 172800 }]);
        assert_eq!(parse_time_to_beat(&ttb), Some(TimeToBeat { main: Some(1320), full: Some(2880) }));
        assert_eq!(parse_time_to_beat(&json!([{ "hastily": 3600 }])), Some(TimeToBeat { main: Some(60), full: None }));
        assert_eq!(parse_time_to_beat(&json!([])), None);
        assert_eq!(quote("Say \"Hi\""), "Say  Hi ");
    }
}
