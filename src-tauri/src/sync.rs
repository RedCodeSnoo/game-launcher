//! Orchestration de la détection : chaque provider est isolé (§6.3), l'interface n'est jamais bloquée.
use crate::drives::DriveSnapshot;
use crate::library::{self, ScanStats};
use crate::models::SourceId;
use crate::providers;
use crate::state::{AppState, Emitter};
use serde::Serialize;
use serde_json::json;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::Ordering;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceReport {
    pub source: SourceId,
    pub available: bool,
    pub enabled: bool,
    pub error: Option<String>,
    pub stats: Option<ScanStats>,
}

/// Détection locale complète. Renvoie `None` si un scan est déjà en cours.
pub fn run_scan(state: &AppState, emit: &Emitter) -> Option<Vec<SourceReport>> {
    if state.scanning.swap(true, Ordering::SeqCst) {
        return None;
    }
    emit("scan-state", json!({ "running": true }));
    let started = std::time::Instant::now();
    let reports = scan_all(state);
    state.scanning.store(false, Ordering::SeqCst);
    log::info!("détection terminée en {} ms", started.elapsed().as_millis());
    emit("scan-state", json!({ "running": false, "reports": reports }));
    emit("library-updated", json!(null));
    Some(reports)
}

pub fn scan_all(state: &AppState) -> Vec<SourceReport> {
    let drives = DriveSnapshot::current();
    if let Err(e) = library::refresh_availability(&mut state.conn(), &drives) {
        log::error!("mise à jour de la disponibilité des disques : {e:#}");
    }
    let disabled = state.disabled_sources();
    let cfg = state.provider_config();
    let mut reports = Vec::new();
    for provider in providers::all(&cfg) {
        let source = provider.id();
        let mut report = SourceReport { source, available: false, enabled: !disabled.contains(source.as_str()), error: None, stats: None };
        if !report.enabled {
            reports.push(report);
            continue;
        }
        // Une panique ou une erreur dans un provider ne touche pas les autres.
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            if !provider.is_available() {
                return Ok(None);
            }
            provider.scan_installed().map(Some)
        }));
        match outcome {
            Ok(Ok(None)) => {}
            Ok(Ok(Some(games))) => {
                report.available = true;
                match library::apply_scan(&mut state.conn(), source, &games, &drives) {
                    Ok(stats) => {
                        log::info!("{} : {} jeu(x) installé(s), {} nouveau(x)", source.as_str(), stats.found, stats.added);
                        report.stats = Some(stats);
                    }
                    Err(e) => {
                        log::error!("{} : enregistrement impossible : {e:#}", source.as_str());
                        report.error = Some(format!("{e:#}"));
                    }
                }
            }
            Ok(Err(e)) => {
                report.available = true;
                log::error!("{} : détection en erreur : {e:#}", source.as_str());
                report.error = Some(format!("{e:#}"));
            }
            Err(_) => {
                report.available = true;
                log::error!("{} : le module a paniqué pendant la détection", source.as_str());
                report.error = Some("erreur interne du module".into());
            }
        }
        reports.push(report);
    }
    if let Err(e) = library::check_local_games(&mut state.conn(), &drives) {
        log::error!("vérification des jeux locaux : {e:#}");
    }
    reports
}
