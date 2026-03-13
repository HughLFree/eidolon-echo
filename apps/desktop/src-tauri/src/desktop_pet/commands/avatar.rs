//! Avatar-related commands: window dragging and hiding the desktop pet windows.

use super::super::window_manager::hide_pet_windows;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn start_window_drag(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    window
        .start_dragging()
        .map_err(|e: tauri::Error| e.to_string())
}

#[tauri::command]
pub fn hide_pet(app: AppHandle) -> Result<(), String> {
    hide_pet_windows(&app)
}
