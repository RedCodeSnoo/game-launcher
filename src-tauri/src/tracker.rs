//! Suivi du temps de jeu (§7.4, E1, E4) : un `ProcessWatcher` qui observe les processus en permanence.
//!
//! Aucune injection ni hook, aucun droit administrateur : la liste des processus est lue toutes les 3 s
//! (voir `procs.rs`) et comparée à un index en mémoire des jeux connus. Une session est ouverte dès qu'un
//! processus d'un jeu apparaît — que le jeu ait été lancé depuis l'application ou non — et close quand
//! plus aucun processus *du jeu* ne tourne (les processus annexes de la liste d'exclusion ne comptent pas).
use crate::db::{self, new_id};
use crate::drives::{is_inside, norm_path};
use crate::error::AppError;
use crate::models::{GameSource, SourceId};
use crate::procs::{self, KillReport, ProcInfo, ProcessLister};
use crate::state::{AppState, Emitter};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Connection};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

/// Intervalle d'observation : un jeu est détecté en 3 s au plus (E4-02 : moins de 10 s).
pub const POLL_SECS: u64 = 3;
/// Délai laissé à un jeu lancé depuis l'application pour apparaître (launcher intermédiaire, mise à jour…).
pub const START_GRACE_DIRECT_SECS: i64 = 60;
pub const START_GRACE_LAUNCHER_SECS: i64 = 180;
/// Une session ouverte depuis moins longtemps que cela au moment du clic sur « Jouer » vient de ce clic.
const JUST_LAUNCHED_SECS: i64 = 15;
/// Délai avant de clore la session quand plus aucun processus du jeu ne tourne (changement de processus).
pub const END_GRACE_SECS: i64 = 10;
/// Battement : la session en cours est enregistrée en base toutes les 60 s (E4-09).
pub const FLUSH_SECS: i64 = 60;
/// Un trou de plus de 5 min entre deux observations = mise en veille : la session est coupée (E4-09).
pub const SLEEP_GAP_SECS: i64 = 300;
/// Sessions plus courtes ignorées (lancement raté), réglable (E4-07).
pub const DEFAULT_MIN_SESSION_SECS: i64 = 60;
/// L'index est reconstruit à chaque modification de la bibliothèque et, par sécurité, au moins chaque minute.
const INDEX_MAX_AGE_SECS: i64 = 60;

fn iso(secs: i64) -> String {
    DateTime::<Utc>::from_timestamp(secs, 0).unwrap_or_default().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_iso(s: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(s).ok().map(|d| d.timestamp())
}

// ---------------------------------------------------------------------------
// Index des jeux connus (E4-04)
// ---------------------------------------------------------------------------

/// Exécutables principaux connus pour les cas difficiles (tableau du document d'évolutions, complété).
/// Ils servent à reconnaître un jeu dont le chemin du processus est illisible (anti-triche, E4-05).
pub fn known_exe_names(source: SourceId, external_id: &str) -> &'static [&'static str] {
    let id = external_id.to_ascii_lowercase();
    match source {
        SourceId::Riot => {
            if id.starts_with("league_of_legends") {
                &["league of legends.exe"]
            } else if id.starts_with("valorant") {
                &["valorant-win64-shipping.exe"]
            } else if id.starts_with("bacon") {
                &["lor.exe"]
            } else {
                &[]
            }
        }
        SourceId::Battlenet => match id.as_str() {
            x if x.starts_with("wow") => &["wow.exe", "wowclassic.exe", "wowt.exe", "wowb.exe"],
            "pro" => &["overwatch.exe"],
            "fenris" => &["diablo iv.exe"],
            "d3" => &["diablo iii64.exe", "diablo iii.exe"],
            "osi" => &["d2r.exe"],
            "hero" => &["heroesofthestorm_x64.exe"],
            "s2" => &["sc2_x64.exe"],
            "s1" => &["starcraft.exe"],
            "w3" => &["warcraft iii.exe"],
            "wtcg" => &["hearthstone.exe"],
            _ => &[],
        },
        _ => &[],
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexEntry {
    pub source_id: String,
    pub game_id: String,
    pub title: String,
    /// Dossier d'installation normalisé (personnalisé ou détecté).
    pub dir: Option<String>,
    /// Exécutable principal normalisé (personnalisé ou détecté).
    pub exe: Option<String>,
    /// Noms d'exécutable (minuscules) reconnus quand le chemin du processus est illisible.
    pub names: Vec<String>,
}

/// Table de hachage chemin normalisé → jeu, plus les dossiers d'installation (du plus long au plus court).
#[derive(Debug, Default)]
pub struct GameIndex {
    pub entries: Vec<IndexEntry>,
    by_exe: HashMap<String, usize>,
    by_name: HashMap<String, usize>,
    dirs: Vec<(String, usize)>,
    extra_exclusions: Vec<String>,
}

impl GameIndex {
    pub fn new(entries: Vec<IndexEntry>, extra_exclusions: Vec<String>) -> Self {
        let mut index = GameIndex { extra_exclusions, ..Default::default() };
        for (i, e) in entries.iter().enumerate() {
            if let Some(exe) = &e.exe {
                index.by_exe.entry(exe.clone()).or_insert(i);
            }
            for name in &e.names {
                index.by_name.entry(name.clone()).or_insert(i);
            }
            // Une racine de disque n'identifie pas un jeu.
            if let Some(dir) = e.dir.as_ref().filter(|d| d.len() > 3) {
                if !index.dirs.iter().any(|(known, _)| known == dir) {
                    index.dirs.push((dir.clone(), i));
                }
            }
        }
        index.dirs.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        index.entries = entries;
        index
    }

    /// Jeux installés et suivis, lus dans la base. Les jeux d'un disque débranché restent dans l'index (E4-14).
    pub fn from_db(conn: &Connection, disabled: &HashSet<String>) -> rusqlite::Result<Self> {
        let extra: Vec<String> = db::get_setting(conn, "tracking_exclusions")
            .unwrap_or_default()
            .split(|c| c == '\n' || c == ',' || c == ';')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        let mut stmt = conn.prepare(
            "SELECT gs.id, gs.game_id, g.title, gs.source, gs.external_id,
                    COALESCE(gs.install_path_override, gs.install_path), COALESCE(gs.executable_path_override, gs.executable_path)
             FROM game_sources gs JOIN games g ON g.id = gs.game_id
             WHERE gs.installed = 1 AND g.no_tracking = 0",
        )?;
        let mut entries = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            let source: String = r.get(3)?;
            if disabled.contains(&source) {
                continue;
            }
            let external_id: String = r.get(4)?;
            let dir: Option<String> = r.get(5)?;
            let exe: Option<String> = r.get(6)?;
            let exe = exe.map(|e| norm_path(&e));
            let mut names: Vec<String> = exe.iter().filter_map(|e| e.rsplit('\\').next()).map(|n| n.to_string()).collect();
            if let Some(id) = SourceId::parse(&source) {
                names.extend(known_exe_names(id, &external_id).iter().map(|n| n.to_string()));
            }
            entries.push(IndexEntry { source_id: r.get(0)?, game_id: r.get(1)?, title: r.get(2)?, dir: dir.map(|d| norm_path(&d)), exe, names });
        }
        Ok(GameIndex::new(entries, extra))
    }

    /// Jeu auquel appartient un processus (indice dans `entries`), ou `None`.
    pub fn match_proc(&self, p: &ProcInfo) -> Option<usize> {
        match &p.path {
            Some(path) => {
                // 1. Exécutable principal : correspondance directe (même s'il porte un nom « launcher »).
                if let Some(&i) = self.by_exe.get(path) {
                    return Some(i);
                }
                // 2. Exécutable situé dans le dossier d'un jeu et absent de la liste d'exclusion.
                let (_, i) = self.dirs.iter().find(|(dir, _)| is_inside(path, dir))?;
                (!procs::is_excluded(&p.name, &self.extra_exclusions)).then_some(*i)
            }
            // 3. Processus protégé (chemin illisible) : comparaison par nom d'exécutable (E4-05).
            None => self.by_name.get(&p.name).copied(),
        }
    }

    /// Processus à terminer pour « Forcer la fermeture » : ceux du jeu, tout ce qui tourne depuis son dossier
    /// (y compris les processus annexes), et leurs descendants dont le chemin est illisible ou dans le dossier.
    pub fn kill_targets(&self, procs_list: &[ProcInfo], game_id: &str) -> Vec<ProcInfo> {
        let dirs: Vec<&String> = self.entries.iter().filter(|e| e.game_id == game_id).filter_map(|e| e.dir.as_ref()).filter(|d| d.len() > 3).collect();
        let in_game_dir = |p: &ProcInfo| p.path.as_ref().map_or(false, |path| dirs.iter().any(|d| is_inside(path, d)));
        let mut targets: Vec<ProcInfo> = procs_list
            .iter()
            .filter(|p| in_game_dir(p) || self.match_proc(p).map_or(false, |i| self.entries[i].game_id == game_id))
            .cloned()
            .collect();
        let roots: Vec<u32> = targets.iter().map(|p| p.pid).collect();
        for child in procs::descendants(procs_list, &roots) {
            if child.path.is_none() || in_game_dir(child) {
                targets.push(child.clone());
            }
        }
        targets
    }
}

// ---------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Lancée depuis l'application.
    App,
    /// Détectée : jeu lancé depuis son launcher, un raccourci…
    Detected,
}

