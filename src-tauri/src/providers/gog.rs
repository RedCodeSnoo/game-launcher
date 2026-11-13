//! Provider GOG (§4.3) : registre pour les jeux installés, base locale de GOG Galaxy pour les jeux possédés et les heures.
use super::GameProvider;
use crate::drives::clean_path;
use crate::models::{Capabilities, DetectedGame, GameSource, LaunchMethod, OwnedGame, SourceId};
use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
use winreg::RegKey;

pub struct GogProvider;

const GAMES_KEYS: &[&str] = &["SOFTWARE\\WOW6432Node\\GOG.com\\Games", "SOFTWARE\\GOG.com\\Games"];

/// Valeurs d'une clé `GOG.com\Games\<id>` du registre.
#[derive(Debug, Default, Clone)]
pub struct RegistryGame {
    pub id: String,
    pub name: String,
    pub path: String,
    pub exe: String,
    pub working_dir: String,
    pub launch_param: String,
    /// Non vide pour un DLC : identifiant du jeu de base.
    pub depends_on: String,
}

pub fn to_detected(r: &RegistryGame) -> Option<DetectedGame> {
    if r.id.is_empty() || r.path.is_empty() || r.exe.is_empty() || !r.depends_on.is_empty() {
        return None;
    }
    let install = clean_path(&r.path);
    let exe = clean_path(&r.exe);
    Some(DetectedGame {
        source: SourceId::Gog,
        external_id: r.id.clone(),
        title: if r.name.is_empty() { Path::new(&install).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| r.id.clone()) } else { r.name.clone() },
        install_path: Some(install),
        executable_path: Some(exe.clone()),
        // Exécutable direct : fonctionne même sans GOG Galaxy.
        launch_method: LaunchMethod::Exe,
        launch_target: exe,
        launch_args: Some(r.launch_param.trim().to_string()).filter(|s| !s.is_empty()),
        working_dir: Some(clean_path(&r.working_dir)).filter(|s| !s.is_empty()),
        last_played: None,
    })
}

fn read_registry() -> Vec<RegistryGame> {
    let mut out = Vec::new();
    for key_path in GAMES_KEYS {
        let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(key_path, KEY_READ) else { continue };
        for id in key.enum_keys().flatten() {
            let Ok(sub) = key.open_subkey_with_flags(&id, KEY_READ) else { continue };
            let get = |name: &str| sub.get_value::<String, _>(name).unwrap_or_default();
            out.push(RegistryGame {
                id: id.clone(),
                name: get("gameName"),
                path: get("path"),
                exe: get("exe"),
                working_dir: get("workingDir"),
                launch_param: get("launchParam"),
                depends_on: get("dependsOn"),
            });
        }
    }
    out
}

pub fn galaxy_db_path() -> PathBuf {
    let program_data = std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData".into());
    Path::new(&program_data).join("GOG.com").join("Galaxy").join("storage").join("galaxy-2.0.db")
}

fn title_from_piece(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    v.get("title").and_then(|t| t.as_str()).map(str::trim).filter(|s| !s.is_empty()).map(|s| s.to_string())
}

/// Jeux GOG possédés et temps de jeu lus dans la base de Galaxy (lecture seule).
pub fn read_galaxy_db(conn: &Connection) -> Result<Vec<OwnedGame>> {
    // Titres : pièces de type « title » (ou « originalTitle ») au format JSON {"title": "..."}.
    let mut titles: HashMap<String, String> = HashMap::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT gp.releaseKey, gp.value, t.type FROM GamePieces gp JOIN GamePieceTypes t ON t.id = gp.gamePieceTypeId
                 WHERE t.type IN ('title', 'originalTitle') AND gp.releaseKey LIKE 'gog\\_%' ESCAPE '\\'",
            )
            .context("base Galaxy : table GamePieces inattendue")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))?;
        for row in rows.flatten() {
            let (key, value, kind) = row;
            if let Some(title) = title_from_piece(&value) {
                // « title » (éventuellement modifié par l'utilisateur) prime sur « originalTitle ».
                if kind == "title" || !titles.contains_key(&key) {
                    titles.insert(key, title);
                }
            }
        }
    }
    let optional_map = |sql: &str| -> HashMap<String, i64> {
        let Ok(mut stmt) = conn.prepare(sql) else { return HashMap::new() };
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))).map(|rows| rows.flatten().collect()).unwrap_or_default();
        rows
    };
    let minutes = optional_map("SELECT releaseKey, MAX(minutesInGame) FROM GameTimes GROUP BY releaseKey");
    let last_played = optional_map("SELECT gameReleaseKey, CAST(strftime('%s', MAX(lastPlayedDate)) AS INTEGER) FROM LastPlayedDates WHERE lastPlayedDate IS NOT NULL GROUP BY gameReleaseKey");

    // Jeux possédés : dates d'achat, à défaut les entrées de la bibliothèque.
    let keys: Vec<String> = {
        let try_keys = |sql: &str| -> Option<Vec<String>> {
            let mut stmt = conn.prepare(sql).ok()?;
            let rows: Vec<String> = stmt.query_map([], |r| r.get::<_, String>(0)).ok()?.flatten().collect();
            Some(rows)
        };
        try_keys("SELECT DISTINCT gameReleaseKey FROM ProductPurchaseDates WHERE gameReleaseKey LIKE 'gog\\_%' ESCAPE '\\'")
            .filter(|k| !k.is_empty())
            .or_else(|| try_keys("SELECT DISTINCT releaseKey FROM LibraryReleases WHERE releaseKey LIKE 'gog\\_%' ESCAPE '\\'"))
            .unwrap_or_default()
    };
    let mut games: Vec<OwnedGame> = keys
        .into_iter()
        .filter_map(|key| {
            let id = key.strip_prefix("gog_")?.to_string();
            let title = titles.get(&key)?.clone(); // sans titre : pièce technique (DLC, bonus)
            Some(OwnedGame {
                launch_target: format!("goggalaxy://openGameView/{id}"),
                external_id: id,
                title,
                playtime_minutes: minutes.get(&key).copied(),
                last_played: last_played.get(&key).copied().filter(|t| *t > 0),
                launch_method: LaunchMethod::Uri,
            })
        })
        .collect();
    games.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(games)
}

