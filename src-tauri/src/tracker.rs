//! PlaytimeTracker (§7.4) : suivi maison des sessions de jeu par surveillance des processus.
//!
//! Aucune injection ni hook : on lit simplement la liste des processus toutes les 5 s,
//! et seulement pendant qu'un jeu lancé depuis l'application est suivi.
use crate::db::new_id;
use crate::drives::{is_inside, norm_path};
use crate::models::GameSource;
use crate::state::{AppState, Emitter};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::params;
use serde_json::json;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

pub const POLL_SECS: u64 = 5;
/// Délai laissé au jeu pour apparaître (launcher intermédiaire, mise à jour, connexion…).
pub const START_GRACE_DIRECT_SECS: u64 = 60;
pub const START_GRACE_LAUNCHER_SECS: u64 = 180;
/// Délai avant de clore la session quand plus aucun processus ne tourne (changement de processus).
pub const END_GRACE_SECS: u64 = 10;
pub const FLUSH_SECS: u64 = 60;

/// Chronologie d'une session, en secondes depuis le lancement. Logique pure, testée.
#[derive(Debug, Clone, PartialEq)]
pub struct Timeline {
    pub start_grace: u64,
    pub first_seen: Option<u64>,
    pub last_seen: u64,
    pub last_flush: u64,
}

#[derive(Debug, PartialEq)]
pub enum Step {
    /// Le jeu n'est pas encore apparu.
    Waiting,
    /// Première détection du processus.
    Started,
    /// Le jeu tourne ; `flush` demande d'enregistrer la progression.
    Running { flush: bool },
    /// Le processus a disparu mais le délai de grâce court encore.
    Ending,
    /// Le jeu n'est jamais apparu : session annulée.
    Abort,
    /// Session terminée.
    End,
}

impl Timeline {
    pub fn new(start_grace: u64) -> Self {
        Timeline { start_grace, first_seen: None, last_seen: 0, last_flush: 0 }
    }

    pub fn step(&mut self, running: bool, now: u64) -> Step {
        match (running, self.first_seen) {
            (true, None) => {
                // Lancement direct : on compte depuis le clic ; via un launcher : depuis l'apparition.
                let start = if now <= POLL_SECS * 2 { 0 } else { now };
                self.first_seen = Some(start);
                self.last_seen = now;
                self.last_flush = now;
                Step::Started
            }
            (true, Some(_)) => {
                self.last_seen = now;
                let flush = now.saturating_sub(self.last_flush) >= FLUSH_SECS;
                if flush {
                    self.last_flush = now;
                }
                Step::Running { flush }
            }
            (false, None) => {
                if now > self.start_grace {
                    Step::Abort
                } else {
                    Step::Waiting
                }
            }
            (false, Some(_)) => {
                if now.saturating_sub(self.last_seen) > END_GRACE_SECS {
                    Step::End
                } else {
                    Step::Ending
                }
            }
        }
    }

    /// Durée jouée, en secondes.
    pub fn duration(&self) -> u64 {
        self.first_seen.map(|s| self.last_seen.saturating_sub(s)).unwrap_or(0)
    }
}

/// Ce qu'il faut surveiller pour un jeu : son dossier (F-HEU-03) ou, à défaut, son exécutable.
#[derive(Debug, Clone)]
pub struct Watch {
    pub dir: Option<String>,
    pub exe: Option<String>,
}

impl Watch {
    pub fn for_source(g: &GameSource) -> Option<Watch> {
        let dir = g.install_path.as_deref().map(norm_path).filter(|d| d.len() > 3);
        let exe = g.executable_path.as_deref().map(norm_path);
        (dir.is_some() || exe.is_some()).then_some(Watch { dir, exe })
    }

    /// `exes` : chemins normalisés des exécutables des processus en cours.
    pub fn matches(&self, exes: &[String]) -> bool {
        exes.iter().any(|p| match (&self.dir, &self.exe) {
            (Some(dir), _) => is_inside(p, dir),
            (None, Some(exe)) => p == exe,
            (None, None) => false,
        })
    }
}

