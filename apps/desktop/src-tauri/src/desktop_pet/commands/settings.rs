//! Settings panel commands: open settings window.

use super::super::{backend, window_manager::open_settings_window};
use tauri::AppHandle;

#[tauri::command]
pub fn open_settings_panel(app: AppHandle) -> Result<(), String> {
    open_settings_window(&app)
}

#[tauri::command]
pub fn clear_local_data(app: AppHandle) -> Result<backend::ClearLocalDataResult, String> {
    backend::clear_backend_local_data(&app)
}
