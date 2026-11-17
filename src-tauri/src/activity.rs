//! Activité quotidienne et séries de jours joués (« streaks », V1.3).
//!
//! Règles (S1) :
//! - un « jour de jeu » commence à l'heure de bascule (4 h par défaut) : une partie à 1 h du matin compte pour la veille ;
//! - une session à cheval sur deux jours est répartie entre eux au prorata du temps passé de chaque côté ;
//! - seules les sessions réelles comptent (lancées ou détectées) : une base saisie à la main n'a pas de date ;
//! - un jour est « joué » si le temps total de la journée atteint le seuil (15 min par défaut).
//!
//! Les totaux par jour et par jeu sont tenus dans la table `activity_days`, reconstruite quand les sessions changent
//! (signature des sessions), pour un affichage instantané même avec des années d'historique.
use crate::db;
use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub const DEFAULT_ROLLOVER_HOUR: u32 = 4;
pub const DEFAULT_MIN_MINUTES: i64 = 15;
/// Un joker gagné tous les 7 jours joués d'affilée, 2 au plus.
pub const JOKER_EVERY: u32 = 7;
pub const MAX_JOKERS: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    pub rollover_hour: u32,
    pub min_minutes: i64,
    pub jokers: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config { rollover_hour: DEFAULT_ROLLOVER_HOUR, min_minutes: DEFAULT_MIN_MINUTES, jokers: false }
    }
}

impl Config {
    pub fn load(conn: &Connection) -> Config {
        let int = |key: &str| db::get_setting(conn, key).and_then(|v| v.trim().parse::<i64>().ok());
        Config {
            rollover_hour: int("streak_rollover_hour").map(|h| h.clamp(0, 12) as u32).unwrap_or(DEFAULT_ROLLOVER_HOUR),
            min_minutes: int("streak_min_minutes").map(|m| m.clamp(1, 600)).unwrap_or(DEFAULT_MIN_MINUTES),
            jokers: db::get_setting(conn, "streak_jokers").as_deref() == Some("1"),
        }
    }
}

// ---------------------------------------------------------------------------
// Découpage du temps (logique pure, testée avec plusieurs fuseaux)
// ---------------------------------------------------------------------------

/// Instant UTC d'une heure locale ; en cas de trou (passage à l'heure d'été), l'heure suivante qui existe.
fn local_to_utc<Tz: TimeZone>(tz: &Tz, local: NaiveDateTime) -> DateTime<Utc> {
    for extra in 0..4 {
        match tz.from_local_datetime(&(local + Duration::hours(extra))) {
            LocalResult::Single(t) => return t.with_timezone(&Utc),
            LocalResult::Ambiguous(first, _) => return first.with_timezone(&Utc),
            LocalResult::None => continue,
        }
    }
    Utc.from_utc_datetime(&local)
}

/// Jour de jeu d'un instant : date locale, décalée de l'heure de bascule.
pub fn game_day<Tz: TimeZone>(t: DateTime<Utc>, tz: &Tz, rollover: u32) -> NaiveDate {
    let local = t.with_timezone(tz).naive_local();
    (local - Duration::hours(rollover as i64)).date()
}

/// Début (UTC) du jour de jeu `day`.
pub fn day_start<Tz: TimeZone>(day: NaiveDate, tz: &Tz, rollover: u32) -> DateTime<Utc> {
    local_to_utc(tz, day.and_time(NaiveTime::from_hms_opt(rollover, 0, 0).unwrap_or_default()))
}

/// Répartit une session [`start`, `end`[ entre les jours de jeu qu'elle couvre. `duration` (secondes enregistrées)
/// est réparti au prorata du temps réel passé dans chaque jour.
pub fn split_by_day<Tz: TimeZone>(start: DateTime<Utc>, end: DateTime<Utc>, duration: i64, tz: &Tz, rollover: u32) -> Vec<(NaiveDate, i64)> {
    if duration <= 0 {
        return Vec::new();
    }
    if end <= start {
        return vec![(game_day(start, tz, rollover), duration)];
    }
    let span = (end - start).num_seconds().max(1) as f64;
    let mut out: Vec<(NaiveDate, i64)> = Vec::new();
    let mut cur = start;
    let mut assigned = 0i64;
    while cur < end {
        let day = game_day(cur, tz, rollover);
        let next = day_start(day + Duration::days(1), tz, rollover).max(cur + Duration::seconds(1));
        let seg_end = next.min(end);
        let part = if seg_end >= end { duration - assigned } else { ((seg_end - cur).num_seconds() as f64 / span * duration as f64).round() as i64 };
        if part > 0 {
            out.push((day, part));
            assigned += part;
        }
        cur = seg_end;
    }
    out
}

