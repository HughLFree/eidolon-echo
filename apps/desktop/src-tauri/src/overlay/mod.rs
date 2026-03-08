//! Platform dispatch layer for desktop pet floating window behavior.

use tauri::AppHandle;

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

pub fn prepare_startup(app: &AppHandle) -> Result<(), String> {
    platform::prepare_startup(app)
}

pub fn configure_runtime(app: &AppHandle) -> Result<(), String> {
    platform::configure_runtime(app)
}

pub fn set_overlay_always_on_top(app: &AppHandle, always_on_top: bool) -> Result<(), String> {
    platform::set_overlay_always_on_top(app, always_on_top)
}

pub fn apply_visibility(
    app: &AppHandle,
    show_main: bool,
    show_chat: bool,
    show_bubble: bool,
    show_menu: bool,
) -> Result<(), String> {
    platform::apply_visibility(app, show_main, show_chat, show_bubble, show_menu)
}