struct Tracked {
    game_id: String,
    source_id: String,
    session_id: String,
    watch: Watch,
    launched: std::time::Instant,
    launched_wall: DateTime<Utc>,
    timeline: Timeline,
}

fn iso(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Default)]
pub struct Tracker {
    active: Mutex<Vec<Tracked>>,
    wake: Condvar,
}

impl Tracker {
    pub fn new() -> Arc<Self> {
        Arc::new(Tracker::default())
    }

    pub fn running_games(&self) -> Vec<String> {
        self.active.lock().unwrap_or_else(|e| e.into_inner()).iter().map(|t| t.game_id.clone()).collect()
    }

    /// Démarre le suivi d'un jeu qui vient d'être lancé : la session est écrite tout de suite (§6.3).
    pub fn start(&self, state: &AppState, emit: &Emitter, g: &GameSource, launcher_mediated: bool) {
        let Some(watch) = Watch::for_source(g) else {
            log::info!("suivi impossible pour {} : ni dossier ni exécutable connus", g.external_id);
            return;
        };
        let mut active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        if active.iter().any(|t| t.source_id == g.id) {
            return; // déjà suivi
        }
        let now = Utc::now();
        let session_id = new_id();
        let inserted = state.conn().execute(
            "INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds, active) VALUES(?1, ?2, ?3, ?3, 0, 1)",
            params![session_id, g.id, iso(now)],
        );
        if let Err(e) = inserted {
            log::error!("suivi : création de la session impossible : {e}");
            return;
        }
        active.push(Tracked {
            game_id: g.game_id.clone(),
            source_id: g.id.clone(),
            session_id,
            watch,
            launched: std::time::Instant::now(),
            launched_wall: now,
            timeline: Timeline::new(if launcher_mediated { START_GRACE_LAUNCHER_SECS } else { START_GRACE_DIRECT_SECS }),
        });
        emit("running-changed", json!(active.iter().map(|t| t.game_id.clone()).collect::<Vec<_>>()));
        drop(active);
        self.wake.notify_all();
    }

    /// Boucle de surveillance : dort sans consommer de CPU tant qu'aucun jeu n'est suivi.
    pub fn run_loop(self: Arc<Self>, state: Arc<AppState>, emit: Emitter) {
        let mut system: Option<sysinfo::System> = None;
        loop {
            {
                let mut guard = self.active.lock().unwrap_or_else(|e| e.into_inner());
                while guard.is_empty() {
                    system = None; // libère la mémoire de la liste de processus
                    guard = self.wake.wait(guard).unwrap_or_else(|e| e.into_inner());
                }
            }
            std::thread::sleep(Duration::from_secs(POLL_SECS));
            let exes = running_exes(system.get_or_insert_with(sysinfo::System::new));
            self.tick(&state, &emit, &exes);
        }
    }

    fn tick(&self, state: &AppState, emit: &Emitter, exes: &[String]) {
        let mut active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        let before = active.len();
        let mut sessions_changed = false;
        active.retain_mut(|t| {
            let now = t.launched.elapsed().as_secs();
            let step = t.timeline.step(t.watch.matches(exes), now);
            let conn = state.conn();
            let wall = |secs: u64| iso(t.launched_wall + chrono::Duration::seconds(secs as i64));
            let result = match step {
                Step::Waiting | Step::Ending | Step::Running { flush: false } => Ok(0),
                Step::Started => conn.execute(
                    "UPDATE play_sessions SET started_at = ?2, ended_at = ?3 WHERE id = ?1",
                    params![t.session_id, wall(t.timeline.first_seen.unwrap_or(0)), wall(t.timeline.last_seen)],
                ),
                Step::Running { flush: true } => conn.execute(
                    "UPDATE play_sessions SET ended_at = ?2, duration_seconds = ?3 WHERE id = ?1",
                    params![t.session_id, wall(t.timeline.last_seen), t.timeline.duration() as i64],
                ),
                Step::Abort => {
                    log::info!("suivi : le jeu {} n'est pas apparu, session annulée", t.game_id);
                    conn.execute("DELETE FROM play_sessions WHERE id = ?1", params![t.session_id])
                }
                Step::End => {
                    log::info!("suivi : session de {} s pour {}", t.timeline.duration(), t.game_id);
                    sessions_changed = true;
                    conn.execute(
                        "UPDATE play_sessions SET ended_at = ?2, duration_seconds = ?3, active = 0 WHERE id = ?1",
                        params![t.session_id, wall(t.timeline.last_seen), t.timeline.duration() as i64],
                    )
                }
            };
            if let Err(e) = result {
                log::error!("suivi : écriture de la session impossible : {e}");
            }
            !matches!(step, Step::Abort | Step::End)
        });
        if active.len() != before {
            emit("running-changed", json!(active.iter().map(|t| t.game_id.clone()).collect::<Vec<_>>()));
        }
        drop(active);
        if sessions_changed {
            emit("library-updated", json!(null));
        }
    }
}

