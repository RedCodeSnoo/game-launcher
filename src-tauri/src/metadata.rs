//! MetadataService : jaquette, bannière, logo et fiche descriptive, mis en cache localement (F-MET-01/02/05).
//!
//! Ordre des sources d'images pour un jeu associé à un appid Steam :
//! cache local du client Steam (hors ligne) → CDN Steam → SteamGridDB (si une clé est fournie) → image d'en-tête.
//! Pour les autres jeux : SteamGridDB par recherche du nom, sinon association à une fiche Steam de même nom.
use crate::error::AppError;
use crate::library::normalize_title;
use crate::models::SourceId;
use crate::state::{AppState, Emitter};
use crate::{http, secrets};
use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const SGDB: &str = "https://www.steamgriddb.com/api/v2";
const CDN: &str = "https://cdn.cloudflare.steamstatic.com/steam/apps";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageKind {
    Cover,
    Hero,
    Logo,
}

impl ImageKind {
    pub const ALL: [ImageKind; 3] = [ImageKind::Cover, ImageKind::Hero, ImageKind::Logo];

    fn column(&self) -> &'static str {
        match self {
            ImageKind::Cover => "cover_path",
            ImageKind::Hero => "hero_path",
            ImageKind::Logo => "logo_path",
        }
    }
    fn slug(&self) -> &'static str {
        match self {
            ImageKind::Cover => "cover",
            ImageKind::Hero => "hero",
            ImageKind::Logo => "logo",
        }
    }
    /// Noms de fichier dans le cache du client Steam (ancien puis nouveau format).
    fn steam_cache_names(&self) -> &'static [&'static str] {
        match self {
            ImageKind::Cover => &["library_600x900.jpg", "library_capsule.jpg"],
            ImageKind::Hero => &["library_hero.jpg"],
            ImageKind::Logo => &["logo.png"],
        }
    }
    fn cdn_name(&self) -> &'static str {
        match self {
            ImageKind::Cover => "library_600x900.jpg",
            ImageKind::Hero => "library_hero.jpg",
            ImageKind::Logo => "logo.png",
        }
    }
    fn sgdb_endpoint(&self) -> &'static str {
        match self {
            ImageKind::Cover => "grids",
            ImageKind::Hero => "heroes",
            ImageKind::Logo => "logos",
        }
    }
}

pub fn images_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("images")
}

fn extension_of(bytes: &[u8], fallback: &str) -> &'static str {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => "png",
        [0xFF, 0xD8, ..] => "jpg",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => "webp",
        _ if fallback.eq_ignore_ascii_case("png") => "png",
        _ if fallback.eq_ignore_ascii_case("webp") => "webp",
        _ => "jpg",
    }
}

/// Écrit une image dans le cache et renvoie son chemin. Les anciennes versions du même type sont supprimées.
pub fn store_image(data_dir: &Path, game_id: &str, kind: ImageKind, bytes: &[u8], ext_hint: &str) -> Result<String> {
    let dir = images_dir(data_dir);
    std::fs::create_dir_all(&dir)?;
    let prefix = format!("{game_id}_{}", kind.slug());
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            if e.file_name().to_string_lossy().starts_with(&prefix) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    // Horodatage dans le nom : l'interface ne garde pas l'ancienne image en cache.
    let stamp = chrono::Utc::now().timestamp_millis();
    let path = dir.join(format!("{prefix}_{stamp}.{}", extension_of(bytes, ext_hint)));
    std::fs::write(&path, bytes)?;
    Ok(path.to_string_lossy().to_string())
}

/// Cherche une image dans le cache du client Steam : `<Steam>/appcache/librarycache/<appid>/[<hash>/]<nom>`.
pub fn find_in_steam_cache(steam_root: &Path, appid: &str, kind: ImageKind) -> Option<PathBuf> {
    let base = steam_root.join("appcache").join("librarycache");
    let dir = base.join(appid);
    for name in kind.steam_cache_names() {
        let direct = dir.join(name);
        if direct.is_file() {
            return Some(direct);
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let nested = e.path().join(name);
                if nested.is_file() {
                    return Some(nested);
                }
            }
        }
        // Très ancien format : `<appid>_library_600x900.jpg` à la racine du cache.
        let flat = base.join(format!("{appid}_{name}"));
        if flat.is_file() {
            return Some(flat);
        }
    }
    None
}

