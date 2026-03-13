//! Desktop pet runtime composition: state setup, command registration and app bootstrapping.

mod backend;
mod commands;
mod state;
mod window_manager;

use state::{AiModePayload, AiRoleMode, OverlayState};
use std::{sync::Mutex, thread};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, SubmenuBuilder},
    tray::TrayIconBuilder,
    Emitter, Manager,
};
use tracing::{error, warn};
use window_manager::{
    bootstrap_desktop_pet, open_settings_window, sync_bubble_position, sync_chat_position,
    sync_menu_position, toggle_pet_windows,
};

fn bootstrap_overlay_deferred(app_handle: tauri::AppHandle) {
    thread::spawn(move || {
        match crate::overlay::bootstrap(&app_handle) {
            Ok(()) => {
                let _ = sync_bubble_position(&app_handle);
                let _ = sync_chat_position(&app_handle);
                let _ = sync_menu_position(&app_handle);
            }
            Err(error) => {
                error!("overlay bootstrap failed: {error}");
                // Fallback so windows remain usable even if panel bootstrap fails.
                if let Err(visibility_error) =
                    crate::overlay::apply_visibility(&app_handle, true, true, false, false)
                {
                    error!("overlay fallback visibility failed: {visibility_error}");
                }
            }
        }
    });
}

fn set_ai_role_mode(app: &tauri::AppHandle, mode: AiRoleMode) -> Result<(), String> {
    let payload = {
        let overlay_state: tauri::State<Mutex<OverlayState>> = app.state();
        let mut overlay = overlay_state.lock().unwrap_or_else(|e| e.into_inner());
        if overlay.ai_role_mode == mode {
            None
        } else {
            overlay.ai_role_mode = mode;
            Some(AiModePayload::from(mode))
        }
    };

    if let Some(payload) = payload {
        app.emit("ai:mode-changed", payload)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn setup_tray(app: &tauri::App) -> Result<(), tauri::Error> {
    let mode_default_item =
        CheckMenuItem::with_id(app, "ai-mode-default", "默认模式", true, true, None::<&str>)?;
    let mode_roleplay_item = CheckMenuItem::with_id(
        app,
        "ai-mode-roleplay",
        "扮演模式",
        true,
        false,
        None::<&str>,
    )?;
    let mode_submenu = SubmenuBuilder::with_id(app, "ai-mode-submenu", "角色模式")
        .items(&[&mode_default_item, &mode_roleplay_item])
        .build()?;
    let toggle_item = MenuItem::with_id(
        app,
        "toggle-visibility",
        "Minimize/Restore",
        true,
        None::<&str>,
    )?;
    let settings_item = MenuItem::with_id(app, "open-settings", "设置", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let tray_menu = Menu::with_items(
        app,
        &[&mode_submenu, &settings_item, &toggle_item, &quit_item],
    )?;

    let mut tray_builder = TrayIconBuilder::with_id("main-tray").menu(&tray_menu);
    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    tray_builder
        .on_menu_event(
            move |app: &tauri::AppHandle, event| match event.id.as_ref() {
                "ai-mode-default" => {
                    if let Err(error) = set_ai_role_mode(app, AiRoleMode::Default) {
                        warn!("set tray ai mode failed: {error}");
                    }
                    let _ = mode_default_item.set_checked(true);
                    let _ = mode_roleplay_item.set_checked(false);
                }
                "ai-mode-roleplay" => {
                    if let Err(error) = set_ai_role_mode(app, AiRoleMode::Roleplay) {
                        warn!("set tray ai mode failed: {error}");
                    }
                    let _ = mode_default_item.set_checked(false);
                    let _ = mode_roleplay_item.set_checked(true);
                }
                "toggle-visibility" => {
                    if let Err(error) = toggle_pet_windows(app) {
                        warn!("toggle tray visibility failed: {error}");
                    }
                }
                "open-settings" => {
                    if let Err(error) = open_settings_window(app) {
                        warn!("open settings failed: {error}");
                    }
                }
                "quit" => {
                    backend::stop_backend_process(app);
                    app.exit(0);
                }
                _ => {}
            },
        )
        .build(app)?;

    Ok(())
}

pub fn run() {
    std::panic::set_hook(Box::new(|info| {
        error!("panic hook: {info}");
    }));

    let app = tauri::Builder::default()
        .manage(Mutex::new(OverlayState::default()))
        .manage(backend::BackendProcessState::default())
        .setup(|app| {
            setup_tray(app)?;
            let app_handle = app.handle().clone();
            if let Err(error) = backend::ensure_backend_running(&app_handle) {
                error!("backend startup failed: {error}");
            }
            if let Err(error) = crate::overlay::prepare_startup(&app_handle) {
                error!("overlay startup preparation failed: {error}");
            }
            bootstrap_desktop_pet(&app_handle);
            bootstrap_overlay_deferred(app_handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ai::get_ai_mode,
            commands::avatar::start_window_drag,
            commands::chat::set_bubble_text,
            commands::chat::set_bubble_interactive,
            commands::menu::toggle_avatar_menu,
            commands::menu::menu_keep_alive,
            commands::menu::open_history_panel,
            commands::menu::hide_menu_window,
            commands::settings::open_settings_panel,
            commands::settings::clear_local_data,
            commands::overlay::set_overlay_always_on_top,
            commands::avatar::hide_pet
        ])
        .build(tauri::generate_context!())
        .unwrap_or_else(|error| {
            error!("error while running tauri application: {error}");
            std::process::exit(1);
        });

    app.run(|app_handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            backend::stop_backend_process(app_handle);
        }
    });
}