/// Chemins normalisés des exécutables de tous les processus visibles.
pub fn running_exes(system: &mut sysinfo::System) -> Vec<String> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet));
    system.processes().values().filter_map(|p| p.exe()).map(|p| norm_path(&p.to_string_lossy())).collect()
}

/// Au démarrage : clôt les sessions restées « en cours » après un plantage (au plus une minute perdue).
pub fn close_orphan_sessions(conn: &rusqlite::Connection) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM play_sessions WHERE active = 1 AND duration_seconds = 0", [])?;
    conn.execute("UPDATE play_sessions SET active = 0 WHERE active = 1", [])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_launch_is_counted_from_the_click() {
        let mut t = Timeline::new(START_GRACE_DIRECT_SECS);
        assert_eq!(t.step(true, 5), Step::Started);
        assert_eq!(t.first_seen, Some(0));
        let mut flushes = 0;
        for now in (10..=600).step_by(5) {
            match t.step(true, now) {
                Step::Running { flush } => flushes += flush as u32,
                other => panic!("étape inattendue : {other:?}"),
            }
        }
        assert_eq!(flushes, 9, "une écriture par minute");
        // Le jeu se ferme : délai de grâce de 10 s puis clôture.
        assert_eq!(t.step(false, 605), Step::Ending);
        assert_eq!(t.step(false, 610), Step::Ending);
        assert_eq!(t.step(false, 615), Step::End);
        assert_eq!(t.duration(), 600, "10 minutes de jeu comptées à la seconde près");
    }

    #[test]
    fn intermediate_launcher_and_process_switch() {
        let mut t = Timeline::new(START_GRACE_LAUNCHER_SECS);
        // Le launcher met 40 s à démarrer le jeu.
        for now in (5..40).step_by(5) {
            assert_eq!(t.step(false, now), Step::Waiting);
        }
        assert_eq!(t.step(true, 40), Step::Started);
        assert_eq!(t.first_seen, Some(40));
        // launcher.exe se ferme, game.exe démarre 5 s plus tard : la session continue.
        assert_eq!(t.step(false, 45), Step::Ending);
        assert_eq!(t.step(true, 50), Step::Running { flush: false });
        assert_eq!(t.step(false, 55), Step::Ending);
        assert_eq!(t.step(false, 65), Step::End);
        assert_eq!(t.duration(), 10);
    }

    #[test]
    fn game_that_never_appears_is_aborted() {
        let mut t = Timeline::new(START_GRACE_DIRECT_SECS);
        assert_eq!(t.step(false, 60), Step::Waiting);
        assert_eq!(t.step(false, 65), Step::Abort);
    }

    #[test]
    fn watch_matches_any_process_in_game_folder() {
        let w = Watch { dir: Some("d:\\jeux\\foo".into()), exe: Some("d:\\jeux\\foo\\launcher.exe".into()) };
        assert!(w.matches(&["c:\\windows\\explorer.exe".into(), "d:\\jeux\\foo\\bin\\game.exe".into()]));
        assert!(!w.matches(&["d:\\jeux\\foobar\\game.exe".into()]));
        let exe_only = Watch { dir: None, exe: Some("e:\\tiny.exe".into()) };
        assert!(exe_only.matches(&["e:\\tiny.exe".into()]));
        assert!(!exe_only.matches(&["e:\\other.exe".into()]));
    }

    #[test]
    fn sees_current_process() {
        let mut sys = sysinfo::System::new();
        let exes = running_exes(&mut sys);
        let me = norm_path(&std::env::current_exe().unwrap().to_string_lossy());
        assert!(exes.contains(&me), "le processus de test doit être visible");
    }

    /// Bout en bout avec un vrai processus (~35 s) : `cargo test -- --ignored`.
    /// Un « jeu » ajouté à la main est lancé, suivi, puis sa session est close avec la bonne durée.
    #[test]
    #[ignore]
    fn tracks_a_real_process_end_to_end() {
        use crate::drives::DriveSnapshot;
        use crate::providers::local::detected_from_exe;
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("Faux Jeu");
        std::fs::create_dir_all(&game_dir).unwrap();
        let exe = game_dir.join("fauxjeu.exe");
        std::fs::copy("C:\\Windows\\System32\\PING.EXE", &exe).unwrap();

        let state = Arc::new(AppState::new(crate::db::open_in_memory().unwrap(), tmp.path().to_path_buf()));
        let detected = detected_from_exe(&exe.to_string_lossy(), None, None, Some("-n 16 127.0.0.1"), None, &[]);
        crate::library::upsert_detected(&state.conn(), &detected, &DriveSnapshot::current()).unwrap();
        let source = crate::db::all_sources(&state.conn()).unwrap().remove(0);

        let emit: Emitter = Arc::new(|_, _| {});
        let tracker = state.tracker.clone();
        {
            let (tracker, state, emit) = (tracker.clone(), state.clone(), emit.clone());
            std::thread::spawn(move || tracker.run_loop(state, emit));
        }
        crate::launch::launch_source(&source).unwrap();
        tracker.start(&state, &emit, &source, false);
        assert_eq!(tracker.running_games().len(), 1, "indicateur « En cours » dès le lancement");

        // ping -n 16 dure ~15 s ; la session doit se clore peu après.
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        while !tracker.running_games().is_empty() {
            assert!(std::time::Instant::now() < deadline, "la session ne s'est pas close");
            std::thread::sleep(Duration::from_secs(1));
        }
        let (duration, active): (i64, bool) =
            state.conn().query_row("SELECT duration_seconds, active FROM play_sessions", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert!(!active);
        assert!((10..=22).contains(&duration), "durée suivie : {duration} s pour ~15 s de jeu");
        let game = &crate::library::list_games(&state.conn()).unwrap()[0];
        assert_eq!(game.tracked_seconds, duration);
        assert!(game.last_played.is_some());
    }

    #[test]
    fn orphan_sessions_are_closed() {
        let conn = crate::db::open_in_memory().unwrap();
        conn.execute_batch(
            "INSERT INTO games(id, title, sort_title, added_at) VALUES('g', 'G', 'g', '2026-01-01T00:00:00Z');
             INSERT INTO game_sources(id, game_id, source, external_id, launch_method, launch_target) VALUES('s', 'g', 'local', 'x', 'exe', 'x');
             INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds, active) VALUES
               ('a', 's', '2026-01-01T10:00:00Z', '2026-01-01T10:07:00Z', 420, 1),
               ('b', 's', '2026-01-02T10:00:00Z', '2026-01-02T10:00:00Z', 0, 1);",
        )
        .unwrap();
        close_orphan_sessions(&conn).unwrap();
        let rows: Vec<(String, i64, bool)> = conn
            .prepare("SELECT id, duration_seconds, active FROM play_sessions")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(rows, vec![("a".to_string(), 420, false)]);
    }
}
