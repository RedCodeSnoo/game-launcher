//! Provider des jeux locaux, sans launcher (§4.9) : ajout manuel et scan de dossiers surveillés.
use super::GameProvider;
use crate::drives::{clean_path, norm_path};
use crate::library::normalize_title;
use crate::models::{Capabilities, DetectedGame, LaunchMethod, SourceId};
use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub struct LocalProvider;

impl GameProvider for LocalProvider {
    fn id(&self) -> SourceId {
        SourceId::Local
    }
    fn is_available(&self) -> bool {
        true
    }
    /// Les jeux locaux sont ajoutés par l'utilisateur ; il n'y a rien à détecter automatiquement.
    fn scan_installed(&self) -> Result<Vec<DetectedGame>> {
        Ok(Vec::new())
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities { installed: true, owned: false, playtime: false, achievements: false }
    }
}

// Faux positifs à écarter (F-LOC-03). Comparaison sur le nom sans extension, en minuscules.
const EXCLUDED_PREFIXES: &[&str] = &[
    "unins", "vc_redist", "vcredist", "setup", "install", "dxsetup", "dxwebsetup", "ue4prereqsetup", "ueprereqsetup", "dotnet", "ndp4", "directx", "oalinst",
    "physx", "crashreport", "crashpad", "python", "7z", "update", "createdump", "dxdiag", "ffmpeg", "ffprobe", "qtwebengine", "vconsole",
];
const EXCLUDED_CONTAINS: &[&str] = &[
    "crashhandler", "crashreporter", "crashsender", "crashdump", "uninstall", "redist", "prereq", "helper", "subprocess", "cefprocess", "easyanticheat", "battleye",
    "beservice", "bugreport", "errorreport", "notification", "updater", "installer", "dedicatedserver", "anticheat", "diagnostic", "sendrpt",
];

pub fn is_excluded_exe(file_name: &str) -> bool {
    let lower = file_name.to_lowercase();
    let stem = lower.strip_suffix(".exe").unwrap_or(&lower);
    EXCLUDED_PREFIXES.iter().any(|p| stem.starts_with(p)) || EXCLUDED_CONTAINS.iter().any(|c| stem.contains(c))
}

const SKIPPED_DIRS: &[&str] = &["$recycle.bin", "system volume information", "windows", "programdata", "users", "recovery", "msocache", "perflogs"];

/// Sous-dossiers où chercher les exécutables de 2e niveau (§4.9).
const SECOND_LEVEL_DIRS: &[&[&str]] = &[&["bin"], &["binaries", "win64"], &["x64"], &["bin", "x64"], &["bin64"], &["win64"]];

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// Nom proposé (dossier du jeu, ou nom de l'exécutable isolé).
    pub name: String,
    /// Dossier du jeu ; `None` pour un exécutable posé directement dans le dossier surveillé.
    pub folder: Option<String>,
    pub exe: String,
    pub size: u64,
    /// Autres exécutables plausibles du même dossier.
    pub alternatives: Vec<String>,
}

fn list_exes(dir: &Path) -> Vec<(PathBuf, u64)> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let is_exe = path.extension().map(|x| x.eq_ignore_ascii_case("exe")).unwrap_or(false);
            let meta = e.metadata().ok()?;
            if !is_exe || !meta.is_file() || is_excluded_exe(&e.file_name().to_string_lossy()) {
                return None;
            }
            Some((path, meta.len()))
        })
        .collect()
}

fn child_dir_ci(parent: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(parent).ok()?.flatten().find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name) && e.path().is_dir()).map(|e| e.path())
}

/// Score d'un exécutable : le nom ressemble au dossier > premier niveau > taille.
fn score(folder_name: &str, exe: &Path, size: u64, top_level: bool) -> (bool, bool, u64) {
    let folder = normalize_title(folder_name).replace(' ', "");
    let stem = normalize_title(&exe.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()).replace(' ', "");
    let similar = !stem.is_empty() && stem.len() >= 3 && (folder.contains(&stem) || stem.contains(&folder));
    (similar, top_level, size)
}

