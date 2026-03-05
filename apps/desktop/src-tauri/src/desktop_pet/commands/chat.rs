//! Chat-related commands: bubble updates and dialog window show/hide toggles.

use crate::overlay;

use super::super::windows::sync_bubble_position;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn set_bubble_text(app: AppHandle, text: String) -> Result<(), String> {
    let bubble = app
        .get_webview_window("bubble")
        .ok_or_else(|| "bubble window not found".to_string())?;

    let _ = sync_bubble_position(&app);
    let _ = bubble.show();
    let _ = bubble.set_ignore_cursor_events(true);

    let payload = serde_json::to_string(&text).map_err(|e| e.to_string())?;
    bubble
        .eval(&format!("window.__desktopAiSetBubble({payload});"))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hide_dialog_window(app: AppHandle) -> Result<(), String> {
    overlay::hide_chat_panel(&app)
}

#[tauri::command]
pub fn overlay_show_chat_panel(app: AppHandle) -> Result<(), String> {
    overlay::show_chat_panel(&app, None)?;
    Ok(())
}

#[tauri::command]
pub fn overlay_toggle_chat_panel(app: AppHandle) -> Result<bool, String> {
    overlay::toggle_chat_panel(&app, None)
}

#[tauri::command]
pub fn overlay_hide_chat_panel(app: AppHandle) -> Result<(), String> {
    overlay::hide_chat_panel(&app)
}
