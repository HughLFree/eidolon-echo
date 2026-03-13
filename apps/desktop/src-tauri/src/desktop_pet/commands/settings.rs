//! Settings panel commands: open settings window.

use super::super::{backend, window_manager::open_settings_window};
use tauri::{AppHandle, Runtime};

fn open_settings_panel_impl<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    open_settings_window(app)
}

#[tauri::command]
pub fn open_settings_panel(app: AppHandle) -> Result<(), String> {
    open_settings_panel_impl(&app)
}

#[tauri::command]
pub fn clear_local_data(app: AppHandle) -> Result<backend::ClearLocalDataResult, String> {
    backend::clear_backend_local_data(&app)
}

#[cfg(test)]
mod tests {
    use super::open_settings_panel_impl;
    use crate::desktop_pet::window_labels::SETTINGS_LABEL;
    use tauri::{
        test::{mock_builder, mock_context, noop_assets},
        Manager,
    };

    #[test]
    fn open_settings_panel_creates_and_shows_settings_window() {
        let app = mock_builder()
            .build(mock_context(noop_assets()))
            .expect("build mock tauri app");

        open_settings_panel_impl(app.handle()).expect("open settings panel");

        let settings = app
            .get_webview_window(SETTINGS_LABEL)
            .expect("settings window should exist");
        assert!(
            settings.is_visible().expect("query settings visibility"),
            "settings window should be visible after open_settings_panel"
        );
    }
}
