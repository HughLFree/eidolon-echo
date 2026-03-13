//! Menu-related commands: show/hide menu panel, keepalive and history mode transitions.

use super::super::state::{AnchorRect, MenuMode, OverlayState};
use super::super::window_labels::MENU_LABEL;
use super::super::window_manager::sync_menu_position;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub fn toggle_avatar_menu(
    app: AppHandle,
    state: State<Mutex<OverlayState>>,
    anchor: AnchorRect,
) -> Result<(), String> {
    let menu = app
        .get_webview_window(MENU_LABEL)
        .ok_or_else(|| "menu window not found".to_string())?;

    let should_show = {
        let mut overlay = state.lock().unwrap_or_else(|e| e.into_inner());
        if overlay.menu_visible {
            overlay.menu_visible = false;
            false
        } else {
            overlay.anchor = Some(anchor);
            overlay.menu_visible = true;
            overlay.menu_mode = MenuMode::Buttons;
            true
        }
    };

    if !should_show {
        menu.hide().map_err(|e| e.to_string())?;
        return Ok(());
    }

    sync_menu_position(&app)?;
    menu.show().map_err(|e| e.to_string())?;
    menu.set_ignore_cursor_events(false)
        .map_err(|e| e.to_string())?;
    menu.emit("menu:show", ()).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn menu_keep_alive(app: AppHandle, state: State<Mutex<OverlayState>>) -> Result<(), String> {
    let should_emit = {
        let overlay = state.lock().unwrap_or_else(|e| e.into_inner());
        overlay.menu_visible && overlay.menu_mode == MenuMode::Buttons
    };

    if !should_emit {
        return Ok(());
    }

    let menu = app
        .get_webview_window(MENU_LABEL)
        .ok_or_else(|| "menu window not found".to_string())?;
    menu.emit("menu:keepalive", true).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_history_panel(app: AppHandle, state: State<Mutex<OverlayState>>) -> Result<(), String> {
    let menu = app
        .get_webview_window(MENU_LABEL)
        .ok_or_else(|| "menu window not found".to_string())?;

    {
        let mut overlay = state.lock().unwrap_or_else(|e| e.into_inner());
        overlay.menu_visible = true;
        overlay.menu_mode = MenuMode::History;
    }

    sync_menu_position(&app)?;
    menu.show().map_err(|e| e.to_string())?;
    menu.set_ignore_cursor_events(false)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn hide_menu_window(app: AppHandle, state: State<Mutex<OverlayState>>) -> Result<(), String> {
    let menu = app
        .get_webview_window(MENU_LABEL)
        .ok_or_else(|| "menu window not found".to_string())?;

    {
        let mut overlay = state.lock().unwrap_or_else(|e| e.into_inner());
        overlay.menu_visible = false;
        overlay.menu_mode = MenuMode::Buttons;
    }

    menu.hide().map_err(|e| e.to_string())
}
