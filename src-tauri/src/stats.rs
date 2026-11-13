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
    /// 12 dernières semaines (lundi → dimanche), temps suivi par l'application.
    pub weekly: Vec<Bucket>,
    /// 12 derniers mois, temps suivi par l'application.
    pub monthly: Vec<Bucket>,
    pub recent_achievements: Vec<RecentAchievement>,
}

pub fn summarize(games: &[GameView]) -> (i64, Vec<SourceTime>, Vec<TopGame>) {
    let visible: Vec<&GameView> = games.iter().filter(|g| !g.hidden).collect();
    let total: i64 = visible.iter().map(|g| g.playtime_minutes).sum();
    let mut by_source: BTreeMap<&'static str, (SourceId, i64, usize)> = BTreeMap::new();
    for g in &visible {
        for s in &g.sources {
            let entry = by_source.entry(s.source.source.as_str()).or_insert((s.source.source, 0, 0));
            entry.1 += s.playtime_minutes;
            entry.2 += 1;
        }
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
}
