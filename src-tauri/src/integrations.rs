//! Intégrations en ligne (V1.5) : jeux gratuits de la semaine sur Epic et actualités Steam (API officielle
//! `ISteamNews`). Données publiques, sans compte ni clé. Hors ligne : la dernière liste connue reste affichée.
use crate::db;
use crate::http;
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const EPIC_URL: &str = "https://store-site-backend-static.ak.epicgames.com/freeGamesPromotions?locale=fr&country=FR&allowCountries=FR";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FreeGame {
    pub id: String,
    pub title: String,
    pub url: String,
    pub image: Option<String>,
    pub start: String,
    pub end: String,
    /// Gratuit maintenant (sinon : bientôt).
    pub current: bool,
}

/// Jeux gratuits (réduction de 100 %) en cours et à venir, d'après la réponse des promotions Epic.
pub fn parse_free_games(body: &Value, now: DateTime<Utc>) -> Vec<FreeGame> {
    let elements = body.pointer("/data/Catalog/searchStore/elements").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut out = Vec::new();
    for e in elements {
        let title = e.get("title").and_then(|v| v.as_str()).unwrap_or_default().trim().replace('\u{a0}', " ");
        if title.is_empty() {
            continue;
        }
        let offers = |key: &str| -> Vec<Value> {
            e.pointer(&format!("/promotions/{key}"))
                .and_then(|v| v.as_array())
                .map(|groups| groups.iter().flat_map(|g| g.get("promotionalOffers").and_then(|o| o.as_array()).cloned().unwrap_or_default()).collect())
                .unwrap_or_default()
        };
        let free = |o: &Value| o.pointer("/discountSetting/discountPercentage").and_then(|v| v.as_i64()) == Some(0);
        let (offer, current) = match offers("promotionalOffers").into_iter().find(free) {
            Some(o) => (o, true),
            None => match offers("upcomingPromotionalOffers").into_iter().find(free) {
                Some(o) => (o, false),
                None => continue,
            },
        };
        let start = offer.get("startDate").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let end = offer.get("endDate").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        // Une offre « en cours » déjà terminée (cache ancien) n'est plus affichée.
        if DateTime::parse_from_rfc3339(&end).map(|d| d.with_timezone(&Utc) < now).unwrap_or(false) {
            continue;
        }
        let slug = e
            .pointer("/catalogNs/mappings")
            .and_then(|m| m.as_array())
            .and_then(|m| m.iter().find_map(|x| x.get("pageSlug").and_then(|s| s.as_str())))
            .or_else(|| e.get("productSlug").and_then(|s| s.as_str()).map(|s| s.trim_end_matches("/home")))
            .unwrap_or_default();
        let url = if slug.is_empty() { "https://store.epicgames.com/fr/free-games".to_string() } else { format!("https://store.epicgames.com/fr/p/{slug}") };
        let images = e.get("keyImages").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let image = ["OfferImageWide", "DieselStoreFrontWide", "Thumbnail", "OfferImageTall"]
            .iter()
            .find_map(|kind| images.iter().find(|i| i.get("type").and_then(|t| t.as_str()) == Some(kind)).and_then(|i| i.get("url").and_then(|u| u.as_str())))
            .filter(|u| u.starts_with("https://"))
            .map(|u| u.to_string());
        let id = e.get("id").and_then(|v| v.as_str()).unwrap_or(&title).to_string();
        out.push(FreeGame { id, title, url, image, start, end, current });
    }
    out.sort_by(|a, b| b.current.cmp(&a.current).then(a.start.cmp(&b.start)));
    out
}

/// Dernière liste connue (mise en cache dans les réglages pour le hors-ligne), offres terminées retirées.
pub fn cached_free_games(conn: &Connection) -> (Vec<FreeGame>, Option<String>) {
    let list = db::get_setting(conn, "epic_free_cache").and_then(|v| serde_json::from_str::<Vec<FreeGame>>(&v).ok()).unwrap_or_default();
    let now = Utc::now();
    let list = list.into_iter().filter(|g| DateTime::parse_from_rfc3339(&g.end).map(|d| d.with_timezone(&Utc) >= now).unwrap_or(true)).collect();
    (list, db::get_setting(conn, "epic_free_checked"))
}

