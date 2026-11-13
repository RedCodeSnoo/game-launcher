//! LaunchService : démarrage d'un jeu selon sa méthode (URI, CLI du launcher, exécutable direct).
use crate::error::AppError;
use crate::models::{GameSource, LaunchMethod, SourceId};
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
        LaunchMethod::Exe => run_exe(&g.launch_target, args, g.working_dir.as_deref()),
    }
}

/// Démarre un exécutable avec ses arguments ; dossier de travail par défaut : celui de l'exécutable.
fn run_exe(target: &str, args: Option<&str>, working_dir: Option<&str>) -> Result<()> {
    let exe = Path::new(target);
    if !exe.is_file() {
        return Err(AppError::new("FILE_NOT_FOUND", target.to_string()).into());
    }
    let workdir = working_dir
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
        Err(e) if e.raw_os_error() == Some(740) => shell_open(target, args, workdir.as_deref()),
        Err(e) => Err(AppError::new("LAUNCH_FAILED", format!("{target} : {e}")).into()),
    }
}

/// Ce qu'un lancement « exécutable direct » démarre : (exécutable, arguments, dossier de travail).
/// Jeu local : son exécutable (personnalisé ou d'origine) et ses propres réglages. Jeu d'un launcher :
/// l'exécutable principal effectif et les réglages du mode direct (E2-05, E2-08).
pub fn direct_command(g: &GameSource) -> Result<(String, Option<String>, Option<String>)> {
    let pick = |v: &Option<String>| v.as_deref().map(str::trim).filter(|a| !a.is_empty()).map(|a| a.to_string());
    if g.source == SourceId::Local {
        let exe = g.effective_exe().unwrap_or(&g.launch_target).to_string();
        return Ok((exe, pick(&g.launch_args), pick(&g.working_dir)));
    }
    let exe = g.effective_exe().ok_or_else(|| AppError::new("EXE_NOT_SET", ""))?.to_string();
    Ok((exe, pick(&g.direct_args), pick(&g.direct_working_dir)))
}

pub fn launch_direct(g: &GameSource) -> Result<()> {
    let (exe, args, dir) = direct_command(g)?;
    run_exe(&exe, args.as_deref(), dir.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

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
            install_path_override: None,
            executable_path_override: None,
            launch_mode: None,
            direct_args: None,
            direct_working_dir: None,
        };
        let err: AppError = launch_source(&g).unwrap_err().into();
        assert_eq!(err.code, "FILE_NOT_FOUND");

        // Exécutable direct (E2) : jeu local = son exécutable, remplacé par la valeur personnalisée si elle existe.
        assert_eq!(direct_command(&g).unwrap().0, r"C:\dossier_absent\jeu.exe");
        let custom = GameSource { executable_path_override: Some(r"D:\autre\vrai.exe".into()), launch_args: Some("-w".into()), ..g.clone() };
        assert_eq!(direct_command(&custom).unwrap(), (r"D:\autre\vrai.exe".to_string(), Some("-w".to_string()), None));
        // Jeu d'un launcher : exécutable effectif et réglages du mode direct, pas ceux du launcher.
        let steam = GameSource { source: SourceId::Steam, launch_method: LaunchMethod::Uri, launch_target: "steam://rungameid/1".into(), launch_args: Some("--launcher".into()), ..g.clone() };
        let err: AppError = direct_command(&steam).unwrap_err().into();
        assert_eq!(err.code, "EXE_NOT_SET");
        let steam = GameSource { executable_path: Some(r"C:\jeu\detecte.exe".into()), direct_args: Some("-dx11".into()), direct_working_dir: Some(r"C:\jeu".into()), ..steam };
        assert_eq!(direct_command(&steam).unwrap(), (r"C:\jeu\detecte.exe".to_string(), Some("-dx11".to_string()), Some(r"C:\jeu".to_string())));
    }
}