/// Répartit une session par (jour de la semaine local, 0 = lundi ; heure locale) pour la carte des habitudes.
pub fn split_by_hour<Tz: TimeZone>(start: DateTime<Utc>, end: DateTime<Utc>, duration: i64, tz: &Tz) -> Vec<(u32, u32, i64)> {
    if duration <= 0 || end <= start {
        return Vec::new();
    }
    let span = (end - start).num_seconds().max(1) as f64;
    let mut out = Vec::new();
    let mut cur = start;
    while cur < end {
        let local = cur.with_timezone(tz);
        let hour_start = local.naive_local().date().and_hms_opt(local.hour(), 0, 0).unwrap_or_default();
        let next = local_to_utc(tz, hour_start + Duration::hours(1)).max(cur + Duration::seconds(1));
        let seg_end = next.min(end);
        let part = ((seg_end - cur).num_seconds() as f64 / span * duration as f64).round() as i64;
        if part > 0 {
            out.push((local.weekday().num_days_from_monday(), local.hour(), part));
        }
        cur = seg_end;
    }
    out
}

// ---------------------------------------------------------------------------
// Séries de jours joués
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Streaks {
    /// Série en cours (jours joués d'affilée jusqu'à aujourd'hui, ou jusqu'à hier si aujourd'hui n'est pas encore joué).
    pub current: u32,
    pub current_start: Option<String>,
    /// Aujourd'hui n'est pas encore joué : la série d'hier sera perdue à la bascule.
    pub in_danger: bool,
    pub today_played: bool,
    pub best: u32,
    pub best_start: Option<String>,
    pub best_end: Option<String>,
    pub jokers_available: u32,
    /// Jours manqués sauvés par un joker.
    pub joker_days: Vec<String>,
}

fn iso(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Calcule la série en cours et la meilleure série à partir des jours joués.
pub fn streaks(played: &BTreeSet<NaiveDate>, today: NaiveDate, jokers_enabled: bool) -> Streaks {
    let mut s = Streaks { today_played: played.contains(&today), ..Default::default() };
    let Some(&first) = played.iter().next() else { return s };
    let (mut len, mut start, mut last_played): (u32, Option<NaiveDate>, Option<NaiveDate>) = (0, None, None);
    let mut jokers = 0u32;
    let mut run_since_joker = 0u32;
    let mut saved: Vec<NaiveDate> = Vec::new();
    let mut run_saved: Vec<NaiveDate> = Vec::new();
    let mut day = first;
    while day <= today {
        if played.contains(&day) {
            if len == 0 {
                start = Some(day);
                run_saved.clear();
            }
            len += 1;
            last_played = Some(day);
            run_since_joker += 1;
            if jokers_enabled && run_since_joker >= JOKER_EVERY {
                run_since_joker = 0;
                jokers = (jokers + 1).min(MAX_JOKERS);
            }
            if len > s.best {
                s.best = len;
                s.best_start = start.map(iso);
                s.best_end = Some(iso(day));
            }
        } else if day != today && len > 0 {
            if jokers_enabled && jokers > 0 {
                jokers -= 1;
                run_saved.push(day);
            } else {
                len = 0;
                start = None;
                jokers = 0;
                run_since_joker = 0;
                run_saved.clear();
            }
        }
        day += Duration::days(1);
    }
    if len > 0 && last_played.is_some() {
        s.current = len;
        s.current_start = start.map(iso);
        s.in_danger = !s.today_played;
        saved.extend(run_saved);
    }
    s.jokers_available = if s.current > 0 { jokers } else { 0 };
    s.joker_days = saved.into_iter().map(iso).collect();
    s
}

// ---------------------------------------------------------------------------
// Table des résumés journaliers
// ---------------------------------------------------------------------------

pub struct SessionRow {
    pub game_id: String,
    pub started: DateTime<Utc>,
    pub ended: DateTime<Utc>,
    pub duration: i64,
}

fn parse(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc))
}

