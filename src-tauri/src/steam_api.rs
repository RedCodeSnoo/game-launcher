//! API Steam Web (§4.1) : jeux possédés, heures, succès, rareté. Analyse séparée des appels réseau pour être testée.
use crate::error::AppError;
use crate::http;
use crate::models::{unix_to_iso, Achievement, LaunchMethod, OwnedGame};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

const API: &str = "https://api.steampowered.com";

#[derive(Debug, Clone)]
pub struct SteamCreds {
    pub key: String,
    pub steam_id: String,
}

pub fn is_valid_steam_id(id: &str) -> bool {
    id.len() == 17 && id.starts_with("7656") && id.chars().all(|c| c.is_ascii_digit())
}

fn check_status(status: u16) -> Result<()> {
    match status {
        200 => Ok(()),
        401 | 403 => Err(AppError::new("STEAM_BAD_KEY", "").into()),
        429 => Err(AppError::new("RATE_LIMITED", "Steam").into()),
        s if s >= 500 => Err(AppError::new("SERVICE_UNAVAILABLE", format!("Steam (HTTP {s})")).into()),
        s => Err(AppError::new("BAD_RESPONSE", format!("Steam (HTTP {s})")).into()),
    }
}

/// `IPlayerService/GetOwnedGames` → jeux possédés avec temps de jeu et dernier lancement.
pub fn parse_owned_games(body: &Value) -> Result<Vec<OwnedGame>> {
    let response = body.get("response").ok_or_else(|| AppError::new("BAD_RESPONSE", "Steam : champ « response » absent"))?;
    // Profil privé ou « Détails des jeux » non publics : la réponse est vide.
    let Some(games) = response.get("games").and_then(Value::as_array) else {
        if response.get("game_count").and_then(Value::as_i64) == Some(0) {
            return Ok(Vec::new());
        }
        return Err(AppError::new("STEAM_PRIVATE", "").into());
    };
    Ok(games
        .iter()
        .filter_map(|g| {
            let appid = g.get("appid")?.as_i64()?;
            Some(OwnedGame {
                external_id: appid.to_string(),
                title: g.get("name").and_then(Value::as_str).map(|s| s.to_string()).unwrap_or_else(|| format!("App {appid}")),
                playtime_minutes: g.get("playtime_forever").and_then(Value::as_i64),
                last_played: g.get("rtime_last_played").and_then(Value::as_i64).filter(|t| *t > 0),
                launch_method: LaunchMethod::Uri,
                launch_target: format!("steam://rungameid/{appid}"),
            })
        })
        .collect())
}

/// Succès du joueur : nom d'API → (débloqué, date). `None` si le jeu n'a pas de succès.
#[derive(Debug, Default, PartialEq)]
pub struct PlayerAchievements {
    pub items: Vec<PlayerAchievement>,
}

#[derive(Debug, PartialEq)]
pub struct PlayerAchievement {
    pub api_name: String,
    pub unlocked: bool,
    pub unlocked_at: Option<i64>,
    pub name: Option<String>,
    pub description: Option<String>,
}

pub fn parse_player_achievements(status: u16, body: &Value) -> Result<Option<PlayerAchievements>> {
    let stats = body.get("playerstats");
    if let Some(error) = stats.and_then(|s| s.get("error")).and_then(Value::as_str) {
        let lower = error.to_lowercase();
        if lower.contains("no stats") || lower.contains("not found") {
            return Ok(None);
        }
        if lower.contains("not public") || lower.contains("private") {
            return Err(AppError::new("STEAM_PRIVATE", "").into());
        }
        return Err(AppError::new("BAD_RESPONSE", format!("Steam : {error}")).into());
    }
    check_status(status)?;
    let Some(list) = stats.and_then(|s| s.get("achievements")).and_then(Value::as_array) else {
        return Ok(None);
    };
    let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(|s| s.to_string());
    Ok(Some(PlayerAchievements {
        items: list
            .iter()
            .filter_map(|a| {
                Some(PlayerAchievement {
                    api_name: a.get("apiname")?.as_str()?.to_string(),
                    unlocked: a.get("achieved").and_then(Value::as_i64).unwrap_or(0) != 0,
                    unlocked_at: a.get("unlocktime").and_then(Value::as_i64).filter(|t| *t > 0),
                    name: text(a, "name"),
                    description: text(a, "description"),
                })
            })
            .collect(),
    }))
}

#[derive(Debug, PartialEq)]
pub struct SchemaAchievement {
    pub api_name: String,
    pub name: String,
    pub description: String,
    pub hidden: bool,
    pub icon: Option<String>,
    pub icon_gray: Option<String>,
}