fn first_sgdb_url(body: &Value) -> Option<String> {
    body.get("data")?.as_array()?.iter().find_map(|d| d.get("url").and_then(Value::as_str)).map(|s| s.to_string())
}

fn sgdb_image_url(key: &str, kind: ImageKind, target: &str) -> Result<Option<String>> {
    let dims = if kind == ImageKind::Cover { "?dimensions=600x900" } else { "" };
    let resp = http::get_json(&format!("{SGDB}/{}/{target}{dims}", kind.sgdb_endpoint()), Some(key))?;
    match resp.status {
        200 => Ok(first_sgdb_url(&resp.body)),
        401 | 403 => Err(AppError::new("SGDB_BAD_KEY", "").into()),
        _ => Ok(None),
    }
}

/// Identifiant SteamGridDB du jeu le plus proche du nom donné.
pub fn sgdb_search(key: &str, title: &str) -> Result<Option<i64>> {
    let resp = http::get_json(&format!("{SGDB}/search/autocomplete/{}", urlencoding::encode(title)), Some(key))?;
    match resp.status {
        200 => Ok(pick_sgdb_match(&resp.body, title)),
        401 | 403 => Err(AppError::new("SGDB_BAD_KEY", "").into()),
        _ => Ok(None),
    }
}

/// Préfère une correspondance exacte du titre normalisé, sinon le premier résultat.
pub fn pick_sgdb_match(body: &Value, title: &str) -> Option<i64> {
    let list = body.get("data")?.as_array()?;
    let wanted = normalize_title(title);
    list.iter()
        .find(|d| d.get("name").and_then(Value::as_str).map(normalize_title).as_deref() == Some(wanted.as_str()))
        .or_else(|| list.first())
        .and_then(|d| d.get("id").and_then(Value::as_i64))
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoreMatch {
    pub appid: i64,
    pub name: String,
    pub image: Option<String>,
}

/// Résultats de la recherche de la boutique Steam (sans clé) : sert à associer un jeu local à sa fiche (F-MET-03).
pub fn parse_store_search(body: &Value) -> Vec<StoreMatch> {
    let Some(items) = body.get("items").and_then(Value::as_array) else { return Vec::new() };
    items
        .iter()
        .filter(|i| i.get("type").and_then(Value::as_str).map_or(true, |t| t == "app"))
        .filter_map(|i| {
            Some(StoreMatch {
                appid: i.get("id")?.as_i64()?,
                name: i.get("name")?.as_str()?.to_string(),
                image: i.get("tiny_image").and_then(Value::as_str).map(|s| s.to_string()),
            })
        })
        .collect()
}

pub fn store_search(term: &str) -> Result<Vec<StoreMatch>> {
    let resp = http::get_json(&format!("https://store.steampowered.com/api/storesearch/?term={}&l=french&cc=FR", urlencoding::encode(term)), None)?;
    Ok(parse_store_search(&resp.body))
}

#[derive(Debug, Default, PartialEq)]
pub struct AppDetails {
    pub description: Option<String>,
    pub genres: Vec<String>,
    pub developer: Option<String>,
    pub release_date: Option<String>,
    pub header_image: Option<String>,
}

fn decode_entities(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&#39;", "'").replace("&nbsp;", " ")
}

/// Réponse de `store.steampowered.com/api/appdetails`.
pub fn parse_app_details(body: &Value, appid: &str) -> Option<AppDetails> {
    let entry = body.get(appid)?;
    if entry.get("success").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    let data = entry.get("data")?;
    let text = |k: &str| data.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(decode_entities);
    Some(AppDetails {
        description: text("short_description"),
        genres: data
            .get("genres")
            .and_then(Value::as_array)
            .map(|g| g.iter().filter_map(|x| x.get("description").and_then(Value::as_str)).map(|s| s.to_string()).collect())
            .unwrap_or_default(),
        developer: data.get("developers").and_then(Value::as_array).map(|d| d.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ")).filter(|s| !s.is_empty()),
        release_date: data.pointer("/release_date/date").and_then(Value::as_str).map(|s| s.to_string()).filter(|s| !s.is_empty()),
        header_image: text("header_image"),
    })
}

pub fn fetch_app_details(appid: &str) -> Result<Option<AppDetails>> {
    let resp = http::get_json(&format!("https://store.steampowered.com/api/appdetails?appids={appid}&l=french&cc=fr"), None)?;
    if resp.status == 429 {
        return Err(AppError::new("RATE_LIMITED", "boutique Steam").into());
    }
    Ok(parse_app_details(&resp.body, appid))
}

struct GameMeta {
    id: String,
    title: String,
    steam_appid: Option<String>,
    has: [bool; 3],
    custom_cover: bool,
}

fn load_meta(conn: &rusqlite::Connection, game_id: &str) -> Result<Option<GameMeta>> {
    let row = conn
        .query_row(
            "SELECT title, meta_steam_appid, cover_path, hero_path, logo_path, custom_cover FROM games WHERE id = ?1",
            params![game_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<i64>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, bool>(5)?,
                ))
            },
        )
        .optional()?;
    let Some((title, meta_appid, cover, hero, logo, custom_cover)) = row else { return Ok(None) };
    let steam_source: Option<String> = conn
        .query_row("SELECT external_id FROM game_sources WHERE game_id = ?1 AND source = ?2 LIMIT 1", params![game_id, SourceId::Steam.as_str()], |r| r.get(0))
        .optional()?;
    let exists = |p: &Option<String>| p.as_deref().map(|p| Path::new(p).is_file()).unwrap_or(false);
    Ok(Some(GameMeta {
        id: game_id.to_string(),
        title,
        steam_appid: steam_source.or(meta_appid.map(|a| a.to_string())),
        has: [exists(&cover) || custom_cover, exists(&hero), exists(&logo)],
        custom_cover,
    }))
}

