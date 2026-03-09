//! AI mode commands exposed to frontend windows.

use super::super::state::{AiModePayload, AiRoleMode, OverlayState};
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
pub fn get_ai_mode(state: State<Mutex<OverlayState>>) -> Result<AiModePayload, String> {
    let overlay = state.lock().unwrap_or_else(|e| e.into_inner());
    Ok(AiModePayload::from(overlay.ai_role_mode))
}

#[tauri::command]
pub fn get_active_session_id(
    state: State<Mutex<OverlayState>>,
    mode: AiRoleMode,
) -> Result<Option<i64>, String> {
    let overlay = state.lock().unwrap_or_else(|e| e.into_inner());
    Ok(overlay.active_session_id(mode))
}

#[tauri::command]
pub fn set_active_session_id(
    state: State<Mutex<OverlayState>>,
    mode: AiRoleMode,
    session_id: Option<i64>,
) -> Result<(), String> {
    let mut overlay = state.lock().unwrap_or_else(|e| e.into_inner());
    overlay.set_active_session_id(mode, session_id);
    Ok(())
}
