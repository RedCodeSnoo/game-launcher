//! Journal applicatif dans un fichier (consultable depuis les paramètres, §6.3).
use log::{Level, LevelFilter, Log, Metadata, Record};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const MAX_SIZE: u64 = 1024 * 1024;

struct FileLogger {
    file: Mutex<Option<File>>,
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info && metadata.target().starts_with("game_launcher")
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!("{} [{}] {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), record.level(), record.args());
        if cfg!(debug_assertions) {
            eprint!("{line}");
        }
        if let Ok(mut guard) = self.file.lock() {
            if let Some(f) = guard.as_mut() {
                let _ = f.write_all(line.as_bytes());
            }
        }
    }

    fn flush(&self) {}
}

pub fn log_path(data_dir: &Path) -> PathBuf {
    data_dir.join("logs").join("launcher.log")
}

pub fn init(data_dir: &Path) {
    let path = log_path(data_dir);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Rotation simple : un seul fichier précédent conservé.
    if std::fs::metadata(&path).map(|m| m.len() > MAX_SIZE).unwrap_or(false) {
        let _ = std::fs::rename(&path, path.with_extension("old.log"));
    }
    let file = OpenOptions::new().create(true).append(true).open(&path).ok();
    if log::set_boxed_logger(Box::new(FileLogger { file: Mutex::new(file) })).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}

/// Dernières lignes du journal.
pub fn tail(data_dir: &Path, max_lines: usize) -> String {
    let content = std::fs::read_to_string(log_path(data_dir)).unwrap_or_default();
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}