/// Récupère les images manquantes d'un jeu. Renvoie vrai si au moins une image a été ajoutée.
/// Une erreur réseau est renvoyée telle quelle : le jeu sera retenté plus tard.
pub fn fetch_images(state: &AppState, game_id: &str) -> Result<bool> {
    let Some(mut meta) = load_meta(&state.conn(), game_id)? else { return Ok(false) };
    let sgdb_key = secrets::get(secrets::STEAMGRIDDB_API_KEY);
    let steam_root = crate::providers::steam::steam_path();
    let mut changed = false;
    let mut offline: Option<anyhow::Error> = None;

    // Jeu Epic : le cache du catalogue du launcher fournit visuels et description, sans compte ni clé.
    let epic_app: Option<String> = state
        .conn()
        .query_row("SELECT external_id FROM game_sources WHERE game_id = ?1 AND source = ?2 LIMIT 1", params![game_id, SourceId::Epic.as_str()], |r| r.get(0))
        .optional()?;
    let epic_entry = epic_app.and_then(|app| crate::providers::epic::catalog().remove(&app));
    if let Some(e) = &epic_entry {
        state.conn().execute(
            "UPDATE games SET description = COALESCE(description, ?2), developer = COALESCE(developer, ?3) WHERE id = ?1",
            params![meta.id, e.description, e.developer],
        )?;
    }

    // Jeu sans appid : tentative d'association automatique à une fiche Steam de même nom exact.
    if meta.steam_appid.is_none() && epic_entry.is_none() && sgdb_key.is_none() && !meta.has.iter().all(|h| *h) {
        match store_search(&meta.title) {
            Ok(results) => {
                let wanted = normalize_title(&meta.title);
                if let Some(m) = results.iter().find(|m| normalize_title(&m.name) == wanted) {
                    state.conn().execute("UPDATE games SET meta_steam_appid = ?2 WHERE id = ?1", params![meta.id, m.appid])?;
                    meta.steam_appid = Some(m.appid.to_string());
                }
            }
            Err(e) if http::is_offline(&e) => offline = Some(e),
            Err(e) => log::warn!("recherche de fiche pour « {} » : {e:#}", meta.title),
        }
    }

    let mut sgdb_id: Option<Option<i64>> = None;
    for (i, kind) in ImageKind::ALL.into_iter().enumerate() {
        if meta.has[i] || (kind == ImageKind::Cover && meta.custom_cover) {
            continue;
        }
        let mut bytes: Option<(Vec<u8>, String)> = None;
        if let Some(appid) = &meta.steam_appid {
            if let Some(path) = steam_root.as_deref().and_then(|root| find_in_steam_cache(root, appid, kind)) {
                if let Ok(data) = std::fs::read(&path) {
                    bytes = Some((data, path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default()));
                }
            }
            if bytes.is_none() && offline.is_none() {
                match http::get_bytes(&format!("{CDN}/{appid}/{}", kind.cdn_name())) {
                    Ok(Some(data)) => bytes = Some((data, kind.cdn_name().rsplit('.').next().unwrap_or("jpg").to_string())),
                    Ok(None) => {}
                    Err(e) => offline = Some(e),
                }
            }
        }
        if bytes.is_none() && offline.is_none() {
            let url = epic_entry.as_ref().and_then(|e| match kind {
                ImageKind::Cover => e.cover_url.clone(),
                ImageKind::Hero => e.hero_url.clone(),
                ImageKind::Logo => e.logo_url.clone(),
            });
            if let Some(url) = url {
                match http::get_bytes(&url) {
                    Ok(Some(data)) => bytes = Some((data, "jpg".to_string())),
                    Ok(None) => {}
                    Err(e) => offline = Some(e),
                }
            }
        }
        if bytes.is_none() && offline.is_none() {
            if let Some(key) = &sgdb_key {
                let target = match &meta.steam_appid {
                    Some(appid) => Some(format!("steam/{appid}")),
                    None => {
                        let id = *sgdb_id.get_or_insert_with(|| sgdb_search(key, &meta.title).unwrap_or(None));
                        id.map(|id| format!("game/{id}"))
                    }
                };
                if let Some(target) = target {
                    match sgdb_image_url(key, kind, &target).and_then(|url| url.map(|u| http::get_bytes(&u).map(|b| b.map(|b| (b, u)))).transpose()) {
                        Ok(Some(Some((data, url)))) => bytes = Some((data, url.rsplit('.').next().unwrap_or("jpg").to_string())),
                        Ok(_) => {}
                        Err(e) if http::is_offline(&e) => offline = Some(e),
                        Err(e) => log::warn!("SteamGridDB pour « {} » : {e:#}", meta.title),
                    }
                }
            }
        }
        if let Some((data, ext)) = bytes {
            let path = store_image(&state.data_dir, &meta.id, kind, &data, &ext)?;
            state.conn().execute(&format!("UPDATE games SET {} = ?2 WHERE id = ?1", kind.column()), params![meta.id, path])?;
            meta.has[i] = true;
            changed = true;
        }
    }
    if let Some(Some(id)) = sgdb_id {
        state.conn().execute("UPDATE games SET steamgrid_id = ?2 WHERE id = ?1", params![meta.id, id])?;
    }
    match offline {
        Some(e) => Err(e),
        None => {
            state.conn().execute("UPDATE games SET images_checked = 1 WHERE id = ?1", params![meta.id])?;
            Ok(changed)
        }
    }
}