/// Interroge Epic ; renvoie la liste et les jeux nouvellement gratuits (pour la notification).
pub fn refresh_free_games(conn_lock: &std::sync::Mutex<Connection>) -> Result<(Vec<FreeGame>, Vec<FreeGame>)> {
    let resp = http::get_json(EPIC_URL, None)?;
    if resp.status != 200 {
        anyhow::bail!(crate::error::AppError::new("SERVICE_UNAVAILABLE", resp.status.to_string()));
    }
    let list = parse_free_games(&resp.body, Utc::now());
    let conn = conn_lock.lock().unwrap_or_else(|e| e.into_inner());
    let known: Vec<String> = db::get_setting(&conn, "epic_free_known").map(|v| v.split(',').map(|s| s.to_string()).collect()).unwrap_or_default();
    let first = db::get_setting(&conn, "epic_free_known").is_none();
    let fresh: Vec<FreeGame> = list.iter().filter(|g| g.current && !known.contains(&g.id)).cloned().collect();
    db::set_setting(&conn, "epic_free_cache", &serde_json::to_string(&list)?)?;
    db::set_setting(&conn, "epic_free_checked", &crate::models::now_iso())?;
    let ids: Vec<String> = known.into_iter().chain(list.iter().filter(|g| g.current).map(|g| g.id.clone())).collect::<std::collections::BTreeSet<_>>().into_iter().rev().take(60).collect();
    db::set_setting(&conn, "epic_free_known", &ids.join(","))?;
    Ok((list, if first { Vec::new() } else { fresh }))
}

// ---------------------------------------------------------------------------
// Actualités Steam
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewsItem {
    pub id: String,
    pub appid: String,
    pub game_id: Option<String>,
    pub game_title: Option<String>,
    pub title: String,
    pub url: String,
    pub date: String,
    pub feed: String,
    pub excerpt: String,
}

/// Texte lisible à partir du contenu d'une actualité (BBCode, HTML, liens d'images Steam).
pub fn clean_excerpt(raw: &str, max: usize) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut depth_angle = 0;
    let mut depth_square = 0;
    for c in raw.chars() {
        match c {
            '<' => depth_angle += 1,
            '>' if depth_angle > 0 => depth_angle -= 1,
            '[' => depth_square += 1,
            ']' if depth_square > 0 => depth_square -= 1,
            _ if depth_angle == 0 && depth_square == 0 => out.push(c),
            _ => {}
        }
    }
    let text = out
        .split_whitespace()
        .filter(|w| !w.starts_with("{STEAM_CLAN_IMAGE}") && !w.starts_with("http://") && !w.starts_with("https://"))
        .collect::<Vec<_>>()
        .join(" ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");
    if text.chars().count() <= max {
        return text;
    }
    let cut: String = text.chars().take(max).collect();
    format!("{}…", cut.rsplit_once(' ').map(|(a, _)| a).unwrap_or(&cut))
}

pub fn parse_news(body: &Value) -> Vec<NewsItem> {
    let items = body.pointer("/appnews/newsitems").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    items
        .iter()
        .filter_map(|n| {
            let date = n.get("date").and_then(|d| d.as_i64())?;
            Some(NewsItem {
                id: n.get("gid").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                appid: n.get("appid").map(|v| v.to_string().trim_matches('"').to_string()).unwrap_or_default(),
                game_id: None,
                game_title: None,
                title: n.get("title").and_then(|v| v.as_str()).unwrap_or_default().trim().to_string(),
                url: n.get("url").and_then(|v| v.as_str()).filter(|u| u.starts_with("https://")).unwrap_or_default().to_string(),
                date: crate::models::unix_to_iso(date)?,
                feed: n.get("feedlabel").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                excerpt: clean_excerpt(n.get("contents").and_then(|v| v.as_str()).unwrap_or_default(), 260),
            })
        })
        .filter(|n| !n.title.is_empty())
        .collect()
}

static NEWS_CACHE: Mutex<Option<HashMap<String, (Instant, Vec<NewsItem>)>>> = Mutex::new(None);
const NEWS_TTL: Duration = Duration::from_secs(3600);