/// Sessions réelles (avec durée), et leur jeu.
pub fn sessions(conn: &Connection) -> Result<Vec<SessionRow>> {
    let mut stmt = conn.prepare(
        "SELECT gs.game_id, ps.started_at, ps.ended_at, ps.duration_seconds FROM play_sessions ps
         JOIN game_sources gs ON gs.id = ps.game_source_id WHERE ps.duration_seconds > 0",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?)))?;
    let mut out = Vec::new();
    for row in rows {
        let (game_id, started, ended, duration) = row?;
        if let (Some(started), Some(ended)) = (parse(&started), parse(&ended)) {
            out.push(SessionRow { game_id, started, ended, duration });
        }
    }
    Ok(out)
}

/// Signature des sessions et des réglages : si elle change, les résumés sont reconstruits.
fn signature<Tz: TimeZone>(conn: &Connection, cfg: &Config, tz: &Tz) -> Result<String> {
    let (n, total, last, games): (i64, i64, String, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(ps.duration_seconds), 0), COALESCE(MAX(ps.ended_at), ''), COUNT(DISTINCT gs.game_id)
         FROM play_sessions ps JOIN game_sources gs ON gs.id = ps.game_source_id",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    // Décalage du fuseau à deux dates : un changement de fuseau reconstruit tout.
    let offsets: Vec<i32> = [(2026, 1, 15), (2026, 7, 15)]
        .iter()
        .map(|&(y, m, d)| {
            let t = Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).single().unwrap_or_default();
            t.with_timezone(tz).naive_local().signed_duration_since(t.naive_utc()).num_minutes() as i32
        })
        .collect();
    Ok(format!("v1|{n}|{total}|{last}|{games}|{}|{:?}", cfg.rollover_hour, offsets))
}

/// Reconstruit `activity_days` si les sessions ont changé depuis la dernière fois. Renvoie vrai si reconstruit.
pub fn ensure_fresh<Tz: TimeZone>(conn: &Connection, tz: &Tz) -> Result<bool> {
    let cfg = Config::load(conn);
    let sig = signature(conn, &cfg, tz)?;
    if db::get_setting(conn, "activity_signature").as_deref() == Some(sig.as_str()) {
        return Ok(false);
    }
    rebuild(conn, tz, cfg.rollover_hour)?;
    db::set_setting(conn, "activity_signature", &sig)?;
    Ok(true)
}

pub fn rebuild<Tz: TimeZone>(conn: &Connection, tz: &Tz, rollover: u32) -> Result<()> {
    let mut acc: HashMap<(NaiveDate, String), (i64, i64)> = HashMap::new();
    for s in sessions(conn)? {
        for (i, (day, secs)) in split_by_day(s.started, s.ended, s.duration, tz, rollover).into_iter().enumerate() {
            let e = acc.entry((day, s.game_id.clone())).or_insert((0, 0));
            e.0 += secs;
            if i == 0 {
                e.1 += 1;
            }
        }
    }
    conn.execute_batch("BEGIN; DELETE FROM activity_days;")?;
    let result = (|| -> Result<()> {
        let mut stmt = conn.prepare("INSERT INTO activity_days(day, game_id, seconds, sessions) VALUES(?1, ?2, ?3, ?4)")?;
        for ((day, game), (secs, n)) in &acc {
            stmt.execute(params![iso(*day), game, secs, n])?;
        }
        Ok(())
    })();
    match result {
        Ok(()) => conn.execute_batch("COMMIT;")?,
        Err(e) => {
            conn.execute_batch("ROLLBACK;")?;
            return Err(e);
        }
    }
    Ok(())
}