/// Récupère description, genres, développeur et date de sortie (F-MET-02) pour un jeu associé à un appid Steam.
pub fn fetch_details(state: &AppState, game_id: &str) -> Result<bool> {
    let Some(meta) = load_meta(&state.conn(), game_id)? else { return Ok(false) };
    let Some(appid) = meta.steam_appid else {
        state.conn().execute("UPDATE games SET details_checked = 1 WHERE id = ?1", params![game_id])?;
        return Ok(false);
    };
    let details = fetch_app_details(&appid)?;
    let mut changed = false;
    if let Some(d) = &details {
        let genres = if d.genres.is_empty() { None } else { Some(d.genres.join(", ")) };
        state.conn().execute(
            "UPDATE games SET description = COALESCE(?2, description), genres = COALESCE(?3, genres), developer = COALESCE(?4, developer),
                release_date = COALESCE(?5, release_date) WHERE id = ?1",
            params![game_id, d.description, genres, d.developer, d.release_date],
        )?;
        changed = true;
        // Dernier recours pour la jaquette : l'image d'en-tête de la boutique.
        if !meta.has[0] {
            if let Some(url) = &d.header_image {
                if let Some(data) = http::get_bytes(url)? {
                    let path = store_image(&state.data_dir, game_id, ImageKind::Cover, &data, "jpg")?;
                    state.conn().execute("UPDATE games SET cover_path = ?2 WHERE id = ?1", params![game_id, path])?;
                }
            }
        }
    }
    state.conn().execute("UPDATE games SET details_checked = 1 WHERE id = ?1", params![game_id])?;
    Ok(changed)
}