/// Actualités d'un jeu Steam (cache d'une heure en mémoire).
pub fn steam_news(appid: &str, count: usize) -> Result<Vec<NewsItem>> {
    if !appid.chars().all(|c| c.is_ascii_digit()) || appid.is_empty() {
        return Ok(Vec::new());
    }
    {
        let cache = NEWS_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, items)) = cache.as_ref().and_then(|c| c.get(appid)) {
            if at.elapsed() < NEWS_TTL {
                return Ok(items.iter().take(count).cloned().collect());
            }
        }
    }
    let url = format!("https://api.steampowered.com/ISteamNews/GetNewsForApp/v2/?appid={appid}&count=8&maxlength=600&format=json");
    let resp = http::get_json(&url, None)?;
    let items = if resp.status == 200 { parse_news(&resp.body) } else { Vec::new() };
    NEWS_CACHE.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(appid.to_string(), (Instant::now(), items.clone()));
    Ok(items.into_iter().take(count).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epic_free_games_current_and_upcoming() {
        let body: Value = serde_json::from_str(include_str!("../tests/fixtures/epic_free_games.json")).unwrap();
        let now = DateTime::parse_from_rfc3339("2026-11-17T12:00:00Z").unwrap().with_timezone(&Utc);
        let list = parse_free_games(&body, now);
        assert_eq!(list.iter().map(|g| (g.title.as_str(), g.current)).collect::<Vec<_>>(), vec![("Knightica", true), ("Mark of the Deep", false)]);
        let k = &list[0];
        assert_eq!(k.url, "https://store.epicgames.com/fr/p/knightica-3be45c");
        assert!(k.image.as_deref().unwrap().starts_with("https://"));
        assert_eq!(k.end, "2026-11-19T16:00:00.000Z");
        // Une semaine plus tard, l'offre en cours est terminée.
        let later = DateTime::parse_from_rfc3339("2026-11-20T12:00:00Z").unwrap().with_timezone(&Utc);
        assert!(parse_free_games(&body, later).iter().all(|g| g.title != "Knightica"));
        assert!(parse_free_games(&Value::Null, now).is_empty());
    }

    #[test]
    fn steam_news_is_parsed_and_cleaned() {
        let body: Value = serde_json::json!({ "appnews": { "appid": 1145360, "newsitems": [
            { "gid": "1", "title": " Patch 1.2 ", "url": "https://store.steampowered.com/news/1", "date": 1781628906, "feedlabel": "Community Announcements", "appid": 1145360,
              "contents": "[h1]Nouveautés[/h1] {STEAM_CLAN_IMAGE}/abc.png Le <b>boss</b> final &amp; ses amis https://exemple.test arrivent." },
            { "gid": "2", "title": "", "url": "", "date": 1, "contents": "" }
        ]}});
        let news = parse_news(&body);
        assert_eq!(news.len(), 1);
        assert_eq!(news[0].title, "Patch 1.2");
        assert_eq!(news[0].appid, "1145360");
        assert_eq!(news[0].excerpt, "Nouveautés Le boss final & ses amis arrivent.");
        assert!(news[0].date.starts_with("2026-"));
        assert_eq!(clean_excerpt("un deux trois quatre", 9), "un deux…");
    }

    /// Services réels (réseau) : `cargo test live_integrations -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_integrations() {
        let conn = std::sync::Mutex::new(db::open_in_memory().unwrap());
        let (list, _) = refresh_free_games(&conn).unwrap();
        println!("Epic : {:?}", list.iter().map(|g| (&g.title, g.current, &g.end)).collect::<Vec<_>>());
        assert!(!list.is_empty());
        let news = steam_news("1145360", 3).unwrap();
        println!("Actualités Hades : {:?}", news.iter().map(|n| (&n.date, &n.title)).collect::<Vec<_>>());
        assert!(!news.is_empty());
        let shots = crate::screenshots::find("Lethal Company", &[(crate::models::SourceId::Steam, "1966720".into())], crate::providers::steam::steam_path().as_deref(), crate::screenshots::videos_dir().as_deref(), &[]);
        println!("Captures Lethal Company : {} (dossiers {:?})", shots.0.len(), shots.1.len());
    }
}

