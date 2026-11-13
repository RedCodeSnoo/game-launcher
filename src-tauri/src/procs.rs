//! Processus Windows : liste légère (E4-03), motifs d'exclusion (E1-07) et fermeture forcée (E1-02).
//!
//! La liste vient de `CreateToolhelp32Snapshot` (un seul appel système, sans droits particuliers).
//! Le chemin complet et l'heure de création ne sont demandés qu'une fois par processus, à son apparition
//! (`QueryFullProcessImageNameW` avec le droit minimal `PROCESS_QUERY_LIMITED_INFORMATION`).
use crate::drives::norm_path;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct ProcInfo {
    pub pid: u32,
    pub parent: u32,
    /// Nom du fichier exécutable, en minuscules (toujours connu).
    pub name: String,
    /// Chemin complet normalisé ; `None` si le processus est protégé (administrateur, anti-triche).
    pub path: Option<String>,
    /// Heure de création (secondes Unix), si elle est lisible.
    pub started: Option<i64>,
}

struct Cached {
    name: String,
    path: Option<String>,
    started: Option<i64>,
    attempts: u8,
}

/// Liste les processus en ne réinterrogeant que les nouveaux.
#[derive(Default)]
pub struct ProcessLister {
    cache: HashMap<u32, Cached>,
}

/// Nombre d'essais de lecture du chemin d'un processus (il peut être illisible juste à sa création).
const MAX_ATTEMPTS: u8 = 3;

impl ProcessLister {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&mut self) -> Vec<ProcInfo> {
        let raw = raw_snapshot();
        let alive: HashSet<u32> = raw.iter().map(|r| r.0).collect();
        self.cache.retain(|pid, _| alive.contains(pid));
        let mut out = Vec::with_capacity(raw.len());
        for (pid, parent, name) in raw {
            let entry = self.cache.entry(pid).or_insert_with(|| Cached { name: String::new(), path: None, started: None, attempts: 0 });
            if entry.name != name {
                // Nouveau processus (ou identifiant réutilisé par un autre programme).
                *entry = Cached { name: name.clone(), path: None, started: None, attempts: 0 };
            }
            if entry.path.is_none() && entry.attempts < MAX_ATTEMPTS {
                entry.attempts += 1;
                let (path, started) = query_process(pid);
                entry.path = path.map(|p| norm_path(&p));
                entry.started = started.or(entry.started);
            }
            out.push(ProcInfo { pid, parent, name, path: entry.path.clone(), started: entry.started });
        }
        out
    }
}

/// (pid, pid du parent, nom en minuscules) de chaque processus vivant.
fn raw_snapshot() -> Vec<(u32, u32, String)> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
    let mut out = Vec::with_capacity(256);
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return out;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut more = Process32FirstW(snap, &mut entry) != 0;
        while more {
            // Processus système (0, 4) et processus terminés dont il ne reste qu'une coquille (aucun fil).
            if entry.th32ProcessID > 4 && entry.cntThreads > 0 {
                let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase();
                out.push((entry.th32ProcessID, entry.th32ParentProcessID, name));
            }
            more = Process32NextW(snap, &mut entry) != 0;
        }
        CloseHandle(snap);
    }
    out
}

/// Chemin complet et heure de création d'un processus, sans droits administrateur.
fn query_process(pid: u32) -> (Option<String>, Option<i64>) {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME};
    use windows_sys::Win32::System::Threading::{GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return (None, None);
        }
        let mut buf = vec![0u16; 1024];
        let mut size = buf.len() as u32;
        let path = (QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) != 0).then(|| String::from_utf16_lossy(&buf[..size as usize]));
        let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
        let started = (GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) != 0)
            .then(|| filetime_to_unix(((created.dwHighDateTime as u64) << 32) | created.dwLowDateTime as u64))
            .filter(|t| *t > 0);
        CloseHandle(handle);
        (path, started)
    }
}

/// FILETIME (pas de 100 ns depuis 1601) vers secondes Unix.
pub fn filetime_to_unix(ticks: u64) -> i64 {
    (ticks / 10_000_000) as i64 - 11_644_473_600
}

// ---------------------------------------------------------------------------
// Exclusions (E1-07)
// ---------------------------------------------------------------------------