/// Remplace la jaquette par une image choisie par l'utilisateur (F-MET-04).
pub fn set_custom_cover(state: &AppState, game_id: &str, source_file: &Path) -> Result<()> {
    let bytes = std::fs::read(source_file).map_err(|_| AppError::new("FILE_NOT_FOUND", source_file.to_string_lossy().to_string()))?;
    if bytes.len() > 20 * 1024 * 1024 || !matches!(extension_of(&bytes, "bin"), "png" | "webp") && !bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(AppError::new("NOT_AN_IMAGE", source_file.to_string_lossy().to_string()).into());
    }
    let ext = source_file.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    let path = store_image(&state.data_dir, game_id, ImageKind::Cover, &bytes, &ext)?;
    state.conn().execute("UPDATE games SET cover_path = ?2, custom_cover = 1 WHERE id = ?1", params![game_id, path])?;
    Ok(())
}

/// Associe un jeu à une fiche Steam choisie par l'utilisateur et recharge ses métadonnées (F-MET-03).
pub fn apply_store_match(state: &AppState, game_id: &str, appid: i64) -> Result<()> {
    state.conn().execute(
        "UPDATE games SET meta_steam_appid = ?2, images_checked = 0, details_checked = 0, hero_path = NULL, logo_path = NULL,
            cover_path = CASE WHEN custom_cover = 1 THEN cover_path ELSE NULL END WHERE id = ?1",
        params![game_id, appid],
    )?;
    fetch_images(state, game_id)?;
    fetch_details(state, game_id)?;
    Ok(())
}

static RUNNING: AtomicBool = AtomicBool::new(false);

/// Tâche de fond : complète images puis fiches de tous les jeux qui n'ont pas encore été traités.
pub fn run_background(state: &AppState, emit: &Emitter) {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let pending = |column: &str| -> Vec<String> {
        let conn = state.conn();
        // Jeux installés d'abord, puis les plus récemment joués.
        let sql = format!(
            "SELECT g.id FROM games g WHERE g.{column} = 0
             ORDER BY (SELECT MAX(installed) FROM game_sources s WHERE s.game_id = g.id) DESC,
                      (SELECT MAX(last_played_source) FROM game_sources s WHERE s.game_id = g.id) DESC"
        );
        let Ok(mut stmt) = conn.prepare(&sql) else { return Vec::new() };
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map(|rows| rows.flatten().collect()).unwrap_or_default();
        rows
    };

    let mut since_emit = 0;
    for game_id in pending("images_checked") {
        match fetch_images(state, &game_id) {
            Ok(true) => {
                since_emit += 1;
                if since_emit >= 12 {
                    emit("library-updated", json!(null));
                    since_emit = 0;
                }
            }
            Ok(false) => {}
            Err(e) if http::is_offline(&e) => {
                log::info!("métadonnées : hors ligne, les images manquantes seront récupérées plus tard");
                break;
            }
            Err(e) => log::warn!("images de {game_id} : {e:#}"),
        }
    }
    if since_emit > 0 {
        emit("library-updated", json!(null));
    }

    let mut fetched = 0;
    for game_id in pending("details_checked") {
        match fetch_details(state, &game_id) {
            Ok(changed) => {
                if changed {
                    fetched += 1;
                    if fetched % 10 == 0 {
                        emit("library-updated", json!(null));
                    }
                    // La boutique Steam limite à ~200 requêtes par 5 minutes.
                    std::thread::sleep(Duration::from_millis(1600));
                }
            }
            Err(e) if http::is_offline(&e) => break,
            Err(e) => {
                let app: AppError = e.into();
                if app.code == "RATE_LIMITED" {
                    log::info!("métadonnées : limite de requêtes atteinte, reprise à la prochaine synchronisation");
                    break;
                }
                log::warn!("fiche de {game_id} : {app}");
            }
        }
    }
    if fetched > 0 {
        emit("library-updated", json!(null));
    }
    RUNNING.store(false, Ordering::SeqCst);
}

/// Remet à zéro les marqueurs pour retenter les jeux sans image (ex. après l'ajout d'une clé SteamGridDB).
pub fn reset_missing(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute("UPDATE games SET images_checked = 0 WHERE cover_path IS NULL OR hero_path IS NULL OR logo_path IS NULL", [])?;
    conn.execute("UPDATE games SET details_checked = 0 WHERE description IS NULL", [])?;
    Ok(())
}

