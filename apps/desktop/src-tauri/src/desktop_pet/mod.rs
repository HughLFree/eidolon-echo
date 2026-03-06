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
use windows::bootstrap_desktop_pet;

fn setup_tray(app: &tauri::App) -> Result<(), tauri::Error> {
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let tray_menu = Menu::with_items(app, &[&quit_item])?;

    let mut tray_builder = TrayIconBuilder::with_id("main-tray").menu(&tray_menu);
    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    tray_builder
        .on_menu_event(|app: &tauri::AppHandle, event| {
            if event.id.as_ref() == "quit" {
                app.exit(0);
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
            commands::avatar::hide_pet,
            commands::chat::hide_dialog_window,
            commands::chat::overlay_show_chat_panel,
            commands::chat::overlay_toggle_chat_panel,
            commands::chat::overlay_hide_chat_panel
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            eprintln!("error while running tauri application: {error}");
        });
}