/// `ISteamUserStats/GetSchemaForGame` → noms, descriptions, icônes.
pub fn parse_schema(body: &Value) -> Vec<SchemaAchievement> {
    let Some(list) = body.pointer("/game/availableGameStats/achievements").and_then(Value::as_array) else {
        return Vec::new();
    };
    let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(|s| s.to_string());
    list.iter()
        .filter_map(|a| {
            let api_name = text(a, "name")?;
            Some(SchemaAchievement {
                name: text(a, "displayName").unwrap_or_else(|| api_name.clone()),
                description: text(a, "description").unwrap_or_default(),
                hidden: a.get("hidden").and_then(Value::as_i64).unwrap_or(0) != 0,
                icon: text(a, "icon"),
                icon_gray: text(a, "icongray"),
                api_name,
            })
        })
        .collect()
}

/// `GetGlobalAchievementPercentagesForApp` → rareté ; `percent` est un nombre ou une chaîne selon les versions.
pub fn parse_global_percentages(body: &Value) -> HashMap<String, f64> {
    let Some(list) = body.pointer("/achievementpercentages/achievements").and_then(Value::as_array) else {
        return HashMap::new();
    };
    list.iter()
        .filter_map(|a| {
            let name = a.get("name")?.as_str()?.to_string();
            let percent = match a.get("percent")? {
                Value::Number(n) => n.as_f64()?,
                Value::String(s) => s.parse::<f64>().ok()?,
                _ => return None,
            };
            Some((name, percent))
        })
        .collect()
}

/// Assemble schéma, progression du joueur et rareté.
pub fn merge_achievements(schema: Vec<SchemaAchievement>, player: PlayerAchievements, global: &HashMap<String, f64>) -> Vec<Achievement> {
    let mut by_name: HashMap<String, PlayerAchievement> = player.items.into_iter().map(|p| (p.api_name.clone(), p)).collect();
    let mut out: Vec<Achievement> = schema
        .into_iter()
        .map(|s| {
            let p = by_name.remove(&s.api_name);
            Achievement {
                global_percent: global.get(&s.api_name).copied(),
                unlocked: p.as_ref().map(|p| p.unlocked).unwrap_or(false),
                unlocked_at: p.as_ref().and_then(|p| p.unlocked_at).and_then(unix_to_iso),
                name: s.name,
                description: s.description,
                icon_unlocked: s.icon,
                icon_locked: s.icon_gray,
                hidden: s.hidden,
                api_name: s.api_name,
            }
        })
        .collect();
    // Succès connus du joueur mais absents du schéma (schéma indisponible).
    let mut rest: Vec<PlayerAchievement> = by_name.into_values().collect();
    rest.sort_by(|a, b| a.api_name.cmp(&b.api_name));
    for p in rest {
        out.push(Achievement {
            global_percent: global.get(&p.api_name).copied(),
            name: p.name.unwrap_or_else(|| p.api_name.clone()),
            description: p.description.unwrap_or_default(),
            icon_unlocked: None,
            icon_locked: None,
            hidden: false,
            unlocked: p.unlocked,
            unlocked_at: p.unlocked_at.and_then(unix_to_iso),
            api_name: p.api_name,
        });
    }
    out
}

pub fn fetch_owned_games(creds: &SteamCreds) -> Result<Vec<OwnedGame>> {
    let url = format!(
        "{API}/IPlayerService/GetOwnedGames/v1/?key={}&steamid={}&include_appinfo=1&include_played_free_games=1&format=json",
        urlencoding::encode(&creds.key),
        creds.steam_id
    );
    let resp = http::get_json(&url, None)?;
    check_status(resp.status)?;
    parse_owned_games(&resp.body)
}

/// Succès complets d'un jeu. `Ok(None)` si le jeu n'a pas de succès.
pub fn fetch_achievements(creds: &SteamCreds, appid: &str) -> Result<Option<Vec<Achievement>>> {
    let key = urlencoding::encode(&creds.key);
    let player = http::get_json(&format!("{API}/ISteamUserStats/GetPlayerAchievements/v1/?appid={appid}&key={key}&steamid={}&l=french", creds.steam_id), None)?;
    let Some(player) = parse_player_achievements(player.status, &player.body)? else {
        return Ok(None);
    };
    // Schéma et rareté : utiles mais non bloquants.
    let schema = http::get_json(&format!("{API}/ISteamUserStats/GetSchemaForGame/v2/?key={key}&appid={appid}&l=french"), None).map(|r| parse_schema(&r.body)).unwrap_or_default();
    let global = http::get_json(&format!("{API}/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/?gameid={appid}"), None)
        .map(|r| parse_global_percentages(&r.body))
        .unwrap_or_default();
    let merged = merge_achievements(schema, player, &global);
    Ok(if merged.is_empty() { None } else { Some(merged) })
}

