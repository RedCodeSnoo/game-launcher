//! Icône de la zone de notification (E3) : ouvrir la fenêtre, jeux récents, jeu en cours, quitter.
use crate::state::Shared;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

const TRAY_ID: &str = "main";

/// Durée lisible pour le menu : « 12 min », « 1 h 05 ».
pub fn short_duration(secs: i64) -> String {
    let minutes = secs.max(0) / 60;
    if minutes < 60 {
        format!("{minutes} min")
    } else {
        format!("{} h {:02}", minutes / 60, minutes % 60)
    }
}

/// Icône de l'application, avec un point vert quand un jeu est en cours (E3-09).
fn icon(app: &AppHandle, running: bool) -> Option<Image<'static>> {
    let base = app.default_window_icon()?;
    let (w, h) = (base.width(), base.height());
    let mut rgba = base.rgba().to_vec();
    if running {
        dot(&mut rgba, w, h);
    }
    Some(Image::new_owned(rgba, w, h))
}

/// Dessine un disque vert bordé de sombre dans le coin inférieur droit.
pub fn dot(rgba: &mut [u8], w: u32, h: u32) {
    let radius = w.min(h) as f32 * 0.27;
    let (cx, cy) = (w as f32 - radius - 0.5, h as f32 - radius - 0.5);
    for y in 0..h {
        for x in 0..w {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            if d > radius {
                continue;
            }
            let color: [u8; 4] = if d > radius * 0.78 { [15, 17, 23, 255] } else { [34, 197, 94, 255] };
            let i = ((y * w + x) * 4) as usize;
            if i + 4 <= rgba.len() {
                rgba[i..i + 4].copy_from_slice(&color);
            }
        }
    }
}

fn build_menu(app: &AppHandle) -> tauri::Result<(Menu<Wry>, usize, String)> {
    let state = app.state::<Shared>();
    let running = state.tracker.running_info();
    let recent = {
        let disabled = state.disabled_sources();
        crate::library::recent_games(&state.conn(), &disabled, 5).unwrap_or_default()
    };
    let now = chrono::Utc::now().timestamp();
    let none = None::<&str>;

    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(app, "open", "Ouvrir GameLauncher", true, none)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    let recents = Submenu::new(app, "Jeux récents", true)?;
    if recent.is_empty() {
        recents.append(&MenuItem::new(app, "Aucun jeu lancé récemment", false, none)?)?;
    }
    for g in &recent {
        let label = if g.playtime_minutes > 0 { format!("{} – {}", g.title, short_duration(g.playtime_minutes * 60)) } else { g.title.clone() };
        recents.append(&MenuItem::with_id(app, format!("launch:{}", g.id), label.replace('&', "&&"), true, none)?)?;
    }
    menu.append(&recents)?;

    let mut tooltip = String::from("GameLauncher");
    if running.is_empty() {
        menu.append(&MenuItem::new(app, "Aucun jeu en cours", false, none)?)?;
    }
    for r in &running {
        let label = match r.started_at {
            Some(start) => format!("En cours : {} – {}", r.title, short_duration(now - start)),
            None => format!("Lancement : {}", r.title),
        };
        tooltip = format!("GameLauncher – {label}");
        menu.append(&MenuItem::new(app, label.replace('&', "&&"), false, none)?)?;
        menu.append(&MenuItem::with_id(app, format!("stop:{}", r.game_id), "    Arrêter le suivi", true, none)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Quitter", true, none)?)?;
    Ok((menu, running.len(), tooltip))
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let (menu, running, tooltip) = build_menu(app)?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip(tooltip)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref().to_string();
            let app = app.clone();
            // Hors du fil principal : un lancement ou la création de la fenêtre ne doit pas bloquer l'interface.
            std::thread::spawn(move || on_menu(&app, &id));
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                crate::show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = icon(app, running > 0) {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

fn on_menu(app: &AppHandle, id: &str) {
    let state = app.state::<Shared>().inner().clone();
    match id {
        "open" => crate::show_main_window(app),
        "quit" => crate::quit(app),
        _ => {
            if let Some(game_id) = id.strip_prefix("launch:") {
                if let Err(e) = crate::commands::do_launch(app, &state, game_id, None) {
                    log::warn!("lancement depuis la zone de notification impossible : {} {}", e.code, e.detail);
                    crate::notify(app, "Lancement impossible", "Ouvrez GameLauncher pour voir le détail de l'erreur.");
                    crate::show_main_window(app);
                }
            } else if let Some(game_id) = id.strip_prefix("stop:") {
                state.tracker.stop_tracking(&state, &crate::commands::emitter(app), game_id);
            }
        }
    }
}

/// Reconstruit le menu, l'icône et l'infobulle (jeu en cours, jeux récents). Sans attente pour l'appelant.
pub fn refresh(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(tray) = handle.tray_by_id(TRAY_ID) else { return };
        match build_menu(&handle) {
            Ok((menu, running, tooltip)) => {
                let _ = tray.set_menu(Some(menu));
                let _ = tray.set_tooltip(Some(tooltip));
                if let Some(icon) = icon(&handle, running > 0) {
                    let _ = tray.set_icon(Some(icon));
                }
            }
            Err(e) => log::warn!("menu de la zone de notification : {e}"),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_are_short_and_readable() {
        assert_eq!(short_duration(0), "0 min");
        assert_eq!(short_duration(59 * 60 + 59), "59 min");
        assert_eq!(short_duration(3900), "1 h 05");
        assert_eq!(short_duration(-5), "0 min");
    }

    #[test]
    fn dot_is_drawn_in_the_bottom_right_corner_only() {
        let (w, h) = (32u32, 32u32);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        dot(&mut rgba, w, h);
        let px = |x: u32, y: u32| rgba[((y * w + x) * 4) as usize..][..4].to_vec();
        assert_eq!(px(23, 23), vec![34, 197, 94, 255], "centre du point : vert");
        assert_eq!(px(0, 0), vec![0, 0, 0, 0], "le reste de l'icône n'est pas touché");
        assert_eq!(px(8, 23), vec![0, 0, 0, 0]);
    }
}
