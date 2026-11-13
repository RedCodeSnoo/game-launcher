// Pas de console en release sous Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    game_launcher_lib::run()
}