/// Cherche le meilleur exécutable d'un dossier de jeu.
pub fn candidate_for_folder(folder: &Path) -> Option<Candidate> {
    let folder_name = folder.file_name()?.to_string_lossy().to_string();
    let mut found: Vec<(PathBuf, u64, bool)> = list_exes(folder).into_iter().map(|(p, s)| (p, s, true)).collect();
    for parts in SECOND_LEVEL_DIRS {
        let mut dir = Some(folder.to_path_buf());
        for part in *parts {
            dir = dir.and_then(|d| child_dir_ci(&d, part));
        }
        if let Some(dir) = dir {
            found.extend(list_exes(&dir).into_iter().map(|(p, s)| (p, s, false)));
        }
    }
    if found.is_empty() {
        return None;
    }
    found.sort_by(|a, b| score(&folder_name, &b.0, b.1, b.2).cmp(&score(&folder_name, &a.0, a.1, a.2)));
    let (best, size, _) = found[0].clone();
    Some(Candidate {
        name: folder_name,
        folder: Some(clean_path(&folder.to_string_lossy())),
        exe: clean_path(&best.to_string_lossy()),
        size,
        alternatives: found[1..].iter().take(8).map(|(p, _, _)| clean_path(&p.to_string_lossy())).collect(),
    })
}

