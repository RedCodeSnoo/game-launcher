//! État partagé de l'application.
use crate::db;
use crate::providers::ProviderConfig;
use crate::tracker::Tracker;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard};

pub struct AppState {
    pub db: Mutex<Connection>,
    pub data_dir: PathBuf,
    pub scanning: AtomicBool,
    pub syncing: AtomicBool,
    pub tracker: Arc<Tracker>,
}

pub type Shared = Arc<AppState>;

thread_local! {
    static HOLDING_DB: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Accès à la base tant que le verrou est tenu.
pub struct ConnGuard<'a> {
    guard: MutexGuard<'a, Connection>,
}

impl Drop for ConnGuard<'_> {
    fn drop(&mut self) {
        HOLDING_DB.with(|h| h.set(false));
    }
}

impl std::ops::Deref for ConnGuard<'_> {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        &self.guard
    }
}

impl std::ops::DerefMut for ConnGuard<'_> {
    fn deref_mut(&mut self) -> &mut Connection {
        &mut self.guard
    }
}

/// Canal vers l'interface : (nom de l'événement, charge utile JSON).
pub type Emitter = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;

impl AppState {
    pub fn new(conn: Connection, data_dir: PathBuf) -> Self {
        AppState { db: Mutex::new(conn), data_dir, scanning: AtomicBool::new(false), syncing: AtomicBool::new(false), tracker: Tracker::new() }
    }

    /// Verrou sur la base ; un verrou empoisonné (panique d'un autre fil) reste utilisable.
    ///
    /// Reprendre le verrou sur un fil qui le tient déjà bloquerait ce fil pour toujours, et avec lui toute
    /// l'application (bibliothèque vide, images figées, fenêtre qui ne se ferme plus). Ce cas est détecté :
    /// l'opération échoue avec une erreur interne au lieu de tout figer, et les tests le signalent.
    pub fn conn(&self) -> ConnGuard<'_> {
        if HOLDING_DB.with(|h| h.get()) {
            log::error!("verrou de la base demandé deux fois sur le même fil");
            panic!("verrou de la base demandé deux fois sur le même fil");
        }
        let guard = self.db.lock().unwrap_or_else(|e| e.into_inner());
        HOLDING_DB.with(|h| h.set(true));
        ConnGuard { guard }
    }

    pub fn provider_config(&self) -> ProviderConfig {
        let conn = self.conn();
        ProviderConfig { steam_id: db::get_setting(&conn, "steam_id").filter(|s| !s.is_empty()) }
    }

    pub fn disabled_sources(&self) -> HashSet<String> {
        let conn = self.conn();
        db::get_setting(&conn, "disabled_sources")
            .and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok())
            .unwrap_or_default()
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        AppState::new(db::open_in_memory().unwrap(), std::env::temp_dir())
    }

    #[test]
    fn lock_released_after_each_use() {
        let st = state();
        // Le schéma d'avant le correctif : le verrou de l'argument est encore tenu quand on lit les réglages.
        let n: i64 = st.conn().query_row("SELECT COUNT(*) FROM games", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
        let _ = st.disabled_sources();
        let _ = st.provider_config();
        let _ = st.conn();
        let _ = st.conn();
    }

    #[test]
    fn double_lock_on_one_thread_fails_instead_of_freezing() {
        let st = std::sync::Arc::new(state());
        let s2 = st.clone();
        let r = std::thread::spawn(move || {
            let _held = s2.conn();
            let _again = s2.disabled_sources();
        })
        .join();
        assert!(r.is_err(), "le second verrou doit échouer");
        // Le verrou reste utilisable ensuite (y compris depuis d'autres fils).
        let _ = st.conn();
        let s3 = st.clone();
        std::thread::spawn(move || drop(s3.conn())).join().unwrap();
    }

    #[test]
    fn news_feed_does_not_lock_twice() {
        let st = state();
        st.conn()
            .execute_batch(
                "INSERT INTO games(id, title, sort_title, added_at) VALUES('a', 'Hades', 'hades', '2025-01-01T00:00:00Z');
                 INSERT INTO game_sources(id, game_id, source, external_id, installed, launch_method, launch_target) VALUES('sa', 'a', 'steam', '1145360', 1, 'uri', 'x');",
            )
            .unwrap();
        let games = crate::commands::news_feed_games(&st).unwrap();
        assert_eq!(games.len(), 1);
    }
}