impl Origin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Origin::App => "app",
            Origin::Detected => "detected",
        }
    }
}

/// Jeu en cours, pour l'interface et la zone de notification.
#[derive(Debug, Clone, PartialEq)]
pub struct RunningInfo {
    pub game_id: String,
    pub title: String,
    /// Début de la session (secondes Unix) ; `None` tant que le jeu lancé n'est pas apparu.
    pub started_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Config {
    /// Suivre aussi les jeux lancés en dehors de l'application (E4-11).
    pub track_external: bool,
    pub min_session_secs: i64,
    pub notify_detected: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config { track_external: true, min_session_secs: DEFAULT_MIN_SESSION_SECS, notify_detected: false }
    }
}

impl Config {
    pub fn load(conn: &Connection) -> Self {
        Config {
            track_external: db::get_setting(conn, "track_external").as_deref() != Some("0"),
            min_session_secs: db::get_setting(conn, "min_session_secs").and_then(|v| v.trim().parse::<i64>().ok()).map(|v| v.clamp(0, 3600)).unwrap_or(DEFAULT_MIN_SESSION_SECS),
            notify_detected: db::get_setting(conn, "notify_detected").as_deref() == Some("1"),
        }
    }
}

struct Live {
    source_id: String,
    game_id: String,
    title: String,
    session_id: String,
    started: i64,
    last_seen: i64,
    last_flush: i64,
}

/// Jeu lancé depuis l'application dont le processus n'est pas encore apparu.
struct Pending {
    source_id: String,
    game_id: String,
    title: String,
    launched: i64,
    grace: i64,
}

#[derive(Default)]
struct Inner {
    live: Vec<Live>,
    pending: Vec<Pending>,
    /// Sources dont le suivi a été arrêté à la main : ignorées tant que leurs processus tournent encore.
    muted: HashSet<String>,
    index: Arc<GameIndex>,
    config: Config,
    index_built: Option<i64>,
    last_tick: Option<i64>,
    /// Les sessions restées « en cours » en base ont été reprises ou closes (première observation).
    adopted: bool,
}

type Events = Vec<(&'static str, serde_json::Value)>;

#[derive(Default)]
pub struct Tracker {
    inner: Mutex<Inner>,
    wake: Condvar,
    dirty: AtomicBool,
}

fn running_ids(inner: &Inner) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for id in inner.live.iter().map(|l| &l.game_id).chain(inner.pending.iter().map(|p| &p.game_id)) {
        if !ids.contains(id) {
            ids.push(id.clone());
        }
    }
    ids
}

/// Clôt une session en base. Trop courte, elle est supprimée (E4-07). Renvoie vrai si elle est conservée.
fn finalize(conn: &Connection, session_id: &str, started: i64, end: i64, min_secs: i64) -> bool {
    let duration = (end - started).max(0);
    let result = if duration < min_secs {
        conn.execute("DELETE FROM play_sessions WHERE id = ?1", params![session_id])
    } else {
        conn.execute("UPDATE play_sessions SET ended_at = ?2, duration_seconds = ?3, active = 0 WHERE id = ?1", params![session_id, iso(end), duration])
    };
    if let Err(e) = result {
        log::error!("suivi : clôture de la session impossible : {e}");
    }
    duration >= min_secs
}

