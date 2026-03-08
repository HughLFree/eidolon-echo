//! Windows floating-window overlay initialization.

use tauri::{AppHandle, Manager};

const OVERLAY_WINDOW_LABELS: &[&str] = &["main", "chat", "bubble", "menu"];

pub fn bootstrap(app: &AppHandle) -> Result<(), String> {
    let _ = app;
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

pub fn refresh_child_relationships(app: &AppHandle) -> Result<(), String> {
    let _ = app;
    Ok(())
}
