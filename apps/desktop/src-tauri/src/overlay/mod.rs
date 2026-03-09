//! Platform dispatch layer for desktop pet floating window behavior.
//!
//! 本模块是 overlay 能力的跨平台统一入口。
//! 通过编译期目标分发到具体实现：
//! - macOS -> `macos`
//! - Windows -> `windows`
//! - 其他平台 -> `fallback`

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

/// 在桌宠启动主流程前执行，用于准备窗口初始状态。
/// macOS 下会先隐藏启动窗口，再由 NSPanel 接管显示。
pub fn prepare_startup(app: &AppHandle) -> Result<(), String> {
    platform::prepare_startup(app)
}

/// 应用平台相关的运行期配置（激活策略、置顶行为等）。
pub fn configure_runtime(app: &AppHandle) -> Result<(), String> {
    platform::configure_runtime(app)
}

/// 统一切换所有 overlay 窗口的层级（是否保持在最上层）。
pub fn set_overlay_always_on_top(app: &AppHandle, always_on_top: bool) -> Result<(), String> {
    platform::set_overlay_always_on_top(app, always_on_top)
}

/// 统一的可见性接口，业务层无需再写平台分支。
/// 参数依次对应 `main/chat/bubble/menu` 的显示状态。
pub fn apply_visibility(
    app: &AppHandle,
    show_main: bool,
    show_chat: bool,
    show_bubble: bool,
    show_menu: bool,
) -> Result<(), String> {
    platform::apply_visibility(app, show_main, show_chat, show_bubble, show_menu)
}
