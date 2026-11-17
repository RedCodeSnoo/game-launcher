//! Statistiques globales (F-HEU-06) : temps total, par source, top des jeux, par semaine et par mois, succès récents.
use crate::library::{self, GameView};
use crate::models::SourceId;
use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceTime {
    pub source: SourceId,
    pub minutes: i64,
    pub games: usize,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TopGame {
    pub game_id: String,
    pub title: String,
    pub cover_path: Option<String>,
    pub minutes: i64,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    /// Premier jour de la période (AAAA-MM-JJ).
    pub start: String,
    pub seconds: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentAchievement {
    pub game_id: String,
    pub game_title: String,
    pub name: String,
    pub icon: Option<String>,
    pub unlocked_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub total_minutes: i64,
    pub tracked_seconds: i64,
    pub games_total: usize,
    pub games_installed: usize,
    pub games_played: usize,
    pub achievements_unlocked: i64,
    pub achievements_total: i64,
    pub by_source: Vec<SourceTime>,
    pub top_games: Vec<TopGame>,
    /// 12 dernières semaines (lundi → dimanche) : sessions réelles uniquement (une base manuelle ou un temps importé
    /// n'a pas de date).
    pub weekly: Vec<Bucket>,
    /// 12 derniers mois, temps suivi par l'application.
    pub monthly: Vec<Bucket>,
    pub recent_achievements: Vec<RecentAchievement>,
}

pub fn summarize(games: &[GameView]) -> (i64, Vec<SourceTime>, Vec<TopGame>) {
    let visible: Vec<&GameView> = games.iter().filter(|g| !g.hidden).collect();
    let total: i64 = visible.iter().map(|g| g.playtime_minutes).sum();
    // Temps par launcher : total de chaque jeu (même valeur que la fiche jeu), compté pour sa source principale
    // (la première de la fiche : celle qui est installée et lancée).
    let mut by_source: BTreeMap<&'static str, (SourceId, i64, usize)> = BTreeMap::new();
    for g in &visible {
        let Some(primary) = g.sources.first() else { continue };
        let entry = by_source.entry(primary.source.source.as_str()).or_insert((primary.source.source, 0, 0));
        entry.1 += g.playtime_minutes;
        entry.2 += 1;
    }
    let mut by_source: Vec<SourceTime> = by_source.into_values().map(|(source, minutes, games)| SourceTime { source, minutes, games }).collect();
    by_source.sort_by(|a, b| b.minutes.cmp(&a.minutes));
    let mut top: Vec<&&GameView> = visible.iter().filter(|g| g.playtime_minutes > 0).collect();
    top.sort_by(|a, b| b.playtime_minutes.cmp(&a.playtime_minutes).then(a.sort_title.cmp(&b.sort_title)));
    let top_games = top.into_iter().take(10).map(|g| TopGame { game_id: g.id.clone(), title: g.title.clone(), cover_path: g.cover_path.clone(), minutes: g.playtime_minutes }).collect();
    (total, by_source, top_games)
}

fn week_start(d: NaiveDate) -> NaiveDate {
    d - Duration::days(d.weekday().num_days_from_monday() as i64)
}

fn month_start(d: NaiveDate) -> NaiveDate {
    d.with_day(1).unwrap_or(d)
}

fn months_back(d: NaiveDate, n: u32) -> NaiveDate {
    let total = d.year() * 12 + d.month0() as i32 - n as i32;
    NaiveDate::from_ymd_opt(total.div_euclid(12), total.rem_euclid(12) as u32 + 1, 1).unwrap_or(d)
}

/// Répartit des sessions (jour local de début, durée) sur les 12 dernières semaines et les 12 derniers mois.
pub fn bucket_sessions(sessions: &[(NaiveDate, i64)], today: NaiveDate) -> (Vec<Bucket>, Vec<Bucket>) {
    let this_week = week_start(today);
    let mut weekly: Vec<(NaiveDate, i64)> = (0..12).rev().map(|i| (this_week - Duration::weeks(i), 0)).collect();
    let this_month = month_start(today);
    let mut monthly: Vec<(NaiveDate, i64)> = (0..12).rev().map(|i| (months_back(this_month, i), 0)).collect();
    for (day, secs) in sessions {
        let w = week_start(*day);
        if let Some(slot) = weekly.iter_mut().find(|(start, _)| *start == w) {
            slot.1 += secs;
        }
        let m = month_start(*day);
        if let Some(slot) = monthly.iter_mut().find(|(start, _)| *start == m) {
            slot.1 += secs;
        }
    }
    let fmt = |v: Vec<(NaiveDate, i64)>| v.into_iter().map(|(d, seconds)| Bucket { start: d.format("%Y-%m-%d").to_string(), seconds }).collect();
    (fmt(weekly), fmt(monthly))
}

pub fn compute(conn: &Connection, disabled: &HashSet<String>) -> Result<Stats> {
    let games = library::list_visible_games(conn, disabled)?;
    let (total_minutes, by_source, top_games) = summarize(&games);
    let visible: Vec<&GameView> = games.iter().filter(|g| !g.hidden).collect();

    let sessions: Vec<(NaiveDate, i64)> = {
        let mut stmt = conn.prepare("SELECT started_at, duration_seconds FROM play_sessions WHERE duration_seconds > 0")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter().filter_map(|(start, secs)| DateTime::parse_from_rfc3339(&start).ok().map(|d| (d.with_timezone(&Local).date_naive(), secs))).collect()
    };
    let (weekly, monthly) = bucket_sessions(&sessions, Local::now().date_naive());

    let recent_achievements = {
        let mut stmt = conn.prepare(
            "SELECT g.id, g.title, a.name, a.icon_unlocked, a.unlocked_at FROM achievements a
             JOIN game_sources gs ON gs.id = a.game_source_id JOIN games g ON g.id = gs.game_id
             WHERE a.unlocked = 1 AND a.unlocked_at IS NOT NULL AND g.hidden = 0 ORDER BY a.unlocked_at DESC LIMIT 20",
        )?;
        let rows = stmt
            .query_map([], |r| Ok(RecentAchievement { game_id: r.get(0)?, game_title: r.get(1)?, name: r.get(2)?, icon: r.get(3)?, unlocked_at: r.get(4)? }))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    Ok(Stats {
        total_minutes,
        tracked_seconds: visible.iter().map(|g| g.tracked_seconds).sum(),
        games_total: visible.len(),
        games_installed: visible.iter().filter(|g| g.installed).count(),
        games_played: visible.iter().filter(|g| g.playtime_minutes > 0).count(),
        achievements_unlocked: visible.iter().map(|g| g.ach_unlocked).sum(),
        achievements_total: visible.iter().map(|g| g.ach_total).sum(),
        by_source,
        top_games,
        weekly,
        monthly,
        recent_achievements,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn buckets_sessions_by_week_and_month() {
        // Vendredi 13 novembre 2026 ; la semaine commence le lundi 9.
        let today = day(2026, 11, 13);
        let sessions = vec![
            (day(2026, 11, 13), 3600),
            (day(2026, 11, 9), 1800),
            (day(2026, 11, 8), 600),  // dimanche : semaine précédente
            (day(2026, 1, 20), 120),  // plus de 12 semaines, mais dans les 12 mois
            (day(2025, 11, 30), 999), // hors des 12 derniers mois
        ];
        let (weekly, monthly) = bucket_sessions(&sessions, today);
        assert_eq!(weekly.len(), 12);
        assert_eq!(weekly[11], Bucket { start: "2026-11-09".into(), seconds: 5400 });
        assert_eq!(weekly[10], Bucket { start: "2026-11-02".into(), seconds: 600 });
        assert_eq!(weekly.iter().map(|b| b.seconds).sum::<i64>(), 6000);
        assert_eq!(monthly.len(), 12);
        assert_eq!(monthly[0].start, "2025-12-01");
        assert_eq!(monthly[11], Bucket { start: "2026-11-01".into(), seconds: 6000 });
        assert_eq!(monthly[1], Bucket { start: "2026-01-01".into(), seconds: 120 });
        assert_eq!(monthly.iter().map(|b| b.seconds).sum::<i64>(), 6120);
    }

    #[test]
    fn month_arithmetic_crosses_years() {
        assert_eq!(months_back(day(2026, 1, 1), 1), day(2025, 12, 1));
        assert_eq!(months_back(day(2026, 3, 1), 14), day(2025, 1, 1));
    }

    /// Un jeu Riot et un jeu Epic avec base manuelle, un jeu Steam avec temps importé, un jeu à deux sources.
    fn library() -> Connection {
        let conn = crate::db::open_in_memory().unwrap();
        conn.execute_batch(
            "INSERT INTO games(id, title, sort_title, added_at, manual_baseline_minutes, manual_baseline_at) VALUES
               ('lol', 'League of Legends', 'league of legends', '2026-11-01T00:00:00Z', 138000, '2026-11-15T10:00:00Z'),
               ('fn', 'Fortnite', 'fortnite', '2026-11-01T00:00:00Z', 17760, '2026-11-15T10:00:00Z');
             INSERT INTO games(id, title, sort_title, added_at) VALUES
               ('hades', 'Hades', 'hades', '2026-11-01T00:00:00Z'),
               ('duo', 'Duo', 'duo', '2026-11-01T00:00:00Z');
             INSERT INTO game_sources(id, game_id, source, external_id, installed, launch_method, launch_target, source_playtime_minutes, playtime_synced_at) VALUES
               ('lol-r', 'lol', 'riot', 'lol', 1, 'launcher_cli', 'x', NULL, NULL),
               ('fn-e', 'fn', 'epic', 'fn', 1, 'uri', 'x', NULL, NULL),
               ('hades-s', 'hades', 'steam', '1145360', 1, 'uri', 'x', 600, '2026-11-15T00:00:00Z'),
               ('duo-l', 'duo', 'local', 'duo', 1, 'exe', 'x', NULL, NULL),
               ('duo-s', 'duo', 'steam', '42', 0, 'uri', 'x', 30, '2026-11-15T00:00:00Z');
             INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds) VALUES
               ('a', 'lol-r', '2026-11-13T20:00:00Z', '2026-11-13T21:00:00Z', 3600),
               ('b', 'lol-r', '2026-11-16T20:00:00Z', '2026-11-16T21:00:00Z', 3600),
               ('c', 'fn-e', '2026-11-16T18:00:00Z', '2026-11-16T18:30:00Z', 1800),
               ('d', 'hades-s', '2026-11-16T10:00:00Z', '2026-11-16T12:00:00Z', 7200),
               ('e', 'duo-l', '2026-11-16T10:00:00Z', '2026-11-16T11:00:00Z', 3600);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn manual_bases_count_in_their_launcher_totals() {
        let conn = library();
        let stats = compute(&conn, &HashSet::new()).unwrap();
        let minutes = |s: SourceId| stats.by_source.iter().find(|x| x.source == s).map(|x| x.minutes);
        assert_eq!(minutes(SourceId::Riot), Some(2301 * 60), "base 2300 h + 1 h jouée après la saisie");
        assert_eq!(minutes(SourceId::Epic), Some(17760 + 30), "base Fortnite + 30 min");
        assert_eq!(minutes(SourceId::Steam), Some(600 + 120), "temps Steam + sessions depuis la synchronisation");
        assert_eq!(minutes(SourceId::Local), Some(30 + 60), "jeu à deux sources : compté une fois, pour sa source principale");
        let sum: i64 = stats.by_source.iter().map(|s| s.minutes).sum();
        assert_eq!(sum, stats.total_minutes, "la somme des launchers égale le total");
    }

    #[test]
    fn game_page_and_stats_show_exactly_the_same_totals() {
        let conn = library();
        let games = library::list_games(&conn).unwrap();
        let stats = compute(&conn, &HashSet::new()).unwrap();
        assert_eq!(stats.total_minutes, games.iter().map(|g| g.playtime_minutes).sum::<i64>());
        for top in &stats.top_games {
            let page = games.iter().find(|g| g.id == top.game_id).unwrap();
            assert_eq!(top.minutes, page.playtime_minutes, "{} : fiche et statistiques identiques", page.title);
        }
        assert_eq!(stats.top_games[0].game_id, "lol");
        // Les graphiques ne comptent que les sessions réelles (5 sessions, 5 h 30), jamais les bases manuelles.
        let charted: i64 = stats.monthly.iter().map(|b| b.seconds).sum();
        assert!(charted <= 3600 + 3600 + 1800 + 7200 + 3600);
    }

    #[test]
    fn tray_recent_games_carry_the_same_total() {
        let conn = library();
        let recent = library::recent_games(&conn, &HashSet::new(), 5).unwrap();
        let lol = recent.iter().find(|g| g.id == "lol").unwrap();
        assert_eq!(lol.playtime_minutes, 2301 * 60);
        assert!(recent.iter().all(|g| g.installed));
    }
}