/// Résout une URL personnalisée (« vanity ») en SteamID64.
pub fn resolve_vanity(key: &str, vanity: &str) -> Result<Option<String>> {
    let url = format!("{API}/ISteamUser/ResolveVanityURL/v1/?key={}&vanityurl={}", urlencoding::encode(key), urlencoding::encode(vanity));
    let resp = http::get_json(&url, None)?;
    check_status(resp.status)?;
    Ok(resp.body.pointer("/response/steamid").and_then(Value::as_str).map(|s| s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture(name: &str) -> Value {
        let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn code(e: anyhow::Error) -> String {
        AppError::from(e).code
    }

    #[test]
    fn validates_steam_ids() {
        assert!(is_valid_steam_id("76561198000000001"));
        assert!(!is_valid_steam_id("7656119800000000"));
        assert!(!is_valid_steam_id("12345678901234567"));
        assert!(!is_valid_steam_id("7656119800000000x"));
    }

    #[test]
    fn parses_owned_games() {
        let games = parse_owned_games(&fixture("steam_owned_games.json")).unwrap();
        assert_eq!(games.len(), 3);
        assert_eq!(games[0].external_id, "108600");
        assert_eq!(games[0].title, "Project Zomboid");
        assert_eq!(games[0].playtime_minutes, Some(4321));
        assert_eq!(games[0].last_played, Some(1792771130));
        assert_eq!(games[0].launch_target, "steam://rungameid/108600");
        assert_eq!(games[2].last_played, None, "jamais lancé");
    }

    #[test]
    fn private_profile_is_reported() {
        assert_eq!(code(parse_owned_games(&json!({ "response": {} })).unwrap_err()), "STEAM_PRIVATE");
        assert!(parse_owned_games(&json!({ "response": { "game_count": 0 } })).unwrap().is_empty());
        assert_eq!(code(parse_owned_games(&json!({})).unwrap_err()), "BAD_RESPONSE");
    }

    #[test]
    fn parses_player_achievements_and_error_cases() {
        let parsed = parse_player_achievements(200, &fixture("steam_player_achievements.json")).unwrap().unwrap();
        assert_eq!(parsed.items.len(), 3);
        assert!(parsed.items[0].unlocked);
        assert_eq!(parsed.items[0].unlocked_at, Some(1700000000));
        assert!(!parsed.items[1].unlocked);
        assert_eq!(parsed.items[1].unlocked_at, None);

        let no_stats = json!({ "playerstats": { "error": "Requested app has no stats", "success": false } });
        assert_eq!(parse_player_achievements(400, &no_stats).unwrap(), None);
        let private = json!({ "playerstats": { "error": "Profile is not public", "success": false } });
        assert_eq!(code(parse_player_achievements(403, &private).unwrap_err()), "STEAM_PRIVATE");
        assert_eq!(code(parse_player_achievements(403, &Value::Null).unwrap_err()), "STEAM_BAD_KEY");
        assert_eq!(parse_player_achievements(200, &json!({ "playerstats": { "success": true } })).unwrap(), None);
    }

    #[test]
    fn merges_schema_progress_and_rarity() {
        let schema = parse_schema(&fixture("steam_schema.json"));
        assert_eq!(schema.len(), 3);
        assert!(schema[2].hidden);
        let global = parse_global_percentages(&fixture("steam_global_percentages.json"));
        assert_eq!(global.get("ACH_FIRST"), Some(&82.5));
        assert_eq!(global.get("ACH_SECRET"), Some(&1.2), "pourcentage fourni sous forme de chaîne");

        let player = parse_player_achievements(200, &fixture("steam_player_achievements.json")).unwrap().unwrap();
        let merged = merge_achievements(schema, player, &global);
        assert_eq!(merged.len(), 3);
        let first = &merged[0];
        assert_eq!((first.api_name.as_str(), first.name.as_str(), first.unlocked), ("ACH_FIRST", "Premier pas", true));
        assert_eq!(first.unlocked_at.as_deref(), Some("2023-11-14T22:13:20Z"));
        assert_eq!(first.global_percent, Some(82.5));
        assert!(first.icon_unlocked.as_deref().unwrap().ends_with("first.jpg"));
        assert!(!merged[1].unlocked);
        assert!(merged[2].hidden && merged[2].unlocked);
    }

    #[test]
    fn merge_without_schema_keeps_player_data() {
        let player = PlayerAchievements {
            items: vec![PlayerAchievement { api_name: "A".into(), unlocked: true, unlocked_at: None, name: Some("Nom".into()), description: None }],
        };
        let merged = merge_achievements(Vec::new(), player, &HashMap::new());
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "Nom");
        assert!(parse_schema(&json!({ "game": {} })).is_empty());
    }
}
