//! AI mode commands exposed to frontend windows.

use super::super::state::{AiModePayload, OverlayState};
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
pub fn get_ai_mode(state: State<Mutex<OverlayState>>) -> Result<AiModePayload, String> {
    let overlay = state.lock().unwrap_or_else(|e| e.into_inner());
    Ok(AiModePayload::from(overlay.ai_role_mode))
}
