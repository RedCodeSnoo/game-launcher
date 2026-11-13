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
    pub tracker: Arc<Tracker>,
}

pub type Shared = Arc<AppState>;

/// Canal vers l'interface : (nom de l'événement, charge utile JSON).
pub type Emitter = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;

impl AppState {
    pub fn new(conn: Connection, data_dir: PathBuf) -> Self {
        AppState { db: Mutex::new(conn), data_dir, scanning: AtomicBool::new(false), tracker: Tracker::new() }
    }

    /// Verrou sur la base ; un verrou empoisonné (panique d'un autre fil) reste utilisable.
    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        self.db.lock().unwrap_or_else(|e| e.into_inner())
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
