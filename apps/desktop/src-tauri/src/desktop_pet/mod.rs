//! Desktop pet runtime composition: state setup, command registration and app bootstrapping.

mod commands;
mod state;
mod windows;

use state::OverlayState;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use windows::{bootstrap_desktop_pet, toggle_pet_windows};

fn setup_tray(app: &tauri::App) -> Result<(), tauri::Error> {
    let toggle_item = MenuItem::with_id(app, "toggle-visibility", "Minimize/Restore", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let tray_menu = Menu::with_items(app, &[&toggle_item, &quit_item])?;

    let mut tray_builder = TrayIconBuilder::with_id("main-tray").menu(&tray_menu);
    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    tray_builder
        .on_menu_event(|app: &tauri::AppHandle, event| {
            match event.id.as_ref() {
                "toggle-visibility" => {
                    if let Err(error) = toggle_pet_windows(app) {
                        eprintln!("toggle tray visibility failed: {error}");
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

pub fn run() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("panic hook: {info}");
    }));

    let builder = tauri::Builder::default();

    builder
        .manage(Mutex::new(OverlayState::default()))
        .setup(|app| {
            setup_tray(app)?;
            let app_handle = app.handle().clone();
            bootstrap_desktop_pet(&app_handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::avatar::start_window_drag,
            commands::chat::set_bubble_text,
            commands::menu::toggle_avatar_menu,
            commands::menu::menu_keep_alive,
            commands::menu::open_history_panel,
            commands::menu::hide_menu_window,
            commands::overlay::set_overlay_always_on_top,
            commands::avatar::hide_pet
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            eprintln!("error while running tauri application: {error}");
        });
}