/// Scanne un dossier surveillé : un candidat par sous-dossier contenant un exécutable plausible,
/// plus les exécutables posés directement à la racine (F-LOC-02/03).
pub fn scan_watched_folder(root: &Path) -> Vec<Candidate> {
    let Ok(entries) = std::fs::read_dir(root) else { return Vec::new() };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            let lower = name.to_lowercase();
            if lower.starts_with('.') || lower.starts_with('$') || SKIPPED_DIRS.contains(&lower.as_str()) {
                continue;
            }
            if let Some(c) = candidate_for_folder(&path) {
                out.push(c);
            }
        } else if path.extension().map(|x| x.eq_ignore_ascii_case("exe")).unwrap_or(false) && !is_excluded_exe(&name) {
            out.push(Candidate {
                name: path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or(name),
                folder: None,
                exe: clean_path(&path.to_string_lossy()),
                size: meta.len(),
                alternatives: Vec::new(),
            });
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// Dossiers trop génériques pour servir de « dossier du jeu » (suivi et dédoublonnage par dossier).
pub fn is_generic_dir(dir: &Path, extra: &[String]) -> bool {
    let norm = norm_path(&dir.to_string_lossy());
    if norm.len() <= 3 {
        return true; // racine de disque
    }
    let last = norm.rsplit('\\').next().unwrap_or("");
    if matches!(last, "desktop" | "bureau" | "downloads" | "téléchargements" | "documents" | "program files" | "program files (x86)" | "games" | "jeux") {
        return true;
    }
    extra.iter().any(|e| norm_path(e) == norm)
}

/// Construit l'entrée d'un jeu local à partir de son exécutable.
/// `folder` est le dossier du jeu s'il est connu (scan) ; sinon le dossier parent, sauf s'il est générique.
pub fn detected_from_exe(exe: &str, title: Option<&str>, folder: Option<&str>, args: Option<&str>, working_dir: Option<&str>, watched: &[String]) -> DetectedGame {
    let exe_clean = clean_path(exe);
    let exe_path = Path::new(&exe_clean);
    let install_path = match folder {
        Some(f) => Some(clean_path(f)),
        None => exe_path.parent().filter(|p| !is_generic_dir(p, watched)).map(|p| clean_path(&p.to_string_lossy())),
    };
    let default_title = match &install_path {
        Some(dir) if folder.is_some() => Path::new(dir).file_name().map(|s| s.to_string_lossy().to_string()),
        _ => exe_path.file_stem().map(|s| s.to_string_lossy().to_string()),
    };
    let clean_opt = |v: Option<&str>| v.map(str::trim).filter(|s| !s.is_empty()).map(|s| s.to_string());
    DetectedGame {
        source: SourceId::Local,
        external_id: norm_path(&exe_clean),
        title: clean_opt(title).or(default_title).unwrap_or_else(|| "Jeu".to_string()),
        install_path,
        executable_path: Some(exe_clean.clone()),
        launch_method: LaunchMethod::Exe,
        launch_target: exe_clean,
        launch_args: clean_opt(args),
        working_dir: clean_opt(working_dir),
        last_played: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn touch(path: &Path, size: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![0u8; size]).unwrap();
    }

    #[test]
    fn excludes_installers_and_tools() {
        for name in [
            "unins000.exe", "UnityCrashHandler64.exe", "vc_redist.x64.exe", "setup.exe", "dxsetup.exe", "UE4PrereqSetup_x64.exe", "CrashReportClient.exe",
            "Uninstall.exe", "QtWebEngineProcess.exe", "EasyAntiCheat_Setup.exe", "notification_helper.exe", "Updater.exe", "DXSETUP.EXE", "python.exe",
        ] {
            assert!(is_excluded_exe(name), "{name} devrait être exclu");
        }
        for name in ["Hades.exe", "game.exe", "Launcher.exe", "start_protected_game.exe", "Celeste.exe", "ProjectZomboid64.exe"] {
            assert!(!is_excluded_exe(name), "{name} ne devrait pas être exclu");
        }
    }

    #[test]
    fn scan_proposes_games_without_installers() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        // Jeu classique : exe au premier niveau + désinstalleur + crash handler.
        touch(&root.join("Hades/Hades.exe"), 500);
        touch(&root.join("Hades/unins000.exe"), 9000);
        touch(&root.join("Hades/UnityCrashHandler64.exe"), 8000);
        // Jeu Unreal : petit lanceur au premier niveau, gros binaire dans Binaries/Win64.
        touch(&root.join("Astro Quest/AstroQuest.exe"), 100);
        touch(&root.join("Astro Quest/Engine/Extras/Redist/en-us/UE4PrereqSetup_x64.exe"), 5000);
        touch(&root.join("Astro Quest/Binaries/Win64/AstroQuest-Win64-Shipping.exe"), 7000);
        // Exe uniquement dans bin/.
        touch(&root.join("OldGame/bin/og.exe"), 300);
        touch(&root.join("OldGame/bin/config.exe"), 200);
        // Dossier sans jeu : seulement un installeur.
        touch(&root.join("Drivers/setup.exe"), 1000);
        // Dossier sans exécutable, dossier système, exe à la racine.
        touch(&root.join("Musique/titre.mp3"), 10);
        touch(&root.join("$RECYCLE.BIN/x/game.exe"), 10);
        touch(&root.join("Portable.exe"), 50);
        touch(&root.join("vc_redist.x64.exe"), 50);

        let found = scan_watched_folder(root);
        let names: Vec<&str> = found.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["Astro Quest", "Hades", "OldGame", "Portable"]);

        let hades = &found[1];
        assert!(hades.exe.ends_with("Hades\\Hades.exe"));
        assert!(hades.alternatives.is_empty(), "désinstalleur et crash handler filtrés");

        // Le nom qui ressemble au dossier et le premier niveau l'emportent sur la taille.
        let astro = &found[0];
        assert!(astro.exe.ends_with("Astro Quest\\AstroQuest.exe"));
        assert_eq!(astro.alternatives.len(), 1);

        // À défaut de ressemblance, le plus volumineux.
        assert!(found[2].exe.ends_with("OldGame\\bin\\og.exe"));
        assert_eq!(found[3].folder, None);
    }

    #[test]
    fn builds_local_entries() {
        let d = detected_from_exe("D:/Jeux/Foo/bin/foo.exe", None, Some("D:/Jeux/Foo"), Some(" -w "), None, &[]);
        assert_eq!(d.title, "Foo");
        assert_eq!(d.install_path.as_deref(), Some("D:\\Jeux\\Foo"));
        assert_eq!(d.launch_target, "D:\\Jeux\\Foo\\bin\\foo.exe");
        assert_eq!(d.external_id, "d:\\jeux\\foo\\bin\\foo.exe");
        assert_eq!(d.launch_args.as_deref(), Some("-w"));

        // Ajout manuel : le dossier parent sert de dossier du jeu…
        let d = detected_from_exe("D:\\Stuff\\Bar\\bar.exe", Some("Bar Deluxe"), None, None, None, &[]);
        assert_eq!((d.title.as_str(), d.install_path.as_deref()), ("Bar Deluxe", Some("D:\\Stuff\\Bar")));
        // … sauf s'il est générique (Bureau, racine, dossier surveillé).
        assert_eq!(detected_from_exe("C:\\Users\\x\\Desktop\\tiny.exe", None, None, None, None, &[]).install_path, None);
        assert_eq!(detected_from_exe("E:\\tiny.exe", None, None, None, None, &[]).install_path, None);
        assert_eq!(detected_from_exe("D:\\Mes Jeux\\tiny.exe", None, None, None, None, &["D:/Mes Jeux/".to_string()]).install_path, None);
    }
}
