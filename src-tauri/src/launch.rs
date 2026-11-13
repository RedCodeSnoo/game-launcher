//! LaunchService : démarrage d'un jeu selon sa méthode (URI, CLI du launcher, exécutable direct).
use crate::error::AppError;
use crate::models::{GameSource, LaunchMethod};
use anyhow::Result;
use std::path::Path;
use std::process::Command;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Ouvre une URI, un fichier ou un dossier avec le programme associé (équivalent de « Exécuter »).
pub fn shell_open(target: &str, args: Option<&str>, dir: Option<&str>) -> Result<()> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    let op = wide("open");
    let file = wide(target);
    let params = args.map(wide);
    let directory = dir.map(wide);
    let code = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            file.as_ptr(),
            params.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()),
            directory.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()),
            1, // SW_SHOWNORMAL
        )
    } as isize;
    if code > 32 {
        Ok(())
    } else {
        // 31 = SE_ERR_NOASSOC : aucun programme n'est associé (launcher absent).
        let app = if code == 31 || code == 2 || code == 3 {
            AppError::new("LAUNCHER_MISSING", target.to_string())
        } else {
            AppError::new("LAUNCH_FAILED", format!("{target} (code {code})"))
        };
        Err(app.into())
    }
}

/// Découpe une ligne d'arguments en respectant les guillemets.
pub fn split_args(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut has_token = false;
    for c in line.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                has_token = true;
            }
            c if c.is_whitespace() && !in_quotes => {
                if has_token {
                    out.push(std::mem::take(&mut cur));
                    has_token = false;
                }
            }
            c => {
                cur.push(c);
                has_token = true;
            }
        }
    }
    if has_token {
        out.push(cur);
    }
    out
}

pub fn launch_source(g: &GameSource) -> Result<()> {
    let args = g.launch_args.as_deref().map(str::trim).filter(|a| !a.is_empty());
    match g.launch_method {
        LaunchMethod::Uri => shell_open(&g.launch_target, None, None),
        LaunchMethod::LauncherCli => {
            if !Path::new(&g.launch_target).is_file() {
                return Err(AppError::new("LAUNCHER_MISSING", g.launch_target.clone()).into());
            }
            let mut cmd = Command::new(&g.launch_target);
            if let Some(a) = args {
                cmd.args(split_args(a));
            }
            if let Some(dir) = Path::new(&g.launch_target).parent() {
                cmd.current_dir(dir);
            }
            cmd.spawn().map_err(|e| AppError::new("LAUNCH_FAILED", format!("{} : {e}", g.launch_target)))?;
            Ok(())
        }
        LaunchMethod::Exe => {
            let exe = Path::new(&g.launch_target);
            if !exe.is_file() {
                return Err(AppError::new("FILE_NOT_FOUND", g.launch_target.clone()).into());
            }
            let workdir = g
                .working_dir
                .as_deref()
                .filter(|w| !w.trim().is_empty() && Path::new(w).is_dir())
                .map(|w| w.to_string())
                .or_else(|| exe.parent().map(|p| p.to_string_lossy().to_string()));
            let mut cmd = Command::new(exe);
            if let Some(a) = args {
                cmd.args(split_args(a));
            }
            if let Some(dir) = &workdir {
                cmd.current_dir(dir);
            }
            match cmd.spawn() {
                Ok(_) => Ok(()),
                // 740 : l'exécutable demande une élévation ; Windows affichera lui-même l'invite UAC.
                Err(e) if e.raw_os_error() == Some(740) => shell_open(&g.launch_target, args, workdir.as_deref()),
                Err(e) => Err(AppError::new("LAUNCH_FAILED", format!("{} : {e}", g.launch_target)).into()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SourceId;

    #[test]
    fn splits_arguments_with_quotes() {
        assert_eq!(split_args("-windowed  -w 1280"), vec!["-windowed", "-w", "1280"]);
        assert_eq!(split_args("--exec=\"launch WoW\" -x"), vec!["--exec=launch WoW", "-x"]);
        assert_eq!(split_args(r#""C:\My Games\save.dat" """#), vec![r"C:\My Games\save.dat", ""]);
        assert!(split_args("   ").is_empty());
    }

    #[test]
    fn missing_exe_gives_clear_error() {
        let g = GameSource {
            id: "s".into(),
            game_id: "g".into(),
            source: SourceId::Local,
            external_id: "x".into(),
            installed: true,
            install_path: None,
            volume_serial: None,
            executable_path: None,
            launch_method: LaunchMethod::Exe,
            launch_target: r"C:\dossier_absent\jeu.exe".into(),
            launch_args: None,
            working_dir: None,
            source_playtime_minutes: None,
            last_played_source: None,
            available: true,
            owned: false,
        };
        let err: AppError = launch_source(&g).unwrap_err().into();
        assert_eq!(err.code, "FILE_NOT_FOUND");
    }
}
