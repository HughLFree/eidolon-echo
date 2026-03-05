//! Platform dispatch layer for dialog overlay behavior.

use tauri::{AppHandle, PhysicalPosition};

#[cfg(not(any(target_os = "macos", windows)))]
mod fallback;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(not(any(target_os = "macos", windows)))]
use fallback as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(windows)]
use windows as platform;

pub fn bootstrap(app: &AppHandle) -> Result<(), String> {
    platform::bootstrap(app)
}

pub fn show_chat_panel(
    app: &AppHandle,
    tray_pos: Option<PhysicalPosition<f64>>,
) -> Result<(), String> {
    platform::show_chat_panel(app, tray_pos)
}

pub fn hide_chat_panel(app: &AppHandle) -> Result<(), String> {
    platform::hide_chat_panel(app)
}

pub fn toggle_chat_panel(
    app: &AppHandle,
    tray_pos: Option<PhysicalPosition<f64>>,
) -> Result<bool, String> {
    platform::toggle_chat_panel(app, tray_pos)
}
