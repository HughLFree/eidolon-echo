//! Backend readiness commands for frontend windows.

use crate::desktop_pet::backend;
use tauri::AppHandle;

#[tauri::command]
pub fn ensure_backend_ready(app: AppHandle) -> Result<(), String> {
    backend::ensure_backend_running(&app)
}

