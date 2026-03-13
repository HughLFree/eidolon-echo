//! Chat-related commands: bubble updates.

use super::super::window_labels::BUBBLE_LABEL;
use super::super::window_manager::sync_bubble_position;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn set_bubble_text(app: AppHandle, text: String) -> Result<(), String> {
    let bubble = app
        .get_webview_window(BUBBLE_LABEL)
        .ok_or_else(|| "bubble window not found".to_string())?;

    let _ = sync_bubble_position(&app);
    let _ = bubble.show();
    let _ = bubble.set_ignore_cursor_events(false);

    let payload = serde_json::to_string(&text).map_err(|e| e.to_string())?;
    bubble
        .eval(&format!("window.__desktopAiSetBubble({payload});"))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_bubble_interactive(app: AppHandle, interactive: bool) -> Result<(), String> {
    let bubble = app
        .get_webview_window(BUBBLE_LABEL)
        .ok_or_else(|| "bubble window not found".to_string())?;
    bubble
        .set_ignore_cursor_events(!interactive)
        .map_err(|e| e.to_string())
}
