//! Desktop pet state models and layout constants shared across command/window modules.

use serde::{Deserialize, Serialize};

pub const BUBBLE_GAP_PX: i32 = 10;
pub const MENU_GAP_PX: f64 = 8.0;
pub const SCREEN_MARGIN_PX: f64 = 8.0;

pub const MENU_WIDTH: f64 = 76.0;
pub const MENU_HEIGHT: f64 = 140.0;
pub const HISTORY_WIDTH: f64 = 340.0;
pub const HISTORY_HEIGHT: f64 = 300.0;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuMode {
    Buttons,
    History,
}

impl Default for MenuMode {
    fn default() -> Self {
        Self::Buttons
    }
}

#[derive(Debug, Default)]
pub struct OverlayState {
    pub anchor: Option<AnchorRect>,
    pub session_id: Option<i64>,
    pub menu_visible: bool,
    pub menu_mode: MenuMode,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuShowPayload {
    pub session_id: Option<i64>,
}

pub fn menu_size(mode: MenuMode) -> (f64, f64) {
    match mode {
        MenuMode::Buttons => (MENU_WIDTH, MENU_HEIGHT),
        MenuMode::History => (HISTORY_WIDTH, HISTORY_HEIGHT),
    }
}