impl GameProvider for GogProvider {
    fn id(&self) -> SourceId {
        SourceId::Gog
    }

    fn is_available(&self) -> bool {
        galaxy_db_path().is_file() || GAMES_KEYS.iter().any(|k| RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(k, KEY_READ).is_ok())
    }

    fn scan_installed(&self) -> Result<Vec<DetectedGame>> {
        let mut games: Vec<DetectedGame> = Vec::new();
        for r in read_registry() {
            if let Some(g) = to_detected(&r) {
                if Path::new(&g.launch_target).is_file() && !games.iter().any(|x| x.external_id == g.external_id) {
                    games.push(g);
                }
            }
        }
        Ok(games)
    }

    fn fetch_owned(&self) -> Result<Vec<OwnedGame>> {
        let path = galaxy_db_path();
        if !path.is_file() {
            return Ok(Vec::new());
        }
        let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).context("ouverture de la base GOG Galaxy")?;
        read_galaxy_db(&conn)
    }

    fn install_uri(&self, g: &GameSource) -> Option<String> {
        Some(format!("goggalaxy://openGameView/{}", g.external_id))
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { installed: true, owned: true, playtime: true, achievements: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_entry_becomes_a_game() {
        let r = RegistryGame {
            id: "1207658924".into(),
            name: "Unreal Tournament 2004".into(),
            path: "D:/GOG Games/UT2004".into(),
            exe: "D:\\GOG Games\\UT2004\\System\\UT2004.exe".into(),
            working_dir: "D:\\GOG Games\\UT2004\\System".into(),
            launch_param: " -nointro ".into(),
            depends_on: String::new(),
        };
        let g = to_detected(&r).unwrap();
        assert_eq!(g.title, "Unreal Tournament 2004");
        assert_eq!(g.install_path.as_deref(), Some("D:\\GOG Games\\UT2004"));
        assert_eq!(g.launch_method, LaunchMethod::Exe);
        assert_eq!(g.launch_args.as_deref(), Some("-nointro"));
        assert_eq!(g.working_dir.as_deref(), Some("D:\\GOG Games\\UT2004\\System"));
        // Un DLC (dependsOn renseigné) ou une entrée incomplète n'est pas un jeu.
        assert!(to_detected(&RegistryGame { depends_on: "1207658924".into(), ..r.clone() }).is_none());
        assert!(to_detected(&RegistryGame { exe: String::new(), ..r }).is_none());
    }

    #[test]
    fn reads_owned_games_and_playtime_from_galaxy_db() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE GamePieceTypes(id INTEGER PRIMARY KEY, type TEXT);
            CREATE TABLE GamePieces(releaseKey TEXT, gamePieceTypeId INTEGER, userId INTEGER, value TEXT);
            CREATE TABLE GameTimes(userId INTEGER, releaseKey TEXT, minutesInGame INTEGER);
            CREATE TABLE LastPlayedDates(gameReleaseKey TEXT, lastPlayedDate TEXT);
            CREATE TABLE ProductPurchaseDates(gameReleaseKey TEXT, userId INTEGER, purchaseDate TEXT);
            INSERT INTO GamePieceTypes VALUES (1, 'originalTitle'), (2, 'title'), (3, 'meta');
            INSERT INTO GamePieces VALUES
              ('gog_1', 1, 7, '{"title":"The Witcher 3: Wild Hunt"}'),
              ('gog_2', 1, 7, '{"title":"Cyberpunk 2077"}'),
              ('gog_2', 2, 7, '{"title":"Cyberpunk 2077 (renommé)"}'),
              ('gog_2', 3, 7, '{"developers":["CDPR"]}'),
              ('steam_10', 1, 7, '{"title":"Jeu Steam importé"}');
            INSERT INTO GameTimes VALUES (7, 'gog_1', 6120), (7, 'steam_10', 5);
            INSERT INTO LastPlayedDates VALUES ('gog_1', '2026-03-01 20:00:00'), ('gog_2', NULL);
            INSERT INTO ProductPurchaseDates VALUES ('gog_1', 7, '2020-01-01'), ('gog_2', 7, '2021-01-01'), ('gog_3', 7, '2021-01-01'), ('steam_10', 7, '2021-01-01');
            "#,
        )
        .unwrap();
        let games = read_galaxy_db(&conn).unwrap();
        assert_eq!(games.len(), 2, "seuls les jeux GOG ayant un titre sont listés");
        assert_eq!(games[0].title, "Cyberpunk 2077 (renommé)");
        assert_eq!(games[0].playtime_minutes, None);
        assert_eq!(games[1].external_id, "1");
        assert_eq!(games[1].playtime_minutes, Some(6120));
        assert_eq!(games[1].last_played, Some(1772395200));
    }

    #[test]
    fn unexpected_galaxy_schema_is_an_error_not_a_panic() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE Autre(x);").unwrap();
        assert!(read_galaxy_db(&conn).is_err());
    }
}
