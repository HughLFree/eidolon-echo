//! Shared window labels for the desktop shell.

pub const MAIN_LABEL: &str = "main";
pub const CHAT_LABEL: &str = "chat";
pub const BUBBLE_LABEL: &str = "bubble";
pub const MENU_LABEL: &str = "menu";
pub const SETTINGS_LABEL: &str = "settings";

pub const OVERLAY_WINDOW_LABELS: &[&str] =
    &[MAIN_LABEL, CHAT_LABEL, BUBBLE_LABEL, MENU_LABEL];
pub const STARTUP_HIDDEN_WINDOW_LABELS: &[&str] = &[MAIN_LABEL, CHAT_LABEL, BUBBLE_LABEL];
pub const CHILD_WINDOW_LABELS: &[&str] = &[CHAT_LABEL];
