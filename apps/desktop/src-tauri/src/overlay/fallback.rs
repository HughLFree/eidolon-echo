//! Fallback floating-window overlay initialization for unsupported platforms.

use tauri::{AppHandle, Manager};

const OVERLAY_WINDOW_LABELS: &[&str] = &["main", "chat", "bubble", "menu"];

pub fn bootstrap(app: &AppHandle) -> Result<(), String> {
    let _ = app;
    Ok(())
}

pub fn prepare_startup(app: &AppHandle) -> Result<(), String> {
    let _ = app;
    Ok(())
}

pub fn configure_runtime(app: &AppHandle) -> Result<(), String> {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.set_visible_on_all_workspaces(true);
        let _ = main.set_always_on_top(true);
    }
    if let Some(chat) = app.get_webview_window("chat") {
        let _ = chat.set_visible_on_all_workspaces(true);
        let _ = chat.set_always_on_top(true);
        let _ = chat.set_ignore_cursor_events(false);
    }
    if let Some(bubble) = app.get_webview_window("bubble") {
        let _ = bubble.set_visible_on_all_workspaces(true);
        let _ = bubble.set_ignore_cursor_events(true);
        let _ = bubble.set_always_on_top(true);
    }
    if let Some(menu) = app.get_webview_window("menu") {
        let _ = menu.set_visible_on_all_workspaces(true);
        let _ = menu.set_always_on_top(true);
        let _ = menu.set_ignore_cursor_events(false);
        let _ = menu.hide();
    }
    Ok(())
}

pub fn set_overlay_always_on_top(app: &AppHandle, always_on_top: bool) -> Result<(), String> {
    for label in OVERLAY_WINDOW_LABELS {
        if let Some(window) = app.get_webview_window(label) {
            window
                .set_always_on_top(always_on_top)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn apply_visibility(
    app: &AppHandle,
    show_main: bool,
    show_chat: bool,
    show_bubble: bool,
    show_menu: bool,
) -> Result<(), String> {
    if let Some(main) = app.get_webview_window("main") {
        if show_main {
            main.show().map_err(|e| e.to_string())?;
        } else {
            main.hide().map_err(|e| e.to_string())?;
        }
    }
    if let Some(chat) = app.get_webview_window("chat") {
        if show_chat {
            chat.show().map_err(|e| e.to_string())?;
        } else {
            chat.hide().map_err(|e| e.to_string())?;
        }
    }
    if let Some(bubble) = app.get_webview_window("bubble") {
        if show_bubble {
            bubble.show().map_err(|e| e.to_string())?;
        } else {
            bubble.hide().map_err(|e| e.to_string())?;
        }
    }
    if let Some(menu) = app.get_webview_window("menu") {
        if show_menu {
            menu.show().map_err(|e| e.to_string())?;
        } else {
            menu.hide().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