impl Tracker {
    pub fn new() -> Arc<Self> {
        Arc::new(Tracker::default())
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// La bibliothèque ou les réglages ont changé : l'index sera reconstruit à la prochaine observation.
    pub fn invalidate(&self) {
        self.dirty.store(true, Ordering::SeqCst);
        self.wake.notify_all();
    }

    pub fn running_games(&self) -> Vec<String> {
        running_ids(&self.lock())
    }

    pub fn running_info(&self) -> Vec<RunningInfo> {
        let inner = self.lock();
        let mut out: Vec<RunningInfo> = inner.live.iter().map(|l| RunningInfo { game_id: l.game_id.clone(), title: l.title.clone(), started_at: Some(l.started) }).collect();
        for p in &inner.pending {
            if !out.iter().any(|r| r.game_id == p.game_id) {
                out.push(RunningInfo { game_id: p.game_id.clone(), title: p.title.clone(), started_at: None });
            }
        }
        out
    }

    /// Un jeu vient d'être lancé depuis l'application : il est « En cours » tout de suite, et sa session
    /// sera marquée « lancée depuis l'app » quand son processus apparaîtra. Sans effet si le jeu est déjà suivi (E4-06).
    pub fn start(&self, state: &AppState, emit: &Emitter, g: &GameSource, launcher_mediated: bool) {
        if g.effective_dir().is_none() && g.effective_exe().is_none() {
            log::info!("suivi impossible pour {} : ni dossier ni exécutable connus", g.external_id);
            return;
        }
        let title: String = state.conn().query_row("SELECT title FROM games WHERE id = ?1", params![g.game_id], |r| r.get(0)).unwrap_or_default();
        let now = Utc::now().timestamp();
        let ids = {
            let mut inner = self.lock();
            inner.muted.remove(&g.id);
            if let Some(l) = inner.live.iter().find(|l| l.source_id == g.id) {
                // L'observation a pu voir le processus avant que ce lancement soit enregistré : la session
                // toute récente est bien « lancée depuis l'app ». Une partie déjà en cours reste telle quelle.
                if now - l.started <= JUST_LAUNCHED_SECS {
                    let _ = state.conn().execute("UPDATE play_sessions SET origin = 'app' WHERE id = ?1", params![l.session_id]);
                }
                return;
            }
            let grace = if launcher_mediated { START_GRACE_LAUNCHER_SECS } else { START_GRACE_DIRECT_SECS };
            match inner.pending.iter_mut().find(|p| p.source_id == g.id) {
                Some(p) => p.launched = now,
                None => inner.pending.push(Pending { source_id: g.id.clone(), game_id: g.game_id.clone(), title, launched: now, grace }),
            }
            running_ids(&inner)
        };
        // La source vient peut-être d'être modifiée (exécutable personnalisé) : index à jour avant d'observer.
        self.invalidate();
        emit("running-changed", json!(ids));
    }

    /// « Arrêter le suivi » (E1-01) : la session est close maintenant, sans toucher aux processus.
    /// Le jeu est ensuite ignoré tant que ses processus tournent, pour ne pas rouvrir une session aussitôt.
    pub fn stop_tracking(&self, state: &AppState, emit: &Emitter, game_id: &str) -> bool {
        let now = Utc::now().timestamp();
        let (ids, stopped) = {
            let mut inner = self.lock();
            let inner = &mut *inner;
            let mut stopped = false;
            let conn = state.conn();
            for l in inner.live.iter().filter(|l| l.game_id == game_id) {
                finalize(&conn, &l.session_id, l.started, now, 0);
                inner.muted.insert(l.source_id.clone());
                log::info!("suivi : arrêt manuel pour {} après {} s", l.title, now - l.started);
                stopped = true;
            }
            for p in inner.pending.iter().filter(|p| p.game_id == game_id) {
                inner.muted.insert(p.source_id.clone());
                stopped = true;
            }
            inner.live.retain(|l| l.game_id != game_id);
            inner.pending.retain(|p| p.game_id != game_id);
            (running_ids(inner), stopped)
        };
        emit("running-changed", json!(ids));
        emit("library-updated", json!(null));
        stopped
    }

    /// « Forcer la fermeture du jeu » (E1-02) : termine les processus du jeu puis clôt la session.
    /// Si un processus résiste (droits insuffisants, anti-triche), la session n'est pas touchée et l'erreur
    /// `KILL_FAILED` nomme les processus restants (E1-03).
    pub fn force_close(&self, state: &AppState, emit: &Emitter, game_id: &str) -> Result<KillReport, AppError> {
        let index = self.current_index(state);
        let snapshot = ProcessLister::new().snapshot();
        let targets = index.kill_targets(&snapshot, game_id);
        let report = procs::kill(&targets);
        log::info!("fermeture forcée de {game_id} : {} processus terminé(s), {} en échec", report.killed, report.failed.len());
        if !report.failed.is_empty() {
            return Err(AppError::new("KILL_FAILED", report.failed.join(", ")));
        }
        self.stop_tracking(state, emit, game_id);
        Ok(report)
    }

    fn current_index(&self, state: &AppState) -> Arc<GameIndex> {
        self.refresh_index(state, Utc::now().timestamp(), true);
        self.lock().index.clone()
    }

    /// Reconstruit l'index et relit les réglages si nécessaire.
    fn refresh_index(&self, state: &AppState, now: i64, force: bool) {
        let dirty = self.dirty.swap(false, Ordering::SeqCst);
        let stale = self.lock().index_built.map_or(true, |built| now - built >= INDEX_MAX_AGE_SECS || now < built);
        if !(dirty || stale || force) {
            return;
        }
        let disabled = state.disabled_sources();
        let (index, config) = {
            let conn = state.conn();
            (GameIndex::from_db(&conn, &disabled), Config::load(&conn))
        };
        let mut inner = self.lock();
        match index {
            Ok(index) => inner.index = Arc::new(index),
            Err(e) => log::error!("suivi : index des jeux illisible : {e}"),
        }
        inner.config = config;
        inner.index_built = Some(now);
    }

    /// Boucle du `ProcessWatcher`. Avec le suivi des jeux externes désactivé, elle dort sans rien consommer
    /// tant qu'aucun jeu lancé depuis l'application n'est suivi.
    pub fn run_loop(self: Arc<Self>, state: Arc<AppState>, emit: Emitter) {
        let mut lister = ProcessLister::new();
        loop {
            let now = Utc::now().timestamp();
            self.refresh_index(&state, now, false);
            let snapshot = lister.snapshot();
            for (event, payload) in self.tick(&state, &snapshot, now) {
                emit(event, payload);
            }
            let guard = self.lock();
            let idle = !guard.config.track_external && guard.live.is_empty() && guard.pending.is_empty();
            if idle {
                lister = ProcessLister::new(); // libère le cache des processus
                let mut guard = guard;
                while guard.pending.is_empty() && !self.dirty.load(Ordering::SeqCst) {
                    guard = self.wake.wait(guard).unwrap_or_else(|e| e.into_inner());
                }
            } else {
                let _ = self.wake.wait_timeout(guard, Duration::from_secs(POLL_SECS));
            }
        }
    }

    /// Une observation : `snapshot` est la liste des processus, `now` l'heure (secondes Unix).
    /// Renvoie les événements à envoyer à l'interface (émis par l'appelant, hors verrou).
    pub fn tick(&self, state: &AppState, snapshot: &[ProcInfo], now: i64) -> Events {
        let mut events: Events = Vec::new();
        let mut inner = self.lock();
        let inner = &mut *inner;
        let index = inner.index.clone();
        let cfg = inner.config.clone();
        let conn = state.conn();
        let before = running_ids(inner);
        let mut sessions_changed = false;

        // Processus de jeux connus : source → (plus ancienne heure de création connue).
        let mut matched: HashMap<&str, Option<i64>> = HashMap::new();
        for p in snapshot {
            if let Some(i) = index.match_proc(p) {
                let slot = matched.entry(index.entries[i].source_id.as_str()).or_insert(p.started);
                *slot = match (*slot, p.started) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (a, b) => a.or(b),
                };
            }
        }

        // Mise en veille ou arrêt prolongé de l'observation : les sessions s'arrêtent à leur dernière observation,
        // le temps de veille n'est pas compté ; un jeu encore ouvert au réveil démarre une nouvelle session.
        let mut min_start = i64::MIN;
        if let Some(prev) = inner.last_tick {
            if now - prev > SLEEP_GAP_SECS {
                log::info!("suivi : {} s sans observation (mise en veille), sessions en cours closes", now - prev);
                for l in inner.live.drain(..) {
                    finalize(&conn, &l.session_id, l.started, l.last_seen, cfg.min_session_secs);
                    sessions_changed = true;
                }
                min_start = now;
            }
        }
        inner.last_tick = Some(now);

        // Première observation : sessions restées « en cours » en base (plantage, arrêt du PC, redémarrage de
        // l'application). Jeu encore ouvert et battement récent → la session reprend ; sinon elle est close à
        // son dernier battement (E1-06).
        if !inner.adopted {
            inner.adopted = true;
            let rows: Vec<(String, String, String, String)> = conn
                .prepare("SELECT id, game_source_id, started_at, ended_at FROM play_sessions WHERE active = 1")
                .and_then(|mut stmt| stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?.collect())
                .unwrap_or_default();
            for (session_id, source_id, started_at, ended_at) in rows {
                let started = parse_iso(&started_at).unwrap_or(now);
                let beat = parse_iso(&ended_at).unwrap_or(started);
                let entry = index.entries.iter().find(|e| e.source_id == source_id);
                let resumable = matched.contains_key(source_id.as_str()) && now - beat <= SLEEP_GAP_SECS && !inner.live.iter().any(|l| l.source_id == source_id);
                match entry {
                    Some(e) if resumable => {
                        log::info!("suivi : session de {} reprise après redémarrage", e.title);
                        inner.live.push(Live { source_id, game_id: e.game_id.clone(), title: e.title.clone(), session_id, started, last_seen: now, last_flush: beat });
                    }
                    _ => {
                        log::info!("suivi : session restée ouverte close à son dernier battement ({} s)", beat - started);
                        finalize(&conn, &session_id, started, beat, cfg.min_session_secs);
                        sessions_changed = true;
                    }
                }
            }
        }

        // Jeux dont un processus tourne.
        for (source_id, created) in &matched {
            if inner.muted.contains(*source_id) {
                continue;
            }
            if let Some(l) = inner.live.iter_mut().find(|l| l.source_id == *source_id) {
                l.last_seen = now;
                if now - l.last_flush >= FLUSH_SECS {
                    l.last_flush = now;
                    if let Err(e) = conn.execute("UPDATE play_sessions SET ended_at = ?2, duration_seconds = ?3 WHERE id = ?1", params![l.session_id, iso(now), (now - l.started).max(0)]) {
                        log::error!("suivi : battement non enregistré : {e}");
                    }
                    events.push(("session-heartbeat", json!(null)));
                }
                continue;
            }
            let pending = inner.pending.iter().position(|p| p.source_id == *source_id);
            if pending.is_none() && !cfg.track_external {
                continue;
            }
            let Some(entry) = index.entries.iter().find(|e| e.source_id == *source_id) else { continue };
            // Début = création du processus (jeu déjà ouvert au démarrage de l'application, E4-08), sans jamais
            // chevaucher la session précédente du même jeu ni précéder un réveil de veille.
            let last_end: Option<String> = conn.query_row("SELECT MAX(ended_at) FROM play_sessions WHERE game_source_id = ?1", params![source_id], |r| r.get(0)).unwrap_or(None);
            let mut start = created.unwrap_or(now).min(now).max(min_start).max(last_end.as_deref().and_then(parse_iso).unwrap_or(i64::MIN)).min(now);
            let origin = match pending {
                Some(i) => {
                    let p = inner.pending.remove(i);
                    if !cfg.track_external {
                        start = start.max(p.launched).min(now);
                    }
                    Origin::App
                }
                None => Origin::Detected,
            };
            let session_id = new_id();
            let inserted = conn.execute(
                "INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds, active, origin) VALUES(?1, ?2, ?3, ?4, ?5, 1, ?6)",
                params![session_id, source_id, iso(start), iso(now), now - start, origin.as_str()],
            );
            if let Err(e) = inserted {
                log::error!("suivi : création de la session impossible : {e}");
                continue;
            }
            log::info!("suivi : session {} pour {}", if origin == Origin::App { "lancée depuis l'application" } else { "détectée" }, entry.title);
            if origin == Origin::Detected && cfg.notify_detected {
                events.push(("session-detected", json!({ "title": entry.title })));
            }
            inner.live.push(Live { source_id: source_id.to_string(), game_id: entry.game_id.clone(), title: entry.title.clone(), session_id, started: start, last_seen: now, last_flush: now });
        }

        // Arrêt manuel : le jeu redevient détectable une fois ses processus terminés.
        inner.muted.retain(|source_id| matched.contains_key(source_id.as_str()));

        // Jeux dont plus aucun processus ne tourne : clôture après le délai de grâce.
        inner.live.retain(|l| {
            if matched.contains_key(l.source_id.as_str()) || now - l.last_seen <= END_GRACE_SECS {
                return true;
            }
            let kept = finalize(&conn, &l.session_id, l.started, l.last_seen, cfg.min_session_secs);
            if kept {
                events.push(("session-ended", json!({ "sessionId": l.session_id, "gameId": l.game_id, "title": l.title, "seconds": l.last_seen - l.started })));
            }
            log::info!("suivi : session de {} s pour {}{}", l.last_seen - l.started, l.title, if kept { "" } else { " (trop courte, ignorée)" });
            sessions_changed = true;
            false
        });

        // Jeux lancés depuis l'application qui ne sont jamais apparus.
        inner.pending.retain(|p| {
            let waiting = now - p.launched <= p.grace;
            if !waiting {
                log::info!("suivi : {} n'est pas apparu après son lancement", p.title);
            }
            waiting
        });

        let after = running_ids(inner);
        if after != before {
            events.push(("running-changed", json!(after)));
        }
        if sessions_changed {
            events.push(("library-updated", json!(null)));
            // Méta-succès du launcher (S2-13) : notification à l'obtention.
            match crate::dashboard::new_badges(&conn) {
                Ok(fresh) => events.extend(fresh.into_iter().map(|b| ("badge-earned", json!({ "name": b.name, "description": b.description, "icon": b.icon })))),
                Err(e) => log::warn!("badges : {e:#}"),
            }
        }
        events
    }

