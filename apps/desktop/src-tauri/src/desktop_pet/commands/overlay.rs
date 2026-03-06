//! Overlay-related commands: runtime always-on-top level switching.

use crate::overlay;
use tauri::AppHandle;

#[tauri::command]
pub fn set_overlay_always_on_top(app: AppHandle, always_on_top: bool) -> Result<(), String> {
    overlay::set_overlay_always_on_top(&app, always_on_top)
}