pub fn game_ids_with_source(conn: &rusqlite::Connection, source: SourceId) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT game_id FROM game_sources WHERE source = ?1")?;
    let rows = stmt.query_map(params![source.as_str()], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_images_in_both_steam_cache_layouts() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path().join("appcache/librarycache");
        std::fs::create_dir_all(cache.join("108600")).unwrap();
        std::fs::write(cache.join("108600/library_600x900.jpg"), b"x").unwrap();
        std::fs::write(cache.join("108600/logo.png"), b"x").unwrap();
        std::fs::create_dir_all(cache.join("4115450/abc123")).unwrap();
        std::fs::create_dir_all(cache.join("4115450/def456")).unwrap();
        std::fs::write(cache.join("4115450/abc123/library_capsule.jpg"), b"x").unwrap();
        std::fs::write(cache.join("4115450/def456/library_hero.jpg"), b"x").unwrap();

        assert!(find_in_steam_cache(tmp.path(), "108600", ImageKind::Cover).unwrap().ends_with("108600/library_600x900.jpg"));
        assert!(find_in_steam_cache(tmp.path(), "108600", ImageKind::Logo).is_some());
        assert!(find_in_steam_cache(tmp.path(), "108600", ImageKind::Hero).is_none());
        assert!(find_in_steam_cache(tmp.path(), "4115450", ImageKind::Cover).unwrap().ends_with("abc123/library_capsule.jpg"));
        assert!(find_in_steam_cache(tmp.path(), "4115450", ImageKind::Hero).unwrap().ends_with("def456/library_hero.jpg"));
        assert!(find_in_steam_cache(tmp.path(), "999", ImageKind::Cover).is_none());
    }

    #[test]
    fn stores_images_and_replaces_previous_version() {
        let tmp = tempfile::tempdir().unwrap();
        let first = store_image(tmp.path(), "g1", ImageKind::Cover, &[0xFF, 0xD8, 0xFF, 0xE0], "bin").unwrap();
        assert!(first.ends_with(".jpg"));
        std::thread::sleep(Duration::from_millis(5));
        let second = store_image(tmp.path(), "g1", ImageKind::Cover, &[0x89, b'P', b'N', b'G', 1], "jpg").unwrap();
        assert!(second.ends_with(".png"), "le type réel prime sur l'extension annoncée");
        assert!(!Path::new(&first).exists());
        assert!(Path::new(&second).exists());
        // Une image d'un autre type n'est pas touchée.
        let hero = store_image(tmp.path(), "g1", ImageKind::Hero, &[0xFF, 0xD8], "jpg").unwrap();
        assert!(Path::new(&second).exists() && Path::new(&hero).exists());
    }

    #[test]
    fn parses_store_search_and_app_details() {
        let search = json!({ "total": 2, "items": [
            { "type": "app", "name": "Hades II", "id": 1145350, "tiny_image": "https://example.invalid/a.jpg" },
            { "type": "app", "name": "Hades", "id": 1145360 }
        ]});
        let results = parse_store_search(&search);
        assert_eq!(results.len(), 2);
        assert_eq!(results[1], StoreMatch { appid: 1145360, name: "Hades".into(), image: None });

        let details = json!({ "1145360": { "success": true, "data": {
            "short_description": "D&eacute;fiez le dieu des morts &quot;Hades&quot; &amp; fuyez.",
            "developers": ["Supergiant Games"],
            "genres": [{ "id": "1", "description": "Action" }, { "id": "3", "description": "RPG" }],
            "release_date": { "coming_soon": false, "date": "17 sept. 2020" },
            "header_image": "https://example.invalid/header.jpg"
        }}});
        let d = parse_app_details(&details, "1145360").unwrap();
        assert_eq!(d.description.as_deref(), Some("D&eacute;fiez le dieu des morts \"Hades\" & fuyez."));
        assert_eq!(d.genres, vec!["Action", "RPG"]);
        assert_eq!(d.developer.as_deref(), Some("Supergiant Games"));
        assert_eq!(d.release_date.as_deref(), Some("17 sept. 2020"));
        assert_eq!(parse_app_details(&json!({ "5": { "success": false } }), "5"), None);
    }

    #[test]
    fn picks_exact_steamgriddb_match_first() {
        let body = json!({ "success": true, "data": [{ "id": 1, "name": "Hades II" }, { "id": 2, "name": "Hades" }] });
        assert_eq!(pick_sgdb_match(&body, "HADES"), Some(2));
        assert_eq!(pick_sgdb_match(&body, "Inconnu"), Some(1));
        assert_eq!(pick_sgdb_match(&json!({ "data": [] }), "x"), None);
        assert_eq!(first_sgdb_url(&json!({ "data": [{ "url": "https://example.invalid/g.png" }] })).as_deref(), Some("https://example.invalid/g.png"));
    }
}