/// Processus annexes qui ne doivent ni ouvrir ni maintenir une session : gestionnaires de plantage,
/// anti-triche, clients et launchers, vues web embarquées, installeurs. Motifs avec `*`, sans casse.
pub const DEFAULT_EXCLUSIONS: &[&str] = &[
    "*crashhandler*.exe",
    "*crashreport*.exe",
    "*crashpad*.exe",
    "*crash_reporter*.exe",
    "*errorreport*.exe",
    "unitycrashhandler*.exe",
    "blizzarderror.exe",
    "werfault.exe",
    "easyanticheat*.exe",
    "*anticheat*.exe",
    "beservice*.exe",
    "battleye*.exe",
    "vgc.exe",
    "vgtray.exe",
    "riotclient*.exe",
    "riot client.exe",
    "leagueclient*.exe",
    "steam.exe",
    "steamwebhelper.exe",
    "steamservice.exe",
    "gameoverlayui*.exe",
    "epicgameslauncher.exe",
    "epicwebhelper.exe",
    "*webhelper*.exe",
    "battle.net*.exe",
    "agent.exe",
    "blizzardbrowser.exe",
    "wowvoiceproxy.exe",
    "*launcher*.exe",
    "upc.exe",
    "ubisoftconnect*.exe",
    "uplaywebcore.exe",
    "eadesktop.exe",
    "eabackgroundservice.exe",
    "ealocalhostsvc.exe",
    "galaxyclient*.exe",
    "*cefsubprocess*.exe",
    "*cefprocess*.exe",
    "cefsharp.browsersubprocess.exe",
    "qtwebengineprocess.exe",
    "*bootstrapper*.exe",
    "*updater*.exe",
    "*patcher*.exe",
    "*installer*.exe",
    "*setup*.exe",
    "unins*.exe",
    "*redist*.exe",
    "dxsetup.exe",
    "dotnetfx*.exe",
    "checkenv.exe",
    "notification_helper.exe",
    // Wallpaper Engine : son moteur tourne en permanence, ce n'est pas une partie en cours.
    "wallpaper32.exe",
    "wallpaper64.exe",
    "wallpaperservice32*.exe",
];

/// Correspondance d'un motif à jokers `*` (déjà en minuscules) avec un nom en minuscules.
pub fn wildcard_match(pattern: &str, name: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == name;
    }
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if !name.starts_with(first) || name.len() < first.len() + last.len() || !name[first.len()..].ends_with(last) {
        return false;
    }
    let mut rest = &name[first.len()..name.len() - last.len()];
    for part in &parts[1..parts.len() - 1] {
        match rest.find(part) {
            Some(i) => rest = &rest[i + part.len()..],
            None => return false,
        }
    }
    true
}

/// `name` : nom de fichier en minuscules. `extra` : motifs ajoutés par l'utilisateur (en minuscules).
pub fn is_excluded(name: &str, extra: &[String]) -> bool {
    DEFAULT_EXCLUSIONS.iter().any(|p| wildcard_match(p, name)) || extra.iter().any(|p| wildcard_match(p, name))
}

// ---------------------------------------------------------------------------
// Fermeture forcée (E1-02, E1-03)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, PartialEq)]
pub struct KillReport {
    pub killed: usize,
    /// Noms des processus qui n'ont pas pu être terminés (droits insuffisants, anti-triche).
    pub failed: Vec<String>,
}

/// Termine les processus indiqués. Ne touche jamais au processus courant.
pub fn kill(targets: &[ProcInfo]) -> KillReport {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    let me = std::process::id();
    let mut report = KillReport::default();
    for p in targets.iter().filter(|p| p.pid != me) {
        let done = unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, p.pid);
            if handle.is_null() {
                false
            } else {
                let ok = TerminateProcess(handle, 1) != 0;
                CloseHandle(handle);
                ok
            }
        };
        if done {
            report.killed += 1;
        } else if !report.failed.contains(&p.name) {
            report.failed.push(p.name.clone());
        }
    }
    report
}

