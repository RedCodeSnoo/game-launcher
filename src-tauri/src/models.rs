//! Types partagés entre les providers, la base et l'interface.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceId {
    Steam,
    Epic,
    Gog,
    Battlenet,
    Riot,
    Ubisoft,
    Ea,
    Xbox,
    Local,
}

impl SourceId {
    pub const ALL: [SourceId; 9] = [
        SourceId::Steam,
        SourceId::Epic,
        SourceId::Gog,
        SourceId::Battlenet,
        SourceId::Riot,
        SourceId::Ubisoft,
        SourceId::Ea,
        SourceId::Xbox,
        SourceId::Local,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            SourceId::Steam => "steam",
            SourceId::Epic => "epic",
            SourceId::Gog => "gog",
            SourceId::Battlenet => "battlenet",
            SourceId::Riot => "riot",
            SourceId::Ubisoft => "ubisoft",
            SourceId::Ea => "ea",
            SourceId::Xbox => "xbox",
            SourceId::Local => "local",
        }
    }

    pub fn parse(s: &str) -> Option<SourceId> {
        SourceId::ALL.iter().copied().find(|x| x.as_str() == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchMethod {
    Uri,
    LauncherCli,
    Exe,
}

impl LaunchMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            LaunchMethod::Uri => "uri",
            LaunchMethod::LauncherCli => "launcher_cli",
            LaunchMethod::Exe => "exe",
        }
    }
    pub fn parse(s: &str) -> LaunchMethod {
        match s {
            "uri" => LaunchMethod::Uri,
            "launcher_cli" => LaunchMethod::LauncherCli,
            _ => LaunchMethod::Exe,
        }
    }
}

/// Jeu installé trouvé par un provider.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedGame {
    pub source: SourceId,
    pub external_id: String,
    pub title: String,
    pub install_path: Option<String>,
    pub executable_path: Option<String>,
    pub launch_method: LaunchMethod,
    pub launch_target: String,
    pub launch_args: Option<String>,
    pub working_dir: Option<String>,
    /// Dernier lancement connu de la source (secondes Unix).
    pub last_played: Option<i64>,
}

/// Jeu possédé (installé ou non) rapporté par l'API ou la base d'une source.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedGame {
    pub external_id: String,
    pub title: String,
    pub playtime_minutes: Option<i64>,
    pub last_played: Option<i64>,
    pub launch_method: LaunchMethod,
    pub launch_target: String,
}

/// Ligne `game_sources`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSource {
    pub id: String,
    pub game_id: String,
    pub source: SourceId,
    pub external_id: String,
    pub installed: bool,
    pub install_path: Option<String>,
    pub volume_serial: Option<String>,
    pub executable_path: Option<String>,
    pub launch_method: LaunchMethod,
    pub launch_target: String,
    pub launch_args: Option<String>,
    pub working_dir: Option<String>,
    pub source_playtime_minutes: Option<i64>,
    pub last_played_source: Option<String>,
    pub available: bool,
    pub owned: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Achievement {
    pub api_name: String,
    pub name: String,
    pub description: String,
    pub icon_unlocked: Option<String>,
    pub icon_locked: Option<String>,
    pub hidden: bool,
    pub unlocked: bool,
    pub unlocked_at: Option<String>,
    pub global_percent: Option<f64>,
}

/// Ce qu'une source sait faire (cf. §2.1 du cahier des charges).
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub installed: bool,
    pub owned: bool,
    pub playtime: bool,
    pub achievements: bool,
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn unix_to_iso(secs: i64) -> Option<String> {
    if secs <= 0 {
        return None;
    }
    chrono::DateTime::from_timestamp(secs, 0)
        .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}
