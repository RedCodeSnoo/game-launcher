//! Galerie de captures d'écran par jeu (V1.5) : Steam, Xbox Game Bar, NVIDIA et dossiers choisis. Lecture seule.
use crate::library::normalize_title;
use crate::models::SourceId;
use serde::Serialize;
use std::path::{Path, PathBuf};

const IMAGE_EXT: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp"];
const MAX_SHOTS: usize = 400;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub path: String,
    pub taken_at: Option<String>,
    pub source: &'static str,
}

fn is_image(p: &Path) -> bool {
    p.extension().map(|e| IMAGE_EXT.contains(&e.to_string_lossy().to_lowercase().as_str())).unwrap_or(false)
}

fn images_in(dir: &Path, source: &'static str, filter: impl Fn(&str) -> bool, out: &mut Vec<Screenshot>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if !p.is_file() || !is_image(&p) {
            continue;
        }
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if !filter(&name) {
            continue;
        }
        let taken_at = e.metadata().ok().and_then(|m| m.modified().ok()).map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        out.push(Screenshot { path: p.to_string_lossy().to_string(), taken_at, source });
    }
}

/// Le nom (sans extension) est le titre du jeu, éventuellement suivi d'une date ou d'un numéro
/// (« Hades 2026-11-02 21-10-03 ») : « Hades II 2026… » n'est pas une capture de « Hades ».
fn fold(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' | 'ì' => 'i',
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

pub fn title_matches(name: &str, title: &str) -> bool {
    let stem = Path::new(name).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let t = fold(&normalize_title(title));
    let n = fold(&normalize_title(if Path::new(name).extension().is_some() { &stem } else { name }));
    if t.is_empty() {
        return false;
    }
    n == t || n.strip_prefix(&format!("{t} ")).map_or(false, |rest| rest.starts_with(|c: char| c.is_ascii_digit()))
}

/// Dossiers où chercher : Steam (par appid), Captures de la Game Bar, dossier vidéo NVIDIA, dossiers personnels.
pub fn find(title: &str, sources: &[(SourceId, String)], steam_root: Option<&Path>, videos: Option<&Path>, custom: &[PathBuf]) -> (Vec<Screenshot>, Vec<PathBuf>) {
    let mut out = Vec::new();
    let mut dirs: Vec<PathBuf> = Vec::new();
    // Steam : userdata/<compte>/760/remote/<appid>/screenshots
    if let Some(root) = steam_root {
        for (source, appid) in sources.iter().filter(|(s, _)| *s == SourceId::Steam) {
            let _ = source;
            if let Ok(users) = std::fs::read_dir(root.join("userdata")) {
                for u in users.flatten() {
                    let dir = u.path().join("760").join("remote").join(appid).join("screenshots");
                    if dir.is_dir() {
                        images_in(&dir, "steam", |_| true, &mut out);
                        dirs.push(dir);
                    }
                }
            }
        }
    }
    if let Some(videos) = videos {
        // Xbox Game Bar : Vidéos\Captures\<Titre de la fenêtre> <date>.png
        let captures = videos.join("Captures");
        if captures.is_dir() {
            let before = out.len();
            images_in(&captures, "xbox", |name| title_matches(name, title), &mut out);
            if out.len() > before {
                dirs.push(captures);
            }
        }
        // NVIDIA : Vidéos\<Titre du jeu>\ (GeForce Experience) ou Vidéos\NVIDIA\<Titre du jeu>\ (application NVIDIA).
        for base in [videos.to_path_buf(), videos.join("NVIDIA")] {
            if let Ok(entries) = std::fs::read_dir(&base) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.is_dir() && title_matches(&e.file_name().to_string_lossy(), title) {
                        images_in(&p, "nvidia", |_| true, &mut out);
                        dirs.push(p);
                    }
                }
            }
        }
    }
    // Dossiers personnels : un sous-dossier au nom du jeu, ou des fichiers dont le nom commence par le titre.
    for folder in custom {
        if let Ok(entries) = std::fs::read_dir(folder) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() && title_matches(&e.file_name().to_string_lossy(), title) {
                    images_in(&p, "custom", |_| true, &mut out);
                    dirs.push(p);
                }
            }
        }
        let before = out.len();
        images_in(folder, "custom", |name| title_matches(name, title), &mut out);
        if out.len() > before {
            dirs.push(folder.clone());
        }
    }
    out.sort_by(|a, b| b.taken_at.cmp(&a.taken_at));
    out.truncate(MAX_SHOTS);
    dirs.sort();
    dirs.dedup();
    (out, dirs)
}

/// Dossier Vidéos de l'utilisateur.
pub fn videos_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("Videos")).filter(|p| p.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_steam_game_bar_nvidia_and_custom_screenshots() {
        let tmp = tempfile::tempdir().unwrap();
        let touch = |p: PathBuf| {
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        };
        let steam = tmp.path().join("Steam");
        touch(steam.join("userdata/123/760/remote/1145360/screenshots/20261101_1.jpg"));
        touch(steam.join("userdata/123/760/remote/1145360/screenshots/thumbnails/20261101_1.jpg"));
        touch(steam.join("userdata/123/760/remote/999/screenshots/autre.jpg"));
        let videos = tmp.path().join("Videos");
        touch(videos.join("Captures/Hades 2026-11-02 21-10-03.png"));
        touch(videos.join("Captures/Hades II 2026-11-02 21-10-03.png"));
        touch(videos.join("Captures/Celeste 2026-11-02.png"));
        touch(videos.join("Captures/Hades 2026-11-02 21-10-03.mp4"));
        touch(videos.join("Hades/Hades_2026.11.03.png"));
        touch(videos.join("NVIDIA/Hades/Hades 2026.11.05.png"));
        let custom = tmp.path().join("Mes captures");
        touch(custom.join("Hades/boss.png"));
        touch(custom.join("Hades_2026-11-04.jpg"));
        let (shots, dirs) = find("Hades", &[(SourceId::Steam, "1145360".into())], Some(&steam), Some(&videos), &[custom.clone()]);
        let mut by_source: Vec<&str> = shots.iter().map(|s| s.source).collect();
        by_source.sort();
        assert_eq!(by_source, vec!["custom", "custom", "nvidia", "nvidia", "steam", "xbox"], "{shots:#?}");
        assert!(shots.iter().all(|s| !s.path.contains("Hades II") && !s.path.ends_with(".mp4") && !s.path.contains("thumbnails")));
        assert_eq!(dirs.len(), 6);
    }

    #[test]
    fn titles_match_loosely_but_not_prefix_of_other_games() {
        assert!(title_matches("Hades 2026-11-02 21-10-03.png", "Hades"));
        assert!(title_matches("HOLLOW KNIGHT 1.png", "Hollow Knight"));
        assert!(!title_matches("Hollow Knight Silksong 1.png", "Hollow Knight"));
        assert!(title_matches("Pokémon Violet 1.png", "Pokemon Violet"));
        assert!(!title_matches("Hades II 2026.png", "Hades"));
        assert!(title_matches("Hades", "Hades"), "nom de dossier");
        assert!(!title_matches("Celeste.png", "Hades"));
    }
}