/// Descendants (enfants, petits-enfants…) des processus `roots` dans `procs`.
/// Un « enfant » créé avant son parent supposé est ignoré : c'est un identifiant de processus réutilisé.
pub fn descendants<'a>(procs: &'a [ProcInfo], roots: &[u32]) -> Vec<&'a ProcInfo> {
    let by_pid: HashMap<u32, &ProcInfo> = procs.iter().map(|p| (p.pid, p)).collect();
    let mut seen: HashSet<u32> = roots.iter().copied().collect();
    let mut queue: Vec<u32> = roots.to_vec();
    let mut out = Vec::new();
    while let Some(parent) = queue.pop() {
        for p in procs.iter().filter(|p| p.parent == parent && !seen.contains(&p.pid)) {
            let older_than_parent = match (p.started, by_pid.get(&parent).and_then(|pp| pp.started)) {
                (Some(child), Some(par)) => child < par,
                _ => false,
            };
            if older_than_parent {
                continue;
            }
            seen.insert(p.pid);
            queue.push(p.pid);
            out.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards() {
        assert!(wildcard_match("vgc.exe", "vgc.exe"));
        assert!(!wildcard_match("vgc.exe", "vgc2.exe"));
        assert!(wildcard_match("*crashhandler*.exe", "unitycrashhandler64.exe"));
        assert!(wildcard_match("riotclient*.exe", "riotclientservices.exe"));
        assert!(wildcard_match("*launcher*.exe", "world of warcraft launcher.exe"));
        assert!(!wildcard_match("*launcher*.exe", "launch.exe"));
        assert!(!wildcard_match("a*a", "a"));
        assert!(wildcard_match("unins*.exe", "unins000.exe"));
    }

    #[test]
    fn annex_processes_are_excluded_but_games_are_not() {
        for name in [
            "unitycrashhandler64.exe",
            "crashreportclient.exe",
            "crashpad_handler.exe",
            "easyanticheat_eos.exe",
            "beservice_x64.exe",
            "vgc.exe",
            "vgtray.exe",
            "riotclientservices.exe",
            "riotclientcrashhandler.exe",
            "leagueclient.exe",
            "leagueclientuxrender.exe",
            "leaguecrashhandler64.exe",
            "steamwebhelper.exe",
            "epicwebhelper.exe",
            "battle.net.exe",
            "battle.net helper.exe",
            "agent.exe",
            "world of warcraft launcher.exe",
            "blizzarderror.exe",
            "fortnitelauncher.exe",
            "wallpaper32.exe",
        ] {
            assert!(is_excluded(name, &[]), "{name} devrait être exclu");
        }
        for name in ["league of legends.exe", "valorant-win64-shipping.exe", "wow.exe", "hades.exe", "pbz-win64-shipping.exe", "terraria.exe", "fortniteclient-win64-shipping.exe"] {
            assert!(!is_excluded(name, &[]), "{name} est un jeu");
        }
        assert!(is_excluded("monoutil.exe", &["monoutil*.exe".to_string()]), "motif ajouté par l'utilisateur");
    }

    #[test]
    fn filetime_conversion() {
        // 1er janvier 2026 00:00:00 UTC.
        assert_eq!(filetime_to_unix(134_116_992_000_000_000), 1_767_225_600);
    }

    #[test]
    fn sees_current_process_with_path_and_start_time() {
        let mut lister = ProcessLister::new();
        let procs = lister.snapshot();
        let me = procs.iter().find(|p| p.pid == std::process::id()).expect("le processus de test doit être visible");
        assert_eq!(me.path.as_deref(), Some(norm_path(&std::env::current_exe().unwrap().to_string_lossy()).as_str()));
        let now = chrono::Utc::now().timestamp();
        let started = me.started.expect("heure de création lisible");
        assert!(started <= now && now - started < 3600, "heure de création plausible");
        // Deuxième passe : mêmes informations, servies par le cache.
        let again = lister.snapshot();
        assert_eq!(again.iter().find(|p| p.pid == me.pid), Some(me));
    }

    #[test]
    fn descendants_follow_the_tree_and_ignore_reused_pids() {
        let p = |pid, parent, started| ProcInfo { pid, parent, name: format!("p{pid}.exe"), path: None, started: Some(started) };
        let procs = vec![p(10, 1, 100), p(11, 10, 110), p(12, 11, 120), p(13, 10, 50), p(20, 2, 100)];
        let mut found: Vec<u32> = descendants(&procs, &[10]).iter().map(|p| p.pid).collect();
        found.sort();
        assert_eq!(found, vec![11, 12], "13 est plus ancien que son parent supposé : identifiant réutilisé");
    }
}
