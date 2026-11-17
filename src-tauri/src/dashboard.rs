//! Page Statistiques (V1.3) : période, filtres, grille d'activité, répartitions, habitudes, records, succès, badges.
//! Tout est calculé ici, en Rust, à partir des résumés journaliers (`activity.rs`) et des sessions.
use crate::activity::{self, Config, Streaks};
use crate::library::{self, GameView};
use crate::models::SourceId;
use crate::stats::{self, Bucket, SourceTime, TopGame};
use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Timelike, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    #[serde(rename = "7d")]
    Week,
    #[serde(rename = "30d")]
    Month,
    Year,
    All,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    pub game_id: Option<String>,
    pub source: Option<SourceId>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenreTime {
    pub genre: String,
    pub minutes: i64,
    pub games: usize,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub value: i64,
    pub label: String,
    pub date: Option<String>,
    pub game_id: Option<String>,
    pub game_title: Option<String>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Records {
    pub longest_session: Option<Record>,
    pub biggest_day: Option<Record>,
    pub best_week: Option<Record>,
    pub latest_hour: Option<Record>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RareAchievement {
    pub game_id: String,
    pub game_title: String,
    pub name: String,
    pub icon: Option<String>,
    pub percent: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlmostDone {
    pub game_id: String,
    pub title: String,
    pub cover_path: Option<String>,
    pub unlocked: i64,
    pub total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementStats {
    pub unlocked: i64,
    pub total: i64,
    /// Moyenne de complétion des jeux qui ont des succès (%).
    pub completion: f64,
    pub perfect_games: usize,
    pub rarest: Vec<RareAchievement>,
    pub almost_done: Vec<AlmostDone>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Badge {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub earned: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub period: Period,
    /// Premier jour de la période (inclus), absent pour « tout ».
    pub since: Option<String>,
    /// Temps de la période : sessions réelles ; pour « tout », total de la bibliothèque (bases et temps importés compris).
    pub total_minutes: i64,
    pub session_seconds: i64,
    pub games_played: usize,
    pub streaks: Streaks,
    pub days_played_this_year: usize,
    pub weekly: Vec<Bucket>,
    pub monthly: Vec<Bucket>,
    pub by_source: Vec<SourceTime>,
    pub by_genre: Vec<GenreTime>,
    pub top_games: Vec<TopGame>,
    /// 7 lignes (lundi → dimanche) × 24 heures, en secondes.
    pub habits: Vec<Vec<i64>>,
    pub records: Records,
    pub achievements: AchievementStats,
    pub badges: Vec<Badge>,
}

/// Source principale d'un jeu : la première de sa fiche (installée et disponible, c'est elle qu'on lance).
fn primary_source(g: &GameView) -> Option<SourceId> {
    g.sources.first().map(|s| s.source.source)
}

fn date_str(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

pub fn period_start(period: Period, today: NaiveDate) -> Option<NaiveDate> {
    match period {
        Period::Week => Some(today - Duration::days(6)),
        Period::Month => Some(today - Duration::days(29)),
        Period::Year => NaiveDate::from_ymd_opt(today.year(), 1, 1),
        Period::All => None,
    }
}

/// Temps par jour et par jeu (résumés journaliers), entre `from` et `to` inclus.
fn day_game_rows(conn: &Connection, from: Option<NaiveDate>, to: NaiveDate) -> Result<Vec<(NaiveDate, String, i64)>> {
    let mut stmt = conn.prepare("SELECT day, game_id, seconds FROM activity_days WHERE (?1 IS NULL OR day >= ?1) AND day <= ?2")?;
    let rows = stmt.query_map(params![from.map(date_str), date_str(to)], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?)))?;
    let mut out = Vec::new();
    for row in rows {
        let (day, game, secs) = row?;
        if let Ok(d) = NaiveDate::parse_from_str(&day, "%Y-%m-%d") {
            out.push((d, game, secs));
        }
    }
    Ok(out)
}

/// Badges du launcher (méta-succès).
pub fn badges(games: &[GameView], sessions: &[activity::SessionRow], best_streak: u32) -> Vec<Badge> {
    let tz = Local;
    let night_owl = sessions.iter().any(|s| activity::split_by_hour(s.started, s.ended, s.duration, &tz).iter().any(|(_, h, secs)| (3..6).contains(h) && *secs >= 60));
    let marathon = sessions.iter().any(|s| s.duration >= 6 * 3600);
    let library_size = games.iter().filter(|g| !g.hidden).count();
    let sources: HashSet<SourceId> = games.iter().flat_map(|g| g.sources.iter()).filter(|s| s.tracked_seconds > 0 || s.source.source_playtime_minutes.unwrap_or(0) > 0).map(|s| s.source.source).collect();
    let perfect = games.iter().any(|g| g.ach_total > 0 && g.ach_unlocked == g.ach_total);
    vec![
        Badge { id: "night-owl", name: "Noctambule", description: "Jouer après 3 h du matin.", icon: "🦉", earned: night_owl },
        Badge { id: "marathon", name: "Marathonien", description: "Une session de plus de 6 heures.", icon: "🏃", earned: marathon },
        Badge { id: "loyal", name: "Fidèle", description: "Une série de 30 jours joués d'affilée.", icon: "🔥", earned: best_streak >= 30 },
        Badge { id: "collector", name: "Collectionneur", description: "100 jeux dans la bibliothèque.", icon: "📚", earned: library_size >= 100 },
        Badge { id: "explorer", name: "Touche-à-tout", description: "Jouer sur 5 launchers différents.", icon: "🧭", earned: sources.len() >= 5 },
        Badge { id: "completionist", name: "Complétionniste", description: "Tous les succès d'un jeu.", icon: "🏆", earned: perfect },
    ]
}

/// Nouveaux badges depuis la dernière vérification (réglage `badges_earned`) ; met le réglage à jour.
pub fn new_badges(conn: &Connection) -> Result<Vec<Badge>> {
    let games = library::list_games(conn)?;
    let sessions = activity::sessions(conn)?;
    let streaks = activity::current_streaks(conn)?;
    let all = badges(&games, &sessions, streaks.best);
    let known: HashSet<String> = crate::db::get_setting(conn, "badges_earned").map(|v| v.split(',').map(|s| s.to_string()).collect()).unwrap_or_default();
    let earned: Vec<&Badge> = all.iter().filter(|b| b.earned).collect();
    let fresh: Vec<Badge> = earned.iter().filter(|b| !known.contains(b.id)).map(|b| (*b).clone()).collect();
    let first_run = crate::db::get_setting(conn, "badges_earned").is_none();
    crate::db::set_setting(conn, "badges_earned", &earned.iter().map(|b| b.id).collect::<Vec<_>>().join(","))?;
    // Premier calcul (mise à jour depuis une version sans badges) : pas de rafale de notifications.
    Ok(if first_run { Vec::new() } else { fresh })
}

pub fn compute<Tz: TimeZone>(conn: &Connection, disabled: &HashSet<String>, period: Period, filter: &Filter, tz: &Tz, now: DateTime<Utc>) -> Result<Dashboard> {
    activity::ensure_fresh(conn, tz)?;
    let cfg = Config::load(conn);
    let today = activity::game_day(now, tz, cfg.rollover_hour);
    let since = period_start(period, today);
    let games = library::list_visible_games(conn, disabled)?;
    let by_id: HashMap<&str, &GameView> = games.iter().map(|g| (g.id.as_str(), g)).collect();
    let keep = |game_id: &str| -> bool {
        let Some(g) = by_id.get(game_id) else { return false };
        if g.hidden {
            return false;
        }
        if let Some(id) = &filter.game_id {
            if id != game_id {
                return false;
            }
        }
        if let Some(src) = filter.source {
            if primary_source(g) != Some(src) {
                return false;
            }
        }
        true
    };

    // Séries (tous jeux, indépendantes des filtres).
    let all_days = activity::day_totals(conn, None)?;
    let played = activity::played_days(&all_days, cfg.min_minutes);
    let streaks = activity::streaks(&played, today, cfg.jokers);
    let days_played_this_year = played.iter().filter(|d| d.year() == today.year()).count();

    // Temps de la période, par jeu (sessions réelles uniquement).
    let rows = day_game_rows(conn, since, today)?;
    let mut per_game: HashMap<String, i64> = HashMap::new();
    for (_, game, secs) in rows.iter().filter(|(_, g, _)| keep(g)) {
        *per_game.entry(game.clone()).or_default() += secs;
    }
    let session_seconds: i64 = per_game.values().sum();
    let unfiltered = filter.game_id.is_none() && filter.source.is_none();

    // Totaux, répartition par launcher et top jeux : pour « tout », la même valeur que la fiche jeu.
    let (total_minutes, by_source, top_games) = if period == Period::All {
        let selected: Vec<GameView> = games.iter().filter(|g| keep(&g.id) || (unfiltered && !g.hidden)).cloned().collect();
        stats::summarize(&selected)
    } else {
        let mut src: BTreeMap<&'static str, (SourceId, i64, usize)> = BTreeMap::new();
        for (game, secs) in &per_game {
            if let Some(s) = by_id.get(game.as_str()).and_then(|g| primary_source(g)) {
                let e = src.entry(s.as_str()).or_insert((s, 0, 0));
                e.1 += secs / 60;
                e.2 += 1;
            }
        }
        let mut by_source: Vec<SourceTime> = src.into_values().map(|(source, minutes, games)| SourceTime { source, minutes, games }).collect();
        by_source.sort_by(|a, b| b.minutes.cmp(&a.minutes));
        let mut top: Vec<(&String, &i64)> = per_game.iter().filter(|(_, s)| **s >= 60).collect();
        top.sort_by(|a, b| b.1.cmp(a.1));
        let top_games = top
            .into_iter()
            .take(10)
            .filter_map(|(id, secs)| by_id.get(id.as_str()).map(|g| TopGame { game_id: g.id.clone(), title: g.title.clone(), cover_path: g.cover_path.clone(), minutes: secs / 60 }))
            .collect();
        (session_seconds / 60, by_source, top_games)
    };
    let games_played = if period == Period::All { games.iter().filter(|g| keep(&g.id) && g.playtime_minutes > 0).count() } else { per_game.len() };

    // Genres : le temps d'un jeu est partagé entre ses genres.
    let mut genres: BTreeMap<String, (i64, usize)> = BTreeMap::new();
    let game_minutes: Vec<(&GameView, i64)> = if period == Period::All {
        games.iter().filter(|g| keep(&g.id) && g.playtime_minutes > 0).map(|g| (g, g.playtime_minutes)).collect()
    } else {
        per_game.iter().filter_map(|(id, s)| by_id.get(id.as_str()).map(|g| (*g, s / 60))).collect()
    };
    for (g, minutes) in game_minutes {
        let list: Vec<String> = if g.genres.is_empty() { vec!["Non classé".to_string()] } else { g.genres.clone() };
        let share = minutes / list.len() as i64;
        for genre in list {
            let e = genres.entry(genre).or_default();
            e.0 += share;
            e.1 += 1;
        }
    }
    let mut by_genre: Vec<GenreTime> = genres.into_iter().map(|(genre, (minutes, games))| GenreTime { genre, minutes, games }).filter(|g| g.minutes > 0).collect();
    by_genre.sort_by(|a, b| b.minutes.cmp(&a.minutes));
    by_genre.truncate(12);

    // Barres par semaine et par mois (12 dernières), filtrées par jeu ou launcher.
    let bucket_rows = day_game_rows(conn, Some(today - Duration::days(400)), today)?;
    let mut daily: BTreeMap<NaiveDate, i64> = BTreeMap::new();
    for (d, g, s) in bucket_rows.iter().filter(|(_, g, _)| keep(g)) {
        *daily.entry(*d).or_default() += s;
        let _ = g;
    }
    let pairs: Vec<(NaiveDate, i64)> = daily.into_iter().collect();
    let (weekly, monthly) = stats::bucket_sessions(&pairs, today);

    // Sessions de la période : habitudes et records.
    let sessions: Vec<activity::SessionRow> = activity::sessions(conn)?
        .into_iter()
        .filter(|s| keep(&s.game_id) && since.map_or(true, |d| activity::game_day(s.ended, tz, cfg.rollover_hour) >= d))
        .collect();
    let mut habits = vec![vec![0i64; 24]; 7];
    for s in &sessions {
        for (wd, h, secs) in activity::split_by_hour(s.started, s.ended, s.duration, tz) {
            habits[wd as usize][h as usize] += secs;
        }
    }
    let title = |id: &str| by_id.get(id).map(|g| g.title.clone());
    let mut records = Records::default();
    if let Some(s) = sessions.iter().max_by_key(|s| s.duration) {
        records.longest_session = Some(Record {
            value: s.duration,
            label: "Plus longue session".into(),
            date: Some(date_str(activity::game_day(s.started, tz, cfg.rollover_hour))),
            game_id: Some(s.game_id.clone()),
            game_title: title(&s.game_id),
        });
    }
    let mut day_sum: BTreeMap<NaiveDate, i64> = BTreeMap::new();
    let mut week_game: HashMap<(NaiveDate, String), i64> = HashMap::new();
    for (d, g, s) in rows.iter().filter(|(_, g, _)| keep(g)) {
        *day_sum.entry(*d).or_default() += s;
        let week = *d - Duration::days(d.weekday().num_days_from_monday() as i64);
        *week_game.entry((week, g.clone())).or_default() += s;
    }
    if let Some((d, s)) = day_sum.iter().max_by_key(|(_, s)| **s) {
        records.biggest_day = Some(Record { value: *s, label: "Plus grosse journée".into(), date: Some(date_str(*d)), game_id: None, game_title: None });
    }
    if let Some(((week, g), s)) = week_game.iter().max_by_key(|(_, s)| **s) {
        records.best_week = Some(Record { value: *s, label: "Jeu le plus joué en une semaine".into(), date: Some(date_str(*week)), game_id: Some(g.clone()), game_title: title(g) });
    }
    // Heure la plus tardive : minutes écoulées depuis l'heure de bascule (2 h du matin est plus tard que 23 h).
    let mut latest: Option<(i64, &activity::SessionRow, DateTime<Utc>)> = None;
    for s in &sessions {
        let end = if s.ended > s.started { s.ended } else { s.started };
        let local = end.with_timezone(tz);
        let since_rollover = ((local.hour() as i64 - cfg.rollover_hour as i64).rem_euclid(24)) * 60 + local.minute() as i64;
        if latest.as_ref().map_or(true, |(m, _, _)| since_rollover > *m) {
            latest = Some((since_rollover, s, end));
        }
    }
    if let Some((_, s, end)) = latest {
        let local = end.with_timezone(tz);
        records.latest_hour = Some(Record {
            value: (local.hour() * 60 + local.minute()) as i64,
            label: "Heure de jeu la plus tardive".into(),
            date: Some(date_str(activity::game_day(end, tz, cfg.rollover_hour))),
            game_id: Some(s.game_id.clone()),
            game_title: title(&s.game_id),
        });
    }

    // Succès (toute la bibliothèque, filtres appliqués).
    let with_ach: Vec<&GameView> = games.iter().filter(|g| keep(&g.id) && g.ach_total > 0).collect();
    let rarest = {
        let mut stmt = conn.prepare(
            "SELECT g.id, g.title, a.name, a.icon_unlocked, a.global_percent FROM achievements a
             JOIN game_sources gs ON gs.id = a.game_source_id JOIN games g ON g.id = gs.game_id
             WHERE a.unlocked = 1 AND a.global_percent IS NOT NULL AND g.hidden = 0 ORDER BY a.global_percent ASC LIMIT 40",
        )?;
        let rows = stmt.query_map([], |r| Ok(RareAchievement { game_id: r.get(0)?, game_title: r.get(1)?, name: r.get(2)?, icon: r.get(3)?, percent: r.get(4)? }))?.collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter().filter(|a| keep(&a.game_id)).take(5).collect()
    };
    let mut almost: Vec<AlmostDone> = with_ach
        .iter()
        .filter(|g| g.ach_unlocked * 100 >= g.ach_total * 80 && g.ach_unlocked < g.ach_total)
        .map(|g| AlmostDone { game_id: g.id.clone(), title: g.title.clone(), cover_path: g.cover_path.clone(), unlocked: g.ach_unlocked, total: g.ach_total })
        .collect();
    almost.sort_by(|a, b| (b.unlocked * 1000 / b.total).cmp(&(a.unlocked * 1000 / a.total)));
    almost.truncate(6);
    let achievements = AchievementStats {
        unlocked: with_ach.iter().map(|g| g.ach_unlocked).sum(),
        total: with_ach.iter().map(|g| g.ach_total).sum(),
        completion: if with_ach.is_empty() { 0.0 } else { with_ach.iter().map(|g| g.ach_unlocked as f64 / g.ach_total as f64).sum::<f64>() / with_ach.len() as f64 * 100.0 },
        perfect_games: with_ach.iter().filter(|g| g.ach_unlocked == g.ach_total).count(),
        rarest,
        almost_done: almost,
    };

    let all_sessions = activity::sessions(conn)?;
    let badges = badges(&games, &all_sessions, streaks.best);

    Ok(Dashboard {
        period,
        since: since.map(date_str),
        total_minutes,
        session_seconds,
        games_played,
        streaks,
        days_played_this_year,
        weekly,
        monthly,
        by_source,
        by_genre,
        top_games,
        habits,
        records,
        achievements,
        badges,
    })
}

// ---------------------------------------------------------------------------
// Grille d'activité
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDay {
    pub day: String,
    pub seconds: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityGrid {
    /// Premier et dernier jour affichés (inclus).
    pub start: String,
    pub end: String,
    pub today: String,
    pub days: Vec<ActivityDay>,
    pub years: Vec<i32>,
    pub min_minutes: i64,
}

/// Grille d'une année (`year`) ou des 12 derniers mois, pour tous les jeux ou un seul.
pub fn grid(conn: &Connection, year: Option<i32>, game_id: Option<&str>) -> Result<ActivityGrid> {
    activity::ensure_fresh(conn, &Local)?;
    let cfg = Config::load(conn);
    let today = activity::today(conn);
    let (start, end) = match year {
        Some(y) => (NaiveDate::from_ymd_opt(y, 1, 1).unwrap_or(today), NaiveDate::from_ymd_opt(y, 12, 31).unwrap_or(today)),
        None => (today - Duration::days(364), today),
    };
    let totals = activity::day_totals(conn, game_id)?;
    let mut years: Vec<i32> = activity::day_totals(conn, None)?.keys().map(|d| d.year()).collect();
    years.push(today.year());
    years.sort_unstable();
    years.dedup();
    years.reverse();
    Ok(ActivityGrid {
        start: date_str(start),
        end: date_str(end),
        today: date_str(today),
        days: totals.range(start..=end).map(|(d, s)| ActivityDay { day: date_str(*d), seconds: *s }).collect(),
        years,
        min_minutes: cfg.min_minutes,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayGame {
    pub game_id: String,
    pub title: String,
    pub cover_path: Option<String>,
    pub seconds: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaySession {
    pub id: String,
    pub game_id: String,
    pub game_title: String,
    pub started_at: String,
    pub ended_at: String,
    pub duration_seconds: i64,
    pub origin: String,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayDetail {
    pub day: String,
    pub seconds: i64,
    pub games: Vec<DayGame>,
    pub sessions: Vec<DaySession>,
}

/// Détail d'un jour de jeu : temps par jeu (avec jaquettes) et sessions qui le touchent.
pub fn day_detail(conn: &Connection, day: &str, game_id: Option<&str>) -> Result<DayDetail> {
    activity::ensure_fresh(conn, &Local)?;
    let cfg = Config::load(conn);
    let d = NaiveDate::parse_from_str(day, "%Y-%m-%d")?;
    let mut stmt = conn.prepare(
        "SELECT a.game_id, g.title, g.cover_path, a.seconds FROM activity_days a JOIN games g ON g.id = a.game_id
         WHERE a.day = ?1 AND (?2 IS NULL OR a.game_id = ?2) ORDER BY a.seconds DESC",
    )?;
    let games = stmt
        .query_map(params![day, game_id], |r| Ok(DayGame { game_id: r.get(0)?, title: r.get(1)?, cover_path: r.get(2)?, seconds: r.get(3)? }))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let from = activity::day_start(d, &Local, cfg.rollover_hour);
    let to = activity::day_start(d + Duration::days(1), &Local, cfg.rollover_hour);
    let iso = |t: DateTime<Utc>| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut stmt = conn.prepare(
        "SELECT ps.id, g.id, g.title, ps.started_at, ps.ended_at, ps.duration_seconds, ps.origin, ps.note FROM play_sessions ps
         JOIN game_sources gs ON gs.id = ps.game_source_id JOIN games g ON g.id = gs.game_id
         WHERE ps.started_at < ?2 AND ps.ended_at >= ?1 AND ps.duration_seconds > 0 AND (?3 IS NULL OR g.id = ?3) ORDER BY ps.started_at",
    )?;
    let sessions = stmt
        .query_map(params![iso(from), iso(to), game_id], |r| {
            Ok(DaySession { id: r.get(0)?, game_id: r.get(1)?, game_title: r.get(2)?, started_at: r.get(3)?, ended_at: r.get(4)?, duration_seconds: r.get(5)?, origin: r.get(6)?, note: r.get(7)? })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(DayDetail { day: day.to_string(), seconds: games.iter().map(|g| g.seconds).sum(), games, sessions })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::Europe::Paris;

    fn library() -> Connection {
        let conn = crate::db::open_in_memory().unwrap();
        conn.execute_batch(
            "INSERT INTO games(id, title, sort_title, added_at, genres, manual_baseline_minutes, manual_baseline_at) VALUES
               ('lol', 'League of Legends', 'league', '2026-01-01T00:00:00Z', 'MOBA', 138000, '2026-11-01T00:00:00Z'),
               ('hades', 'Hades', 'hades', '2026-01-01T00:00:00Z', 'Action, Roguelike', NULL, NULL);
             INSERT INTO game_sources(id, game_id, source, external_id, installed, launch_method, launch_target) VALUES
               ('lol-r', 'lol', 'riot', 'lol', 1, 'launcher_cli', 'x'), ('hades-s', 'hades', 'steam', '1', 1, 'uri', 'x');
             INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds) VALUES
               ('a', 'lol-r', '2026-11-16T19:00:00Z', '2026-11-16T21:00:00Z', 7200),
               ('b', 'lol-r', '2026-11-17T01:30:00Z', '2026-11-17T02:40:00Z', 4200),
               ('c', 'hades-s', '2026-11-10T18:00:00Z', '2026-11-10T19:00:00Z', 3600),
               ('d', 'hades-s', '2026-06-01T18:00:00Z', '2026-06-02T01:00:00Z', 25200),
               ('e', 'hades-s', '2025-12-20T18:00:00Z', '2025-12-20T19:00:00Z', 3600);",
        )
        .unwrap();
        conn
    }

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-17T12:00:00Z").unwrap().with_timezone(&Utc)
    }

    #[test]
    fn periods_count_only_real_sessions_except_all() {
        let conn = library();
        let none = HashSet::new();
        let week = compute(&conn, &none, Period::Week, &Filter::default(), &Paris, now()).unwrap();
        // 7 derniers jours de jeu : du 11 au 17 novembre (la partie de Hades du 10 est en dehors).
        assert_eq!(week.session_seconds, 7200 + 4200);
        assert_eq!(week.total_minutes, (7200 + 4200) / 60);
        assert_eq!(week.top_games[0].title, "League of Legends");
        let all = compute(&conn, &none, Period::All, &Filter::default(), &Paris, now()).unwrap();
        let lol = library::list_games(&conn).unwrap().into_iter().find(|g| g.id == "lol").unwrap();
        assert_eq!(all.top_games[0].minutes, lol.playtime_minutes, "« tout » : même total que la fiche jeu (base manuelle comprise)");
        let year = compute(&conn, &none, Period::Year, &Filter::default(), &Paris, now()).unwrap();
        assert_eq!(year.session_seconds, 7200 + 4200 + 3600 + 25200, "la session de 2025 est exclue");
    }

    #[test]
    fn filters_by_game_and_launcher() {
        let conn = library();
        let none = HashSet::new();
        let only_hades = compute(&conn, &none, Period::Year, &Filter { game_id: Some("hades".into()), source: None }, &Paris, now()).unwrap();
        assert_eq!(only_hades.session_seconds, 3600 + 25200);
        let riot = compute(&conn, &none, Period::Year, &Filter { game_id: None, source: Some(SourceId::Riot) }, &Paris, now()).unwrap();
        assert_eq!(riot.session_seconds, 7200 + 4200);
        assert_eq!(riot.by_source.len(), 1);
        assert_eq!(riot.weekly.iter().map(|b| b.seconds).sum::<i64>(), 7200 + 4200);
    }

    #[test]
    fn genres_habits_records_and_badges() {
        let conn = library();
        let d = compute(&conn, &HashSet::new(), Period::Year, &Filter::default(), &Paris, now()).unwrap();
        // Hades (8 h) partagé entre Action et Roguelike, LoL (3 h 10) en MOBA.
        assert_eq!(d.by_genre.iter().find(|g| g.genre == "Action").map(|g| g.minutes), Some(240));
        assert_eq!(d.by_genre[0].genre, "Action");
        // Lundis soir 20 h → 22 h à Paris : 16 novembre (LoL) et 1er juin (début de la longue partie de Hades).
        assert_eq!(d.habits[0][20] + d.habits[0][21], 7200 + 7200);
        assert_eq!(d.habits.iter().flatten().sum::<i64>(), 7200 + 4200 + 3600 + 25200);
        let r = &d.records;
        assert_eq!(r.longest_session.as_ref().map(|x| x.value), Some(25200));
        // 2 h 30 → 3 h 40 à Paris le 17 : plus tard que 3 h de la session du 1er juin (fin 3 h 00).
        assert_eq!(r.latest_hour.as_ref().map(|x| x.value), Some(3 * 60 + 40));
        assert_eq!(r.latest_hour.as_ref().and_then(|x| x.date.clone()).as_deref(), Some("2026-11-16"), "compte pour la veille (bascule à 4 h)");
        let earned: Vec<&str> = d.badges.iter().filter(|b| b.earned).map(|b| b.id).collect();
        assert!(earned.contains(&"marathon"), "session de 7 h");
        assert!(!earned.contains(&"collector"));
    }

    #[test]
    fn grid_and_day_detail() {
        let conn = library();
        let g = grid(&conn, Some(2026), None).unwrap();
        assert_eq!((g.start.as_str(), g.end.as_str()), ("2026-01-01", "2026-12-31"));
        assert!(g.years.contains(&2025) && g.years.contains(&2026));
        let total: i64 = g.days.iter().map(|d| d.seconds).sum();
        assert_eq!(total, 7200 + 4200 + 3600 + 25200);
        let only_lol = grid(&conn, Some(2026), Some("lol")).unwrap();
        assert_eq!(only_lol.days.iter().map(|d| d.seconds).sum::<i64>(), 11400);
        let day = g.days.iter().find(|d| d.seconds == 25200 || d.day == "2026-06-01").unwrap();
        let detail = day_detail(&conn, &day.day, None).unwrap();
        assert_eq!(detail.games[0].title, "Hades");
        assert_eq!(detail.sessions.len(), 1);
    }

    #[test]
    fn new_badges_are_reported_once() {
        let conn = library();
        assert!(new_badges(&conn).unwrap().is_empty(), "premier calcul : pas de rafale de notifications");
        conn.execute("INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds) VALUES('f', 'hades-s', '2026-11-12T01:00:00Z', '2026-11-12T03:30:00Z', 9000)", []).unwrap();
        crate::db::set_setting(&conn, "badges_earned", "marathon").unwrap();
        let fresh = new_badges(&conn).unwrap();
        assert_eq!(fresh.iter().map(|b| b.id).collect::<Vec<_>>(), vec!["night-owl"], "2 h → 4 h 30 à Paris : noctambule");
        assert!(new_badges(&conn).unwrap().is_empty());
    }

    /// Exigence : la page Statistiques en moins de 300 ms avec 3 ans d'historique (mesuré en release :
    /// `cargo test --release three_years_dashboard -- --nocapture`).
    #[test]
    fn three_years_dashboard() {
        let conn = crate::db::open_in_memory().unwrap();
        let tx = conn.unchecked_transaction().unwrap();
        for g in 0..60 {
            tx.execute("INSERT INTO games(id, title, sort_title, added_at, genres) VALUES(?1, ?1, ?1, '2023-01-01T00:00:00Z', 'Action, RPG')", params![format!("g{g}")]).unwrap();
            tx.execute("INSERT INTO game_sources(id, game_id, source, external_id, installed, launch_method, launch_target) VALUES(?1, ?2, 'steam', ?1, 1, 'uri', 'x')", params![format!("s{g}"), format!("g{g}")]).unwrap();
        }
        let base = DateTime::parse_from_rfc3339("2023-11-18T18:00:00Z").unwrap().with_timezone(&Utc);
        for i in 0..1095i64 {
            for k in 0..4i64 {
                let start = base + Duration::days(i) + Duration::hours(k * 2);
                let secs = 1800 + (i * 37 + k * 11) % 3600;
                tx.execute(
                    "INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds) VALUES(?1, ?2, ?3, ?4, ?5)",
                    params![format!("{i}-{k}"), format!("s{}", (i + k) % 60), start.to_rfc3339(), (start + Duration::seconds(secs)).to_rfc3339(), secs],
                )
                .unwrap();
            }
        }
        tx.commit().unwrap();
        let now = base + Duration::days(1095);
        activity::ensure_fresh(&conn, &Paris).unwrap(); // résumés construits une fois (fin de session)
        let t = std::time::Instant::now();
        for period in [Period::Week, Period::Month, Period::Year, Period::All] {
            compute(&conn, &HashSet::new(), period, &Filter::default(), &Paris, now).unwrap();
        }
        let per_page = t.elapsed() / 4;
        let t = std::time::Instant::now();
        grid(&conn, None, None).unwrap();
        let grid_time = t.elapsed();
        println!("3 ans, 4 380 sessions : page Statistiques {per_page:?} en moyenne, grille {grid_time:?}");
        let budget = if cfg!(debug_assertions) { 3000 } else { 300 };
        assert!(per_page.as_millis() < budget, "page Statistiques : {per_page:?}");
    }
}

