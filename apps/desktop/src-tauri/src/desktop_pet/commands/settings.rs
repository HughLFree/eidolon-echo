//! Settings panel commands: open settings window.

use super::super::window_manager::open_settings_window;
use tauri::AppHandle;

#[tauri::command]
pub fn open_settings_panel(app: AppHandle) -> Result<(), String> {
    open_settings_window(&app)
}
