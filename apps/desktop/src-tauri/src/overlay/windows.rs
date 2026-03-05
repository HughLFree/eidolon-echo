//! Windows overlay implementation using regular Tauri dialog window controls.

use tauri::{AppHandle, Manager, PhysicalPosition, Position};

const DIALOG_LABEL: &str = "dialog";
const DIALOG_OFFSET_Y_PX: i32 = 18;

pub fn bootstrap(app: &AppHandle) -> Result<(), String> {
    let dialog = app
        .get_webview_window(DIALOG_LABEL)
        .ok_or_else(|| "dialog window not found".to_string())?;
    dialog.hide().map_err(|e| e.to_string())?;
    dialog.set_always_on_top(true).map_err(|e| e.to_string())
}

pub fn show_chat_panel(
    app: &AppHandle,
    tray_pos: Option<PhysicalPosition<f64>>,
) -> Result<(), String> {
    let dialog = app
        .get_webview_window(DIALOG_LABEL)
        .ok_or_else(|| "dialog window not found".to_string())?;

    if let Some(pos) = tray_pos {
        let size = dialog.outer_size().map_err(|e| e.to_string())?;
        let x = ((pos.x as i32) - (size.width as i32 / 2)).max(0);
        let y = ((pos.y as i32) + DIALOG_OFFSET_Y_PX).max(0);
        dialog
            .set_position(Position::Physical(PhysicalPosition::new(x, y)))
            .map_err(|e| e.to_string())?;
    }

    dialog.show().map_err(|e| e.to_string())
}

pub fn hide_chat_panel(app: &AppHandle) -> Result<(), String> {
    let dialog = app
        .get_webview_window(DIALOG_LABEL)
        .ok_or_else(|| "dialog window not found".to_string())?;
    dialog.hide().map_err(|e| e.to_string())
}

pub fn is_chat_panel_visible(app: &AppHandle) -> Result<bool, String> {
    let dialog = app
        .get_webview_window(DIALOG_LABEL)
        .ok_or_else(|| "dialog window not found".to_string())?;
    dialog.is_visible().map_err(|e| e.to_string())
}

pub fn toggle_chat_panel(
    app: &AppHandle,
    tray_pos: Option<PhysicalPosition<f64>>,
) -> Result<bool, String> {
    if is_chat_panel_visible(app)? {
        hide_chat_panel(app)?;
        return Ok(false);
    }

    show_chat_panel(app, tray_pos)?;
    Ok(true)
}
