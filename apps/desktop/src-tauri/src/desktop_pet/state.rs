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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiRoleMode {
    Default,
    Roleplay,
}

impl Default for AiRoleMode {
    fn default() -> Self {
        Self::Default
    }
}

impl AiRoleMode {
    pub fn avatar_png(self) -> &'static str {
        match self {
            Self::Default => "/assets/pet/ava.png",
            Self::Roleplay => "/assets/pet/av3a.png",
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WindowVisibilityState {
    pub main: bool,
    pub chat: bool,
    pub bubble: bool,
    pub menu: bool,
}

impl WindowVisibilityState {
    pub fn any_visible(self) -> bool {
        self.main || self.chat || self.bubble || self.menu
    }
}

#[derive(Debug)]
pub struct OverlayState {
    pub anchor: Option<AnchorRect>,
    pub menu_visible: bool,
    pub menu_mode: MenuMode,
    pub tray_restore_visibility: Option<WindowVisibilityState>,
    pub ai_role_mode: AiRoleMode,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            anchor: None,
            menu_visible: false,
            menu_mode: MenuMode::Buttons,
            tray_restore_visibility: None,
            ai_role_mode: AiRoleMode::Default,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModePayload {
    pub mode: AiRoleMode,
    pub avatar_png: String,
}

impl From<AiRoleMode> for AiModePayload {
    fn from(mode: AiRoleMode) -> Self {
        Self {
            mode,
            avatar_png: mode.avatar_png().to_string(),
        }
    }
}

pub fn menu_size(mode: MenuMode) -> (f64, f64) {
    match mode {
        MenuMode::Buttons => (MENU_WIDTH, MENU_HEIGHT),
        MenuMode::History => (HISTORY_WIDTH, HISTORY_HEIGHT),
    }
}