/// Temps total par jour de jeu (tous jeux, ou un seul jeu).
pub fn day_totals(conn: &Connection, game_id: Option<&str>) -> Result<BTreeMap<NaiveDate, i64>> {
    let mut stmt = conn.prepare("SELECT day, SUM(seconds) FROM activity_days WHERE (?1 IS NULL OR game_id = ?1) GROUP BY day")?;
    let rows = stmt.query_map(params![game_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (day, secs) = row?;
        if let Ok(d) = NaiveDate::parse_from_str(&day, "%Y-%m-%d") {
            out.insert(d, secs);
        }
    }
    Ok(out)
}

/// Jours joués : temps total ≥ seuil.
pub fn played_days(totals: &BTreeMap<NaiveDate, i64>, min_minutes: i64) -> BTreeSet<NaiveDate> {
    totals.iter().filter(|(_, s)| **s >= min_minutes * 60).map(|(d, _)| *d).collect()
}

/// Jour de jeu actuel, dans le fuseau de la machine.
pub fn today(conn: &Connection) -> NaiveDate {
    game_day(Utc::now(), &chrono::Local, Config::load(conn).rollover_hour)
}

/// Séries actuelles avec les réglages de l'utilisateur.
pub fn current_streaks(conn: &Connection) -> Result<Streaks> {
    ensure_fresh(conn, &chrono::Local)?;
    let cfg = Config::load(conn);
    let totals = day_totals(conn, None)?;
    Ok(streaks(&played_days(&totals, cfg.min_minutes), today(conn), cfg.jokers))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;
    use chrono_tz::Europe::Paris;

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }
    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn rollover_hour_moves_late_night_to_the_previous_day() {
        let paris = &Paris;
        // 1 h du matin le 12 novembre (heure de Paris) = jour de jeu du 11.
        assert_eq!(game_day(utc("2026-11-12T00:00:00Z"), paris, 4), d("2026-11-11"));
        // 4 h 30 : nouveau jour.
        assert_eq!(game_day(utc("2026-11-12T03:30:00Z"), paris, 4), d("2026-11-12"));
        // Bascule à minuit : jour calendaire.
        assert_eq!(game_day(utc("2026-11-12T00:00:00Z"), paris, 0), d("2026-11-12"));
    }

    #[test]
    fn session_across_the_rollover_is_split_pro_rata() {
        let paris = &Paris;
        // 3 h → 5 h heure de Paris (UTC+1) : 1 h avant la bascule de 4 h, 1 h après.
        let parts = split_by_day(utc("2026-11-12T02:00:00Z"), utc("2026-11-12T04:00:00Z"), 7200, paris, 4);
        assert_eq!(parts, vec![(d("2026-11-11"), 3600), (d("2026-11-12"), 3600)]);
        // Durée enregistrée plus courte que l'intervalle (pause) : répartie dans les mêmes proportions.
        let parts = split_by_day(utc("2026-11-12T02:00:00Z"), utc("2026-11-12T04:00:00Z"), 3600, paris, 4);
        assert_eq!(parts, vec![(d("2026-11-11"), 1800), (d("2026-11-12"), 1800)]);
        // Session sur trois jours.
        let parts = split_by_day(utc("2026-11-10T23:00:00Z"), utc("2026-11-13T03:00:00Z"), 52 * 3600, paris, 0);
        assert_eq!(parts.iter().map(|p| p.1).sum::<i64>(), 52 * 3600);
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn midnight_split_with_a_fixed_offset_time_zone() {
        let tokyo = FixedOffset::east_opt(9 * 3600).unwrap();
        // 23 h 30 → 0 h 30 heure de Tokyo, bascule à minuit.
        let parts = split_by_day(utc("2026-06-01T14:30:00Z"), utc("2026-06-01T15:30:00Z"), 3600, &tokyo, 0);
        assert_eq!(parts, vec![(d("2026-06-01"), 1800), (d("2026-06-02"), 1800)]);
    }

    #[test]
    fn daylight_saving_changes_are_handled() {
        let paris = &Paris;
        // Passage à l'heure d'été le 29 mars 2026 (2 h → 3 h) : le jour de jeu du 28 (bascule 4 h) ne dure que 23 h.
        let start_29 = day_start(d("2026-03-29"), paris, 4);
        assert_eq!(start_29, utc("2026-03-29T02:00:00Z"), "4 h heure d'été = 2 h UTC");
        assert_eq!((start_29 - day_start(d("2026-03-28"), paris, 4)).num_hours(), 23);
        // Retour à l'heure d'hiver le 25 octobre à 3 h : le jour de jeu du 24 (4 h → 4 h) dure 25 h, le 25 en dure 24.
        assert_eq!((day_start(d("2026-10-25"), paris, 4) - day_start(d("2026-10-24"), paris, 4)).num_hours(), 25);
        assert_eq!((day_start(d("2026-10-26"), paris, 4) - day_start(d("2026-10-25"), paris, 4)).num_hours(), 24);
        // Bascule à 2 h le jour où 2 h n'existe pas : la journée commence à 3 h.
        assert_eq!(day_start(d("2026-03-29"), paris, 2), utc("2026-03-29T01:00:00Z"));
        // Une session de 1 h à 1 h 30 → 3 h 30 (heure locale, 1 h réelle) le jour du changement, bascule à minuit : un seul jour.
        let parts = split_by_day(utc("2026-03-29T00:30:00Z"), utc("2026-03-29T01:30:00Z"), 3600, paris, 0);
        assert_eq!(parts, vec![(d("2026-03-29"), 3600)]);
    }

    #[test]
    fn hourly_split_uses_local_weekday_and_hour() {
        let paris = &Paris;
        // Lundi 16 novembre 2026, 22 h 30 → 23 h 30 à Paris.
        let parts = split_by_hour(utc("2026-11-16T21:30:00Z"), utc("2026-11-16T22:30:00Z"), 3600, paris);
        assert_eq!(parts, vec![(0, 22, 1800), (0, 23, 1800)]);
    }

    fn days(list: &[&str]) -> BTreeSet<NaiveDate> {
        list.iter().map(|s| d(s)).collect()
    }

    #[test]
    fn current_and_best_streaks() {
        let played = days(&["2026-11-01", "2026-11-02", "2026-11-03", "2026-11-04", "2026-11-10", "2026-11-11", "2026-11-12"]);
        let s = streaks(&played, d("2026-11-12"), false);
        assert_eq!((s.current, s.today_played, s.in_danger), (3, true, false));
        assert_eq!(s.current_start.as_deref(), Some("2026-11-10"));
        assert_eq!((s.best, s.best_start.as_deref(), s.best_end.as_deref()), (4, Some("2026-11-01"), Some("2026-11-04")));
    }

    #[test]
    fn streak_is_in_danger_until_the_rollover_when_today_is_not_played() {
        let played = days(&["2026-11-10", "2026-11-11"]);
        let s = streaks(&played, d("2026-11-12"), false);
        assert_eq!((s.current, s.in_danger), (2, true), "la série d'hier reste affichée, en danger");
        let s = streaks(&played, d("2026-11-13"), false);
        assert_eq!(s.current, 0, "un jour entier manqué : série perdue");
        assert_eq!(s.best, 2);
        assert_eq!(streaks(&BTreeSet::new(), d("2026-11-13"), false), Streaks::default());
    }

    #[test]
    fn jokers_save_a_missed_day() {
        // 7 jours d'affilée (1 joker gagné), un jour manqué le 8, puis on rejoue.
        let mut list: Vec<String> = (1..=7).map(|i| format!("2026-11-{i:02}")).collect();
        list.extend(["2026-11-09".into(), "2026-11-10".into()]);
        let played: BTreeSet<NaiveDate> = list.iter().map(|s| d(s)).collect();
        let without = streaks(&played, d("2026-11-10"), false);
        assert_eq!(without.current, 2);
        let with = streaks(&played, d("2026-11-10"), true);
        assert_eq!(with.current, 9, "le joker sauve le 8 novembre (qui ne compte pas comme joué)");
        assert_eq!(with.joker_days, vec!["2026-11-08".to_string()]);
        assert_eq!(with.jokers_available, 0);
        // Deux jours manqués d'affilée avec un seul joker : série perdue.
        let played = days(&["2026-11-01", "2026-11-02", "2026-11-03", "2026-11-04", "2026-11-05", "2026-11-06", "2026-11-07", "2026-11-10"]);
        assert_eq!(streaks(&played, d("2026-11-10"), true).current, 1);
        // Au plus deux jokers en réserve.
        let played: BTreeSet<NaiveDate> = (1..=28).map(|i| d(&format!("2026-11-{i:02}"))).collect();
        assert_eq!(streaks(&played, d("2026-11-28"), true).jokers_available, MAX_JOKERS);
    }

    #[test]
    fn threshold_decides_which_days_are_played() {
        let totals: BTreeMap<NaiveDate, i64> = [(d("2026-11-01"), 14 * 60), (d("2026-11-02"), 15 * 60), (d("2026-11-03"), 3 * 3600)].into_iter().collect();
        assert_eq!(played_days(&totals, 15), days(&["2026-11-02", "2026-11-03"]));
        assert_eq!(played_days(&totals, 60), days(&["2026-11-03"]));
    }

    #[test]
    fn summary_table_is_rebuilt_when_sessions_change_and_ignores_manual_bases() {
        let conn = db::open_in_memory().unwrap();
        conn.execute_batch(
            "INSERT INTO games(id, title, sort_title, added_at, manual_baseline_minutes, manual_baseline_at) VALUES('g', 'G', 'g', '2026-01-01T00:00:00Z', 6000, '2026-11-01T00:00:00Z');
             INSERT INTO game_sources(id, game_id, source, external_id, launch_method, launch_target) VALUES('s', 'g', 'local', 'x', 'exe', 'x');
             INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds) VALUES
               ('a', 's', '2026-11-12T02:00:00Z', '2026-11-12T04:00:00Z', 7200);",
        )
        .unwrap();
        let paris = &Paris;
        assert!(ensure_fresh(&conn, paris).unwrap());
        assert!(!ensure_fresh(&conn, paris).unwrap(), "rien n'a changé");
        let totals = day_totals(&conn, None).unwrap();
        assert_eq!(totals.values().sum::<i64>(), 7200, "la base manuelle de 100 h n'a pas de date : elle n'apparaît pas");
        assert_eq!(totals.get(&d("2026-11-11")), Some(&3600));
        conn.execute("INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds) VALUES('b', 's', '2026-11-13T20:00:00Z', '2026-11-13T21:00:00Z', 3600)", []).unwrap();
        assert!(ensure_fresh(&conn, paris).unwrap(), "nouvelle session : reconstruction");
        assert_eq!(day_totals(&conn, Some("g")).unwrap().get(&d("2026-11-13")), Some(&3600));
        db::set_setting(&conn, "streak_rollover_hour", "0").unwrap();
        assert!(ensure_fresh(&conn, paris).unwrap(), "autre heure de bascule : reconstruction");
        assert_eq!(day_totals(&conn, None).unwrap().get(&d("2026-11-12")), Some(&7200));
    }

    /// 3 ans d'historique (≈ 4 400 sessions) : reconstruction et lecture rapides.
    #[test]
    fn three_years_of_history_stay_fast() {
        let conn = db::open_in_memory().unwrap();
        conn.execute_batch(
            "INSERT INTO games(id, title, sort_title, added_at) VALUES('g1', 'A', 'a', '2023-01-01T00:00:00Z'), ('g2', 'B', 'b', '2023-01-01T00:00:00Z');
             INSERT INTO game_sources(id, game_id, source, external_id, launch_method, launch_target) VALUES('s1', 'g1', 'local', 'a', 'exe', 'a'), ('s2', 'g2', 'local', 'b', 'exe', 'b');",
        )
        .unwrap();
        let tx = conn.unchecked_transaction().unwrap();
        let base = utc("2023-11-18T18:00:00Z");
        for i in 0..1095i64 {
            for k in 0..4i64 {
                let start = base + Duration::days(i) + Duration::hours(k * 2);
                let secs = 1800 + (i * 37 + k * 11) % 3600;
                tx.execute(
                    "INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds) VALUES(?1, ?2, ?3, ?4, ?5)",
                    params![format!("{i}-{k}"), if k % 2 == 0 { "s1" } else { "s2" }, start.to_rfc3339(), (start + Duration::seconds(secs)).to_rfc3339(), secs],
                )
                .unwrap();
            }
        }
        tx.commit().unwrap();
        let t = std::time::Instant::now();
        ensure_fresh(&conn, &Paris).unwrap();
        let totals = day_totals(&conn, None).unwrap();
        let s = streaks(&played_days(&totals, 15), *totals.keys().last().unwrap(), true);
        let elapsed = t.elapsed();
        assert!(s.best >= 1000);
        assert!(elapsed < std::time::Duration::from_millis(1500), "reconstruction + séries : {elapsed:?}");
        let t = std::time::Instant::now();
        ensure_fresh(&conn, &Paris).unwrap();
        let _ = day_totals(&conn, None).unwrap();
        assert!(t.elapsed() < std::time::Duration::from_millis(150), "lecture sans reconstruction : {:?}", t.elapsed());
    }
}
