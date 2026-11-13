//! Lecteurs montés, numéros de série de volume et normalisation de chemins (F-EXT-01/02).
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    pub letter: char,
    pub serial: Option<String>,
    pub removable: bool,
}

/// Photographie des lecteurs montés. Injectable dans les tests.
#[derive(Debug, Clone, Default)]
pub struct DriveSnapshot {
    pub drives: Vec<DriveInfo>,
}

impl DriveSnapshot {
    pub fn current() -> Self {
        DriveSnapshot { drives: list_drives() }
    }

    pub fn has_letter(&self, letter: char) -> bool {
        let l = letter.to_ascii_uppercase();
        self.drives.iter().any(|d| d.letter == l)
    }

    pub fn serial_of(&self, letter: char) -> Option<&str> {
        let l = letter.to_ascii_uppercase();
        self.drives.iter().find(|d| d.letter == l).and_then(|d| d.serial.as_deref())
    }

    pub fn serial_of_path(&self, path: &str) -> Option<String> {
        drive_letter(path).and_then(|l| self.serial_of(l)).map(|s| s.to_string())
    }

    pub fn letter_for_serial(&self, serial: &str) -> Option<char> {
        self.drives.iter().find(|d| d.serial.as_deref() == Some(serial)).map(|d| d.letter)
    }
}

/// Lettre de lecteur d'un chemin absolu `X:\...`.
pub fn drive_letter(path: &str) -> Option<char> {
    let mut chars = path.chars();
    let first = chars.next()?;
    if first.is_ascii_alphabetic() && chars.next() == Some(':') {
        Some(first.to_ascii_uppercase())
    } else {
        None
    }
}

/// Remplace la lettre de lecteur d'un chemin.
pub fn with_drive_letter(path: &str, letter: char) -> String {
    if drive_letter(path).is_some() {
        format!("{}{}", letter.to_ascii_uppercase(), &path[1..])
    } else {
        path.to_string()
    }
}

/// Forme canonique pour comparer des chemins : antislashs, minuscules, sans séparateur final.
pub fn norm_path(path: &str) -> String {
    let mut p = path.trim().replace('/', "\\").to_lowercase();
    while p.contains("\\\\") && !p.starts_with("\\\\") {
        p = p.replace("\\\\", "\\");
    }
    while p.len() > 3 && p.ends_with('\\') {
        p.pop();
    }
    p
}

/// Forme d'affichage : antislashs, sans séparateur final.
pub fn clean_path(path: &str) -> String {
    let mut p = path.trim().replace('/', "\\");
    while p.len() > 3 && p.ends_with('\\') {
        p.pop();
    }
    p
}

/// Vrai si `child` est `parent` ou se trouve dedans (chemins normalisés).
pub fn is_inside(child_norm: &str, parent_norm: &str) -> bool {
    child_norm == parent_norm
        || (child_norm.starts_with(parent_norm)
            && (parent_norm.ends_with('\\') || child_norm.as_bytes().get(parent_norm.len()) == Some(&b'\\')))
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn list_drives() -> Vec<DriveInfo> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW};
    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;
    let mask = unsafe { GetLogicalDrives() };
    let mut out = Vec::new();
    for i in 0..26u32 {
        if mask & (1 << i) == 0 {
            continue;
        }
        let letter = (b'A' + i as u8) as char;
        let root = wide(&format!("{letter}:\\"));
        let kind = unsafe { GetDriveTypeW(root.as_ptr()) };
        if kind != DRIVE_REMOVABLE && kind != DRIVE_FIXED {
            continue; // réseau, CD-ROM, RAM disk : ignorés
        }
        let mut serial: u32 = 0;
        let ok = unsafe {
            GetVolumeInformationW(
                root.as_ptr(),
                std::ptr::null_mut(),
                0,
                &mut serial,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            )
        };
        if ok == 0 {
            continue; // lecteur sans média (lecteur de cartes vide)
        }
        out.push(DriveInfo { letter, serial: Some(format!("{serial:08X}")), removable: kind == DRIVE_REMOVABLE });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_paths() {
        assert_eq!(norm_path("D:/Jeux/Foo/"), "d:\\jeux\\foo");
        assert_eq!(norm_path("C:\\"), "c:\\");
        assert_eq!(norm_path("D:\\\\SteamLibrary\\\\x"), "d:\\steamlibrary\\x");
        assert_eq!(clean_path("D:/jeu/Riot Games/"), "D:\\jeu\\Riot Games");
    }

    #[test]
    fn drive_letter_helpers() {
        assert_eq!(drive_letter("e:\\Games"), Some('E'));
        assert_eq!(drive_letter("\\\\server\\share"), None);
        assert_eq!(with_drive_letter("E:\\Games\\a.exe", 'f'), "F:\\Games\\a.exe");
    }

    #[test]
    fn inside_checks_boundaries() {
        assert!(is_inside("d:\\jeux\\foo\\bin\\a.exe", "d:\\jeux\\foo"));
        assert!(is_inside("d:\\jeux\\foo", "d:\\jeux\\foo"));
        assert!(!is_inside("d:\\jeux\\foobar\\a.exe", "d:\\jeux\\foo"));
        assert!(is_inside("d:\\a.exe", "d:\\"));
    }

    #[test]
    fn system_drive_has_serial() {
        let snap = DriveSnapshot::current();
        assert!(snap.has_letter('C'));
        let serial = snap.serial_of('C').unwrap().to_string();
        assert_eq!(snap.letter_for_serial(&serial), Some('C'));
    }
}