    /// À l'arrêt de l'application : dernier battement pour les sessions en cours (elles seront reprises ou
    /// closes à ce battement au prochain démarrage).
    pub fn flush(&self, state: &AppState) {
        let inner = self.lock();
        let conn = state.conn();
        for l in &inner.live {
            let _ = conn.execute("UPDATE play_sessions SET ended_at = ?2, duration_seconds = ?3 WHERE id = ?1", params![l.session_id, iso(l.last_seen.max(l.started)), (l.last_seen - l.started).max(0)]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drives::DriveSnapshot;
    use crate::providers::local::detected_from_exe;

    const T0: i64 = 1_800_000_000;

    fn proc(pid: u32, path: &str) -> ProcInfo {
        let norm = norm_path(path);
        ProcInfo { pid, parent: 1, name: norm.rsplit('\\').next().unwrap().to_string(), path: Some(norm), started: None }
    }

    fn protected(pid: u32, name: &str) -> ProcInfo {
        ProcInfo { pid, parent: 1, name: name.to_lowercase(), path: None, started: None }
    }

    /// Bibliothèque de test : un jeu « Foo » (source locale) installé dans D:\Jeux\Foo.
    fn setup() -> (Arc<AppState>, Arc<Tracker>, String) {
        let state = Arc::new(AppState::new(crate::db::open_in_memory().unwrap(), std::env::temp_dir()));
        let detected = detected_from_exe("D:\\Jeux\\Foo\\Foo.exe", Some("Foo"), Some("D:\\Jeux\\Foo"), None, None, &[]);
        crate::library::upsert_detected(&state.conn(), &detected, &DriveSnapshot::default()).unwrap();
        let source_id = crate::db::all_sources(&state.conn()).unwrap().remove(0).id;
        let tracker = state.tracker.clone();
        tracker.refresh_index(&state, T0, true);
        (state, tracker, source_id)
    }

    fn sessions(state: &AppState) -> Vec<(String, i64, bool, String)> {
        let conn = state.conn();
        let mut stmt = conn.prepare("SELECT started_at, duration_seconds, active, origin FROM play_sessions ORDER BY started_at").unwrap();
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap();
        rows.map(|r| r.unwrap()).collect()
    }

    /// Fait tourner l'observation toutes les 3 s de `from` à `to` inclus avec les mêmes processus.
    fn run(tracker: &Tracker, state: &AppState, procs_list: &[ProcInfo], from: i64, to: i64) -> Vec<&'static str> {
        let mut events = Vec::new();
        let mut now = from;
        while now <= to {
            events.extend(tracker.tick(state, procs_list, now).into_iter().map(|(e, _)| e));
            now += 3;
        }
        events
    }

    #[test]
    fn index_matches_main_exe_folder_and_ignores_case() {
        let entry = IndexEntry { source_id: "s".into(), game_id: "g".into(), title: "Foo".into(), dir: Some("d:\\jeux\\foo".into()), exe: Some("d:\\jeux\\foo\\foolauncher.exe".into()), names: vec!["foolauncher.exe".into()] };
        let index = GameIndex::new(vec![entry], vec![]);
        assert_eq!(index.match_proc(&proc(1, "D:\\JEUX\\Foo\\Bin\\Foo-Win64-Shipping.exe")), Some(0), "casse différente, sous-dossier");
        assert_eq!(index.match_proc(&proc(1, "D:/Jeux/Foo/bin/game.exe")), Some(0), "séparateurs différents");
        assert_eq!(index.match_proc(&proc(1, "\\\\?\\D:\\Jeux\\Foo\\game.exe")), Some(0), "préfixe \\\\?\\");
        assert_eq!(index.match_proc(&proc(1, "D:\\Jeux\\Foo\\FooLauncher.exe")), Some(0), "l'exe principal compte même s'il s'appelle « launcher »");
        assert_eq!(index.match_proc(&proc(1, "D:\\Jeux\\Foo\\UnityCrashHandler64.exe")), None, "processus annexe exclu");
        assert_eq!(index.match_proc(&proc(1, "D:\\Jeux\\Foo\\OtherLauncher.exe")), None, "autre launcher exclu");
        assert_eq!(index.match_proc(&proc(1, "D:\\Jeux\\Foobar\\game.exe")), None, "dossier voisin");
        assert_eq!(index.match_proc(&proc(1, "C:\\Windows\\explorer.exe")), None);
        assert_eq!(index.match_proc(&protected(1, "FooLauncher.exe")), Some(0), "chemin illisible : comparaison par nom");
        assert_eq!(index.match_proc(&protected(1, "game.exe")), None);
    }

    #[test]
    fn nested_folders_pick_the_most_specific_game() {
        let e = |id: &str, dir: &str| IndexEntry { source_id: id.into(), game_id: id.into(), title: id.into(), dir: Some(dir.into()), exe: None, names: vec![] };
        let index = GameIndex::new(vec![e("lol", "d:\\riot\\league of legends"), e("pbe", "d:\\riot\\league of legends (pbe)"), e("root", "d:\\riot")], vec![]);
        assert_eq!(index.match_proc(&proc(1, "D:\\Riot\\League of Legends\\Game\\League of Legends.exe")), Some(0));
        assert_eq!(index.match_proc(&proc(1, "D:\\Riot\\League of Legends (PBE)\\Game\\League of Legends.exe")), Some(1));
        assert_eq!(index.match_proc(&proc(1, "D:\\Riot\\League of Legends\\LeagueClient.exe")), None, "le client n'est pas le jeu");
        assert_eq!(index.match_proc(&proc(1, "D:\\Riot\\League of Legends\\LeagueClientUx.exe")), None);
    }

    #[test]
    fn known_main_executables_cover_protected_games() {
        assert_eq!(known_exe_names(SourceId::Riot, "valorant.live"), ["valorant-win64-shipping.exe"]);
        assert_eq!(known_exe_names(SourceId::Riot, "league_of_legends.live"), ["league of legends.exe"]);
        assert!(known_exe_names(SourceId::Battlenet, "wowf").contains(&"wow.exe"));
        assert!(known_exe_names(SourceId::Steam, "440").is_empty());
    }

    #[test]
    fn game_started_outside_the_app_is_detected_and_closed() {
        let (state, tracker, _) = setup();
        let game = [proc(10, "d:\\JEUX\\foo\\FOO.EXE")];
        assert!(tracker.tick(&state, &[], T0).is_empty());
        let events = tracker.tick(&state, &game, T0 + 3);
        assert!(events.iter().any(|(e, _)| *e == "running-changed"), "détecté dès la première observation");
        assert_eq!(tracker.running_games().len(), 1);
        assert_eq!(sessions(&state), vec![(iso(T0 + 3), 0, true, "detected".to_string())]);

        run(&tracker, &state, &game, T0 + 6, T0 + 600);
        assert_eq!(sessions(&state)[0].1, 540, "battement enregistré toutes les 60 s");
        tracker.tick(&state, &game, T0 + 603);
        // Le jeu se ferme : délai de grâce de 10 s, puis clôture à la dernière observation.
        tracker.tick(&state, &[], T0 + 606);
        assert_eq!(tracker.running_games().len(), 1, "délai de grâce");
        let events = run(&tracker, &state, &[], T0 + 609, T0 + 618);
        assert!(events.contains(&"running-changed") && events.contains(&"library-updated"));
        assert!(tracker.running_games().is_empty());
        assert_eq!(sessions(&state), vec![(iso(T0 + 3), 600, false, "detected".to_string())]);
    }

    #[test]
    fn launch_from_the_app_creates_a_single_session() {
        let (state, tracker, source_id) = setup();
        let source = crate::db::get_source(&state.conn(), &source_id).unwrap().unwrap();
        let emit: Emitter = Arc::new(|_, _| {});
        tracker.start(&state, &emit, &source, false);
        assert_eq!(tracker.running_games().len(), 1, "« En cours » dès le clic");
        assert_eq!(tracker.running_info()[0].started_at, None);
        assert!(sessions(&state).is_empty(), "pas de session tant que le jeu n'est pas apparu");

        let now = Utc::now().timestamp();
        let game = [proc(10, "D:\\Jeux\\Foo\\Foo.exe"), proc(11, "D:\\Jeux\\Foo\\bin\\FooRenderer.exe")];
        run(&tracker, &state, &game, now + 3, now + 120);
        // Un second clic sur Jouer pendant la partie ne crée rien.
        tracker.start(&state, &emit, &source, false);
        run(&tracker, &state, &game, now + 123, now + 126);
        run(&tracker, &state, &[], now + 129, now + 150);
        let all = sessions(&state);
        assert_eq!(all.len(), 1, "une seule session pour plusieurs processus et deux clics");
        assert_eq!((all[0].1, all[0].2, all[0].3.as_str()), (123, false, "app"));
    }

    #[test]
    fn process_seen_just_before_the_launch_is_registered_still_counts_as_app() {
        let (state, tracker, source_id) = setup();
        let source = crate::db::get_source(&state.conn(), &source_id).unwrap().unwrap();
        let emit: Emitter = Arc::new(|_, _| {});
        let now = Utc::now().timestamp();
        let game = [proc(10, "D:\\Jeux\\Foo\\Foo.exe")];
        // L'observation tombe entre la création du processus et l'enregistrement du lancement.
        tracker.tick(&state, &game, now);
        assert_eq!(sessions(&state)[0].3, "detected");
        tracker.start(&state, &emit, &source, false);
        assert_eq!(sessions(&state)[0].3, "app");
        assert_eq!(sessions(&state).len(), 1);

        // En revanche, cliquer sur Jouer pendant une partie détectée depuis longtemps ne change pas son origine.
        let (state, tracker, source_id) = setup();
        let source = crate::db::get_source(&state.conn(), &source_id).unwrap().unwrap();
        run(&tracker, &state, &game, now - 600, now);
        tracker.start(&state, &emit, &source, false);
        assert_eq!(sessions(&state)[0].3, "detected");
    }

    #[test]
    fn launched_game_that_never_appears_is_dropped() {
        let (state, tracker, source_id) = setup();
        let source = crate::db::get_source(&state.conn(), &source_id).unwrap().unwrap();
        let emit: Emitter = Arc::new(|_, _| {});
        tracker.start(&state, &emit, &source, false);
        let now = Utc::now().timestamp();
        run(&tracker, &state, &[], now, now + START_GRACE_DIRECT_SECS - 3);
        assert_eq!(tracker.running_games().len(), 1);
        run(&tracker, &state, &[], now + START_GRACE_DIRECT_SECS, now + START_GRACE_DIRECT_SECS + 6);
        assert!(tracker.running_games().is_empty());
        assert!(sessions(&state).is_empty());
    }

    #[test]
    fn short_sessions_are_ignored_and_threshold_is_configurable() {
        let (state, tracker, _) = setup();
        let game = [proc(10, "D:\\Jeux\\Foo\\Foo.exe")];
        run(&tracker, &state, &game, T0, T0 + 30);
        run(&tracker, &state, &[], T0 + 33, T0 + 48);
        assert!(sessions(&state).is_empty(), "30 s : lancement raté, session ignorée");

        crate::db::set_setting(&state.conn(), "min_session_secs", "10").unwrap();
        tracker.refresh_index(&state, T0 + 100, true);
        run(&tracker, &state, &game, T0 + 100, T0 + 130);
        run(&tracker, &state, &[], T0 + 133, T0 + 148);
        assert_eq!(sessions(&state).len(), 1, "seuil abaissé à 10 s : la session de 30 s est conservée");
    }

    #[test]
    fn annex_process_does_not_keep_the_session_open() {
        // Cause du bogue « En cours » (E1-05) : un processus annexe survit au jeu dans son dossier.
        let (state, tracker, _) = setup();
        let handler = proc(11, "D:\\Jeux\\Foo\\UnityCrashHandler64.exe");
        let helper = proc(12, "D:\\Jeux\\Foo\\Engine\\EpicWebHelper.exe");
        let playing = [proc(10, "D:\\Jeux\\Foo\\Foo.exe"), handler.clone(), helper.clone()];
        let after = [handler, helper];
        run(&tracker, &state, &playing, T0, T0 + 300);
        run(&tracker, &state, &after, T0 + 303, T0 + 330);
        assert!(tracker.running_games().is_empty(), "les processus annexes ne maintiennent pas la session");
        assert_eq!(sessions(&state), vec![(iso(T0), 300, false, "detected".to_string())]);
        // Et ils n'en ouvrent pas non plus.
        run(&tracker, &state, &after, T0 + 333, T0 + 400);
        assert_eq!(sessions(&state).len(), 1);
    }

    #[test]
    fn protected_process_is_matched_by_name() {
        let (state, tracker, _) = setup();
        run(&tracker, &state, &[protected(10, "FOO.exe")], T0, T0 + 90);
        assert_eq!(tracker.running_games().len(), 1);
        run(&tracker, &state, &[], T0 + 93, T0 + 110);
        assert_eq!(sessions(&state)[0].1, 90);
    }

    #[test]
    fn game_already_running_at_startup_starts_at_process_creation() {
        let (state, tracker, _) = setup();
        let mut game = proc(10, "D:\\Jeux\\Foo\\Foo.exe");
        game.started = Some(T0 - 1200);
        tracker.tick(&state, &[game], T0);
        assert_eq!(sessions(&state), vec![(iso(T0 - 1200), 1200, true, "detected".to_string())]);
    }

    #[test]
    fn sessions_left_open_are_resumed_or_closed_at_their_last_heartbeat() {
        let (state, tracker, source_id) = setup();
        // Deux sessions restées « en cours » : l'une d'un jeu encore ouvert (battement récent), l'autre non.
        let other = detected_from_exe("D:\\Jeux\\Bar\\Bar.exe", Some("Bar"), Some("D:\\Jeux\\Bar"), None, None, &[]);
        crate::library::upsert_detected(&state.conn(), &other, &DriveSnapshot::default()).unwrap();
        let bar_id: String = state.conn().query_row("SELECT id FROM game_sources WHERE id != ?1", params![source_id], |r| r.get(0)).unwrap();
        state
            .conn()
            .execute(
                "INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds, active, origin) VALUES
                   ('foo', ?1, ?3, ?4, 540, 1, 'app'), ('bar', ?2, ?5, ?6, 420, 1, 'detected')",
                params![source_id, bar_id, iso(T0 - 600), iso(T0 - 60), iso(T0 - 5000), iso(T0 - 4580)],
            )
            .unwrap();
        tracker.refresh_index(&state, T0, true);
        tracker.tick(&state, &[proc(10, "D:\\Jeux\\Foo\\Foo.exe")], T0);
        assert_eq!(tracker.running_games().len(), 1, "Foo tourne toujours : sa session reprend");
        run(&tracker, &state, &[], T0 + 3, T0 + 20);
        assert_eq!(
            sessions(&state),
            vec![(iso(T0 - 5000), 420, false, "detected".to_string()), (iso(T0 - 600), 600, false, "app".to_string())],
            "Bar : close à son dernier battement ; Foo : une seule session continue"
        );
    }

    #[test]
    fn stale_open_session_is_not_resumed_even_if_the_game_runs() {
        let (state, tracker, source_id) = setup();
        state
            .conn()
            .execute(
                "INSERT INTO play_sessions(id, game_source_id, started_at, ended_at, duration_seconds, active, origin) VALUES('old', ?1, ?2, ?3, 3600, 1, 'app')",
                params![source_id, iso(T0 - 90_000), iso(T0 - 86_400)],
            )
            .unwrap();
        let mut game = proc(10, "D:\\Jeux\\Foo\\Foo.exe");
        game.started = Some(T0 - 100);
        tracker.tick(&state, &[game], T0);
        assert_eq!(
            sessions(&state),
            vec![(iso(T0 - 90_000), 3600, false, "app".to_string()), (iso(T0 - 100), 100, true, "detected".to_string())],
            "l'ancienne session s'arrête à son battement ; la partie en cours en ouvre une nouvelle"
        );
    }

    #[test]
    fn sleep_longer_than_five_minutes_splits_the_session() {
        let (state, tracker, _) = setup();
        let mut game = proc(10, "D:\\Jeux\\Foo\\Foo.exe");
        game.started = Some(T0);
        let game = [game];
        run(&tracker, &state, &game, T0, T0 + 600);
        // Mise en veille d'une heure, jeu toujours ouvert au réveil.
        let wake = T0 + 600 + 3600;
        run(&tracker, &state, &game, wake, wake + 120);
        run(&tracker, &state, &[], wake + 123, wake + 140);
        assert_eq!(
            sessions(&state),
            vec![(iso(T0), 600, false, "detected".to_string()), (iso(wake), 120, false, "detected".to_string())],
            "le temps de veille n'est pas compté"
        );
    }

    #[test]
    fn stop_tracking_closes_now_and_ignores_the_game_until_it_exits() {
        let (state, tracker, source_id) = setup();
        let game = [proc(10, "D:\\Jeux\\Foo\\Foo.exe")];
        let source = crate::db::get_source(&state.conn(), &source_id).unwrap().unwrap();
        let emit: Emitter = Arc::new(|_, _| {});
        let now = Utc::now().timestamp();
        run(&tracker, &state, &game, now - 300, now - 3);
        assert!(tracker.stop_tracking(&state, &emit, &source.game_id));
        assert!(tracker.running_games().is_empty());
        let all = sessions(&state);
        assert_eq!(all.len(), 1);
        assert!(!all[0].2 && (298..=305).contains(&all[0].1), "close maintenant : {} s", all[0].1);
        // Le processus bloqué tourne toujours : pas de nouvelle session.
        run(&tracker, &state, &game, now + 3, now + 60);
        assert_eq!(sessions(&state).len(), 1);
        assert!(tracker.running_games().is_empty());
        // Il finit par disparaître ; au prochain lancement le jeu est de nouveau suivi.
        run(&tracker, &state, &[], now + 63, now + 66);
        run(&tracker, &state, &game, now + 69, now + 72);
        assert_eq!(tracker.running_games().len(), 1);
    }

    #[test]
    fn external_tracking_can_be_disabled() {
        let (state, tracker, source_id) = setup();
        crate::db::set_setting(&state.conn(), "track_external", "0").unwrap();
        tracker.refresh_index(&state, T0, true);
        let game = [proc(10, "D:\\Jeux\\Foo\\Foo.exe")];
        let now = Utc::now().timestamp();
        run(&tracker, &state, &game, now - 200, now - 100);
        assert!(tracker.running_games().is_empty() && sessions(&state).is_empty(), "jeu lancé dehors : ignoré");
        // Lancé depuis l'application : toujours suivi.
        let source = crate::db::get_source(&state.conn(), &source_id).unwrap().unwrap();
        tracker.start(&state, &(Arc::new(|_: &str, _: serde_json::Value| {}) as Emitter), &source, false);
        run(&tracker, &state, &game, now + 3, now + 90);
        assert_eq!(sessions(&state).len(), 1);
        assert_eq!(sessions(&state)[0].3, "app");
    }

    #[test]
    fn games_marked_untracked_or_from_disabled_sources_are_not_indexed() {
        let (state, tracker, _) = setup();
        state.conn().execute("UPDATE games SET no_tracking = 1", []).unwrap();
        tracker.refresh_index(&state, T0, true);
        run(&tracker, &state, &[proc(10, "D:\\Jeux\\Foo\\Foo.exe")], T0, T0 + 30);
        assert!(tracker.running_games().is_empty());
        state.conn().execute("UPDATE games SET no_tracking = 0", []).unwrap();
        let disabled: HashSet<String> = ["local".to_string()].into_iter().collect();
        assert!(GameIndex::from_db(&state.conn(), &disabled).unwrap().entries.is_empty());
        assert_eq!(GameIndex::from_db(&state.conn(), &HashSet::new()).unwrap().entries.len(), 1);
    }

    #[test]
    fn custom_folder_and_executable_identify_the_game() {
        // E2-06 : les valeurs personnalisées remplacent les valeurs détectées dans l'index.
        let (state, tracker, source_id) = setup();
        state
            .conn()
            .execute("UPDATE game_sources SET install_path_override = 'E:\\Ailleurs\\Foo', executable_path_override = 'E:\\Ailleurs\\Foo\\bin\\RealFoo.exe' WHERE id = ?1", params![source_id])
            .unwrap();
        tracker.refresh_index(&state, T0, true);
        run(&tracker, &state, &[proc(10, "D:\\Jeux\\Foo\\Foo.exe")], T0, T0 + 9);
        assert!(tracker.running_games().is_empty(), "l'ancien dossier détecté n'identifie plus le jeu");
        run(&tracker, &state, &[proc(11, "e:\\ailleurs\\foo\\bin\\realfoo.exe")], T0 + 12, T0 + 15);
        assert_eq!(tracker.running_games().len(), 1);
    }

    #[test]
    fn kill_targets_are_the_game_processes_and_their_children() {
        let entry = IndexEntry { source_id: "s".into(), game_id: "g".into(), title: "Foo".into(), dir: Some("d:\\jeux\\foo".into()), exe: None, names: vec![] };
        let index = GameIndex::new(vec![entry], vec![]);
        let mut child = protected(12, "fooservice.exe");
        child.parent = 10;
        let mut browser = proc(13, "C:\\Program Files\\Navigateur\\navigateur.exe");
        browser.parent = 10;
        let procs_list = vec![proc(10, "D:\\Jeux\\Foo\\Foo.exe"), proc(11, "D:\\Jeux\\Foo\\UnityCrashHandler64.exe"), child, browser, proc(14, "C:\\Windows\\explorer.exe")];
        let mut pids: Vec<u32> = index.kill_targets(&procs_list, "g").iter().map(|p| p.pid).collect();
        pids.sort();
        assert_eq!(pids, vec![10, 11, 12], "le jeu, son processus annexe et son enfant ; pas le navigateur ouvert par le jeu");
        assert!(index.kill_targets(&procs_list, "autre").is_empty());
    }

    /// Bibliothèque avec un faux jeu réel (copies d'exécutables Windows) pour les tests de bout en bout.
    fn real_game(tmp: &std::path::Path) -> (Arc<AppState>, Arc<Tracker>, GameSource, std::path::PathBuf) {
        let dir = tmp.join("Faux Jeu");
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("fauxjeu.exe");
        std::fs::copy("C:\\Windows\\System32\\PING.EXE", &exe).unwrap();
        let state = Arc::new(AppState::new(crate::db::open_in_memory().unwrap(), tmp.to_path_buf()));
        crate::db::set_setting(&state.conn(), "min_session_secs", "5").unwrap();
        let detected = detected_from_exe(&exe.to_string_lossy(), None, None, Some("-n 13 127.0.0.1"), None, &[]);
        crate::library::upsert_detected(&state.conn(), &detected, &DriveSnapshot::current()).unwrap();
        let source = crate::db::all_sources(&state.conn()).unwrap().remove(0);
        let tracker = state.tracker.clone();
        {
            let (tracker, state) = (tracker.clone(), state.clone());
            std::thread::spawn(move || tracker.run_loop(state, Arc::new(|_, _| {})));
        }
        (state, tracker, source, dir)
    }

    fn wait_until(what: &str, secs: u64, cond: impl Fn() -> bool) -> Duration {
        let begin = std::time::Instant::now();
        while !cond() {
            assert!(begin.elapsed() < Duration::from_secs(secs), "délai dépassé : {what}");
            std::thread::sleep(Duration::from_millis(250));
        }
        begin.elapsed()
    }

    /// Bout en bout avec de vrais processus (~30 s) : `cargo test -- --ignored`.
    /// Le faux jeu est lancé HORS de l'application, avec un processus annexe qui lui survit.
    #[test]
    #[ignore]
    fn e2e_detects_external_launch_and_ignores_surviving_annex_process() {
        let tmp = tempfile::tempdir().unwrap();
        let (state, tracker, _source, dir) = real_game(tmp.path());
        let handler = dir.join("UnityCrashHandler64.exe");
        std::fs::copy("C:\\Windows\\System32\\PING.EXE", &handler).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        assert!(tracker.running_games().is_empty());

        // Lancement en dehors de l'application, chemin en majuscules.
        let exe_upper = dir.join("FAUXJEU.EXE");
        let mut annex = std::process::Command::new(&handler).args(["-n", "60", "127.0.0.1"]).stdout(std::process::Stdio::null()).spawn().unwrap();
        let mut game = std::process::Command::new(&exe_upper).args(["-n", "13", "127.0.0.1"]).stdout(std::process::Stdio::null()).spawn().unwrap();
        let delay = wait_until("détection du jeu", 10, || !tracker.running_games().is_empty());
        assert!(delay < Duration::from_secs(10), "détecté en {delay:?}");
        let origin: String = state.conn().query_row("SELECT origin FROM play_sessions WHERE active = 1", [], |r| r.get(0)).unwrap();
        assert_eq!(origin, "detected");

        // Le jeu se termine (~12 s) ; le processus annexe tourne encore mais la session se clôt.
        game.wait().unwrap();
        wait_until("clôture de la session", 20, || tracker.running_games().is_empty());
        assert!(annex.try_wait().unwrap().is_none(), "le processus annexe tourne toujours");
        let (duration, active): (i64, bool) = state.conn().query_row("SELECT duration_seconds, active FROM play_sessions", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert!(!active);
        assert!((9..=16).contains(&duration), "durée suivie : {duration} s pour ~12 s de jeu");
        annex.kill().unwrap();
    }

    /// Lancement depuis l'application : une seule session, d'origine « app » (~25 s).
    #[test]
    #[ignore]
    fn e2e_launch_from_app_creates_one_session() {
        let tmp = tempfile::tempdir().unwrap();
        let (state, tracker, source, _dir) = real_game(tmp.path());
        crate::launch::launch_source(&source).unwrap();
        tracker.start(&state, &(Arc::new(|_: &str, _: serde_json::Value| {}) as Emitter), &source, false);
        assert_eq!(tracker.running_games().len(), 1, "indicateur « En cours » dès le lancement");
        wait_until("clôture de la session", 40, || tracker.running_games().is_empty());
        let rows = sessions(&state);
        assert_eq!(rows.len(), 1, "une seule session");
        assert_eq!(rows[0].3, "app");
        assert!((9..=16).contains(&rows[0].1), "durée suivie : {} s pour ~12 s de jeu", rows[0].1);
        let game = &crate::library::list_games(&state.conn()).unwrap()[0];
        assert_eq!(game.tracked_seconds, rows[0].1);
    }

    /// « Forcer la fermeture » termine l'arbre de processus du jeu (~10 s).
    #[test]
    #[ignore]
    fn e2e_force_close_kills_the_process_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let (state, tracker, source, dir) = real_game(tmp.path());
        // Le « jeu » (copie de cmd.exe) lance un enfant persistant situé dans son dossier.
        let parent = dir.join("fauxjeu.exe");
        std::fs::copy("C:\\Windows\\System32\\cmd.exe", &parent).unwrap();
        // Une copie de cmd.exe ne démarre qu'avec ses ressources de langue à côté d'elle.
        for lang in std::fs::read_dir("C:\\Windows\\System32").unwrap().flatten() {
            let mui = lang.path().join("cmd.exe.mui");
            if mui.is_file() {
                let target = dir.join(lang.file_name());
                std::fs::create_dir_all(&target).unwrap();
                std::fs::copy(&mui, target.join("fauxjeu.exe.mui")).unwrap();
            }
        }
        let child = dir.join("fauxmoteur.exe");
        std::fs::copy("C:\\Windows\\System32\\PING.EXE", &child).unwrap();
        let mut game = std::process::Command::new(&parent)
            .arg("/c")
            .arg(&child)
            .args(["-n", "120", "127.0.0.1"])
            .current_dir(&dir)
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        wait_until("détection du jeu", 10, || !tracker.running_games().is_empty());
        let alive = |name: &str| ProcessLister::new().snapshot().iter().any(|p| p.name == name);
        assert!(alive("fauxmoteur.exe"), "l'enfant tourne");

        let report = tracker.force_close(&state, &(Arc::new(|_: &str, _: serde_json::Value| {}) as Emitter), &source.game_id).unwrap();
        assert!(report.killed >= 2, "parent et enfant terminés : {report:?}");
        game.wait().unwrap();
        wait_until("fin de l'enfant", 5, || !alive("fauxmoteur.exe"));
        assert!(tracker.running_games().is_empty(), "session close");
        let active: i64 = state.conn().query_row("SELECT COUNT(*) FROM play_sessions WHERE active = 1", [], |r| r.get(0)).unwrap();
        assert_eq!(active, 0);
    }
}
