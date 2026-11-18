//! « Récap de l'année » (V2.0, B-03) : chiffres d'une année de jeu et profil de joueur.
//! Seules les sessions réelles de l'année comptent (une base saisie à la main n'a pas de date).
use crate::activity::{self, Config};
use crate::dashboard::{self, Badge};
use crate::library::{self, GameView};
use crate::models::SourceId;
use anyhow::Result;
use chrono::{Datelike, NaiveDate, TimeZone};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecapGame {
    pub game_id: String,
    pub title: String,
    pub cover_path: Option<String>,
    pub hero_path: Option<String>,
    pub seconds: i64,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: &'static str,
    pub title: &'static str,
    pub emoji: &'static str,
    pub description: String,
    /// Traits secondaires (titre court).
    pub traits: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recap {
    pub year: i32,
    /// Le récap est disponible (à partir du 1er décembre pour l'année en cours).
    pub available: bool,
    pub total_seconds: i64,
    pub days_played: usize,
    pub sessions: usize,
    pub games_played: usize,
    pub top_games: Vec<RecapGame>,
    pub best_streak: u32,
    pub best_streak_start: Option<String>,
    pub best_streak_end: Option<String>,
    pub record_day: Option<(String, i64)>,
    pub record_day_games: Vec<String>,
    pub favorite_source: Option<(SourceId, i64)>,
    pub source_share: f64,
    pub monthly: Vec<i64>,
    pub hourly: Vec<i64>,
    pub longest_session: Option<(String, i64)>,
    pub night_share: f64,
    pub weekend_share: f64,
    pub badges: Vec<Badge>,
    pub profile: Profile,
}

/// Données de comportement utilisées pour le profil de joueur.
#[derive(Debug, Clone, Copy, Default)]
pub struct Traits {
    pub total_hours: f64,
    pub games_played: usize,
    pub avg_session_hours: f64,
    pub night_share: f64,
    pub weekend_share: f64,
    pub best_streak: u32,
    pub top_share: f64,
    pub days_played: usize,
}

/// Profil de joueur : le trait le plus marqué l'emporte, les autres marqués deviennent des traits secondaires.
pub fn profile(t: &Traits) -> Profile {
    let mut scores: Vec<(f64, &'static str, &'static str, &'static str, String)> = vec![
        (t.night_share / 0.30, "night-owl", "Noctambule", "🦉", format!("{:.0} % de votre temps de jeu après 22 h.", t.night_share * 100.0)),
        (t.avg_session_hours / 2.5, "marathon", "Marathonien", "🏃", format!("Des sessions de {:.1} h en moyenne.", t.avg_session_hours)),
        (t.games_played as f64 / 15.0, "explorer", "Touche-à-tout", "🧭", format!("{} jeux différents lancés cette année.", t.games_played)),
        (t.best_streak as f64 / 21.0, "loyal", "Fidèle", "🔥", format!("Une série de {} jours joués d'affilée.", t.best_streak)),
        (t.weekend_share / 0.55, "weekend", "Guerrier du week-end", "🛡️", format!("{:.0} % de vos parties le samedi et le dimanche.", t.weekend_share * 100.0)),
        (t.top_share / 0.6, "devoted", "Monogame", "💍", format!("{:.0} % du temps sur un seul jeu.", t.top_share * 100.0)),
        (t.days_played as f64 / 200.0, "daily", "Joueur du quotidien", "📅", format!("{} jours joués dans l'année.", t.days_played)),
    ];
    scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    if t.total_hours < 1.0 || scores[0].0 < 0.5 {
        return Profile { id: "casual", title: "Joueur tranquille", emoji: "🌱", description: "Quelques parties, sans pression : le jeu reste un plaisir.".into(), traits: Vec::new() };
    }
    let (_, id, title, emoji, description) = scores.remove(0);
    let traits = scores.iter().filter(|s| s.0 >= 1.0).take(2).map(|s| s.2).collect();
    Profile { id, title, emoji, description, traits }
}

/// Le récap est proposé à partir du 1er décembre pour l'année en cours ; les années passées le sont toujours.
pub fn is_available(year: i32, today: NaiveDate) -> bool {
    year < today.year() || (year == today.year() && today.month() == 12)
}

pub fn compute<Tz: TimeZone>(conn: &Connection, year: i32, tz: &Tz, today: NaiveDate) -> Result<Recap> {
    activity::ensure_fresh(conn, tz)?;
    let cfg = Config::load(conn);
    let start = NaiveDate::from_ymd_opt(year, 1, 1).unwrap_or(today);
    let end = NaiveDate::from_ymd_opt(year, 12, 31).unwrap_or(today);
    let games = library::list_games(conn)?;
    let by_id: HashMap<&str, &GameView> = games.iter().map(|g| (g.id.as_str(), g)).collect();

    // Résumés journaliers de l'année.
    let mut stmt = conn.prepare("SELECT day, game_id, seconds FROM activity_days WHERE day >= ?1 AND day <= ?2")?;
    let rows: Vec<(String, String, i64)> = stmt.query_map([start.to_string(), end.to_string()], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<rusqlite::Result<_>>()?;
    let mut per_game: HashMap<String, i64> = HashMap::new();
    let mut per_day: BTreeMap<NaiveDate, i64> = BTreeMap::new();
    let mut day_games: HashMap<NaiveDate, Vec<(String, i64)>> = HashMap::new();
    let mut monthly = vec![0i64; 12];
    for (day, game, secs) in &rows {
        let Ok(d) = NaiveDate::parse_from_str(day, "%Y-%m-%d") else { continue };
        *per_game.entry(game.clone()).or_default() += secs;
        *per_day.entry(d).or_default() += secs;
        day_games.entry(d).or_default().push((game.clone(), *secs));
        monthly[d.month0() as usize] += secs;
    }
    let total_seconds: i64 = per_game.values().sum();
    let played: BTreeSet<NaiveDate> = activity::played_days(&per_day, cfg.min_minutes);
    let streaks = activity::streaks(&played, end.min(today), false);

    let mut top: Vec<(&String, &i64)> = per_game.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1));
    let top_games: Vec<RecapGame> = top
        .iter()
        .take(5)
        .filter_map(|(id, s)| by_id.get(id.as_str()).map(|g| RecapGame { game_id: g.id.clone(), title: g.title.clone(), cover_path: g.cover_path.clone(), hero_path: g.hero_path.clone(), seconds: **s }))
        .collect();
    let top_share = top.first().map(|(_, s)| **s as f64 / total_seconds.max(1) as f64).unwrap_or(0.0);

    let record = per_day.iter().max_by_key(|(_, s)| **s).map(|(d, s)| (*d, *s));
    let record_day_games = record
        .and_then(|(d, _)| day_games.get(&d))
        .map(|list| {
            let mut l = list.clone();
            l.sort_by(|a, b| b.1.cmp(&a.1));
            l.iter().filter_map(|(id, _)| by_id.get(id.as_str()).map(|g| g.title.clone())).take(3).collect()
        })
        .unwrap_or_default();

    let mut by_source: HashMap<SourceId, i64> = HashMap::new();
    for (id, s) in &per_game {
        if let Some(src) = by_id.get(id.as_str()).and_then(|g| g.sources.first()).map(|v| v.source.source) {
            *by_source.entry(src).or_default() += s;
        }
    }
    let favorite_source = by_source.iter().max_by_key(|(_, s)| **s).map(|(k, v)| (*k, *v));
    let source_share = favorite_source.map(|(_, s)| s as f64 / total_seconds.max(1) as f64).unwrap_or(0.0);

    // Sessions de l'année : heures, week-end, nuit, plus longue session.
    let sessions: Vec<activity::SessionRow> = activity::sessions(conn)?.into_iter().filter(|s| activity::game_day(s.started, tz, cfg.rollover_hour).year() == year).collect();
    let mut hourly = vec![0i64; 24];
    let mut weekend = 0i64;
    let mut night = 0i64;
    for s in &sessions {
        for (wd, h, secs) in activity::split_by_hour(s.started, s.ended, s.duration, tz) {
            hourly[h as usize] += secs;
            if wd >= 5 {
                weekend += secs;
            }
            if h >= 22 || h < 5 {
                night += secs;
            }
        }
    }
    let hour_total: i64 = hourly.iter().sum::<i64>().max(1);
    let longest = sessions.iter().max_by_key(|s| s.duration).map(|s| (by_id.get(s.game_id.as_str()).map(|g| g.title.clone()).unwrap_or_default(), s.duration));
    let avg_session = if sessions.is_empty() { 0.0 } else { sessions.iter().map(|s| s.duration).sum::<i64>() as f64 / sessions.len() as f64 / 3600.0 };

    let traits = Traits {
        total_hours: total_seconds as f64 / 3600.0,
        games_played: per_game.len(),
        avg_session_hours: avg_session,
        night_share: night as f64 / hour_total as f64,
        weekend_share: weekend as f64 / hour_total as f64,
        best_streak: streaks.best,
        top_share,
        days_played: played.len(),
    };
    let all_sessions = activity::sessions(conn)?;
    let badges = dashboard::badges(&games, &all_sessions, activity::current_streaks(conn).map(|s| s.best).unwrap_or(0)).into_iter().filter(|b| b.earned).collect();

    Ok(Recap {
        year,
        available: is_available(year, today),
        total_seconds,
        days_played: played.len(),
        sessions: sessions.len(),
        games_played: per_game.len(),
        top_games,
        best_streak: streaks.best,
        best_streak_start: streaks.best_start,
        best_streak_end: streaks.best_end,
        record_day: record.map(|(d, s)| (d.to_string(), s)),
        record_day_games,
        favorite_source,
        source_share,
        monthly,
        hourly,
        longest_session: longest,
        night_share: traits.night_share,
        weekend_share: traits.weekend_share,
        badges,
        profile: profile(&traits),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::Europe::Paris;

    #[test]
    fn availability_starts_on_december_first() {
        let d = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        assert!(!is_available(2026, d("2026-11-30")));
        assert!(is_available(2026, d("2026-12-01")));
        assert!(is_available(2025, d("2026-03-10")), "année passée : toujours disponible");
        assert!(!is_available(2027, d("2026-12-31")));
    }

    #[test]
    fn profile_picks_the_strongest_trait() {
        let base = Traits { total_hours: 200.0, games_played: 6, avg_session_hours: 1.0, night_share: 0.1, weekend_share: 0.3, best_streak: 5, top_share: 0.3, days_played: 90 };
        assert_eq!(profile(&Traits { night_share: 0.6, ..base }).id, "night-owl");
        assert_eq!(profile(&Traits { avg_session_hours: 5.0, ..base }).id, "marathon");
        assert_eq!(profile(&Traits { games_played: 40, ..base }).id, "explorer");
        assert_eq!(profile(&Traits { best_streak: 60, ..base }).id, "loyal");
        let monogame = profile(&Traits { top_share: 0.9, night_share: 0.33, ..base });
        assert_eq!(monogame.id, "devoted");
        assert_eq!(monogame.traits, vec!["Noctambule"], "trait secondaire marqué");
        assert_eq!(profile(&Traits { total_hours: 0.5, ..base }).id, "casual");
    }

    #[test]
    fn recap_of_a_year() {
        let conn = crate::db::open_in_memory().unwrap();
        conn.execute_batch(
            "INSERT INTO games(id, title, sort_title, added_at) VALUES('a', 'Hades', 'hades', '2025-01-01T00:00:00Z'), ('b', 'Celeste', 'celeste', '2025-01-01T00:00:00Z');
             INSERT INTO game_sources(id, game_id, source, external_id, installed, launch_method, launch_target) VALUES('sa', 'a', 'steam', '1', 1, 'uri', 'x'), ('sb', 'b', 'epic', '2', 1, 'uri', 'x');
             INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds) VALUES
               ('1', 'sa', '2026-03-06T20:00:00Z', '2026-03-06T23:00:00Z', 10800),
               ('2', 'sa', '2026-03-07T21:00:00Z', '2026-03-08T00:00:00Z', 10800),
               ('3', 'sb', '2026-03-08T13:00:00Z', '2026-03-08T14:00:00Z', 3600),
               ('4', 'sa', '2025-12-30T20:00:00Z', '2025-12-30T21:00:00Z', 3600);",
        )
        .unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 12, 2).unwrap();
        let r = compute(&conn, 2026, &Paris, today).unwrap();
        assert!(r.available);
        assert_eq!(r.total_seconds, 25200, "la session de 2025 n'est pas comptée");
        assert_eq!((r.sessions, r.games_played, r.days_played), (3, 2, 3));
        assert_eq!(r.top_games.iter().map(|g| g.title.as_str()).collect::<Vec<_>>(), vec!["Hades", "Celeste"]);
        assert_eq!(r.best_streak, 3);
        assert_eq!(r.favorite_source.map(|f| f.0), Some(SourceId::Steam));
        assert_eq!(r.monthly[2], 25200);
        assert!(r.weekend_share > 0.5, "samedi et dimanche");
        assert_eq!(r.longest_session.as_ref().map(|l| l.1), Some(10800));
        let r2025 = compute(&conn, 2025, &Paris, today).unwrap();
        assert_eq!(r2025.total_seconds, 3600);
    }
}
