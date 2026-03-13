//! Window utilities: position calculation, workspace behavior and startup window initialization.

use super::state::{
    menu_size, AnchorRect, MenuMode, OverlayState, WindowVisibilityState, BUBBLE_GAP_PX,
    MENU_GAP_PX, SCREEN_MARGIN_PX,
};
use crate::overlay;
use std::sync::Mutex;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Position,
    Size, State, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

const CHAT_GAP_PX: i32 = 2;
const SETTINGS_LABEL: &str = "settings";
const SETTINGS_URL: &str = "settings.html";
const SETTINGS_TITLE: &str = "桌宠设置";
const SETTINGS_WIDTH: f64 = 860.0;
const SETTINGS_HEIGHT: f64 = 620.0;
const SETTINGS_MIN_WIDTH: f64 = 720.0;
const SETTINGS_MIN_HEIGHT: f64 = 520.0;

fn logical_main_frame<R: tauri::Runtime>(
    main: &tauri::WebviewWindow<R>,
) -> Result<(LogicalPosition<f64>, LogicalSize<f64>, f64), String> {
    let scale = main.scale_factor().map_err(|e| e.to_string())?;
    let pos = main
        .outer_position()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    let size = main
        .outer_size()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);

    Ok((pos, size, scale))
}

fn monitor_logical_bounds<R: tauri::Runtime>(
    main: &tauri::WebviewWindow<R>,
    fallback_pos: LogicalPosition<f64>,
    fallback_size: LogicalSize<f64>,
) -> Result<(f64, f64, f64, f64), String> {
    let center_x = fallback_pos.x + fallback_size.width / 2.0;
    let center_y = fallback_pos.y + fallback_size.height / 2.0;

    // Prefer monitor bounds resolved by window center. `current_monitor` may lag
    // right after cross-display drags and produce initial menu misplacement.
    for monitor in main.available_monitors().map_err(|e| e.to_string())? {
        let scale = monitor.scale_factor();
        let pos = monitor.position().to_logical::<f64>(scale);
        let size = monitor.size().to_logical::<f64>(scale);
        let left = pos.x;
        let top = pos.y;
        let right = pos.x + size.width;
        let bottom = pos.y + size.height;

        if center_x >= left && center_x <= right && center_y >= top && center_y <= bottom {
            return Ok((left, top, right, bottom));
        }
    }

    if let Some(monitor) = main.current_monitor().map_err(|e| e.to_string())? {
        let scale = monitor.scale_factor();
        let pos = monitor.position().to_logical::<f64>(scale);
        let size = monitor.size().to_logical::<f64>(scale);
        return Ok((pos.x, pos.y, pos.x + size.width, pos.y + size.height));
    }

    Ok((
        fallback_pos.x,
        fallback_pos.y,
        fallback_pos.x + fallback_size.width,
        fallback_pos.y + fallback_size.height,
    ))
}

fn monitor_physical_bounds<R: tauri::Runtime>(
    main: &tauri::WebviewWindow<R>,
    fallback_pos: PhysicalPosition<i32>,
    fallback_size: PhysicalSize<u32>,
) -> Result<(i32, i32, i32, i32), String> {
    if let Some(monitor) = main.current_monitor().map_err(|e| e.to_string())? {
        let pos = monitor.position();
        let size = monitor.size();
        return Ok((
            pos.x,
            pos.y,
            pos.x + size.width as i32,
            pos.y + size.height as i32,
        ));
    }

    Ok((
        fallback_pos.x,
        fallback_pos.y,
        fallback_pos.x + fallback_size.width as i32,
        fallback_pos.y + fallback_size.height as i32,
    ))
}

fn compute_menu_position<R: tauri::Runtime>(
    main: &tauri::WebviewWindow<R>,
    anchor: AnchorRect,
    mode: MenuMode,
) -> Result<(f64, f64), String> {
    let (menu_w, menu_h) = menu_size(mode);
    let (main_pos, main_size, _scale) = logical_main_frame(main)?;

    let avatar_left = main_pos.x + anchor.x;
    let avatar_top = main_pos.y + anchor.y;
    let avatar_right = avatar_left + anchor.width;
    let avatar_center_x = avatar_left + anchor.width / 2.0;

    let (screen_left, screen_top, screen_right, screen_bottom) =
        monitor_logical_bounds(main, main_pos, main_size)?;
    let screen_mid_x = (screen_left + screen_right) / 2.0;

    let prefer_left = avatar_center_x > screen_mid_x;

    let mut x = if prefer_left {
        avatar_left - menu_w - MENU_GAP_PX
    } else {
        avatar_right + MENU_GAP_PX
    };

    let mut y = avatar_top + (anchor.height - menu_h) / 2.0;

    let max_x = (screen_right - menu_w - SCREEN_MARGIN_PX).max(screen_left + SCREEN_MARGIN_PX);
    let max_y = (screen_bottom - menu_h - SCREEN_MARGIN_PX).max(screen_top + SCREEN_MARGIN_PX);

    x = x.clamp(screen_left + SCREEN_MARGIN_PX, max_x);
    y = y.clamp(screen_top + SCREEN_MARGIN_PX, max_y);

    Ok((x, y))
}

pub fn sync_bubble_position(app: &AppHandle) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    let bubble = app
        .get_webview_window("bubble")
        .ok_or_else(|| "bubble window not found".to_string())?;

    let main_pos = main.outer_position().map_err(|e| e.to_string())?;
    let main_size = main.outer_size().map_err(|e| e.to_string())?;
    let bubble_size = bubble.outer_size().map_err(|e| e.to_string())?;

    let (screen_left, screen_top, screen_right, screen_bottom) =
        monitor_physical_bounds(&main, main_pos, main_size)?;
    let max_x = screen_right - bubble_size.width as i32;
    let max_y = screen_bottom - bubble_size.height as i32;

    let x = (main_pos.x + (main_size.width as i32 - bubble_size.width as i32) / 2)
        .clamp(screen_left, max_x);
    let y = (main_pos.y - bubble_size.height as i32 - BUBBLE_GAP_PX).clamp(screen_top, max_y);

    bubble
        .set_position(Position::Physical(PhysicalPosition::new(x, y)))
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn sync_chat_position(app: &AppHandle) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    let chat = app
        .get_webview_window("chat")
        .ok_or_else(|| "chat window not found".to_string())?;

    let main_pos = main.outer_position().map_err(|e| e.to_string())?;
    let main_size = main.outer_size().map_err(|e| e.to_string())?;
    let chat_size = chat.outer_size().map_err(|e| e.to_string())?;

    let (screen_left, screen_top, screen_right, screen_bottom) =
        monitor_physical_bounds(&main, main_pos, main_size)?;
    let max_x = screen_right - chat_size.width as i32;
    let max_y = screen_bottom - chat_size.height as i32;

    let x = (main_pos.x + (main_size.width as i32 - chat_size.width as i32) / 2)
        .clamp(screen_left, max_x);
    let y = (main_pos.y + main_size.height as i32 + CHAT_GAP_PX).clamp(screen_top, max_y);

    chat.set_position(Position::Physical(PhysicalPosition::new(x, y)))
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn sync_menu_position(app: &AppHandle) -> Result<(), String> {
    let overlay_state: State<Mutex<OverlayState>> = app.state();
    let (visible, mode, anchor) = {
        let state = overlay_state.lock().unwrap_or_else(|e| e.into_inner());
        (state.menu_visible, state.menu_mode, state.anchor)
    };

    if !visible {
        return Ok(());
    }

    let anchor = anchor.ok_or_else(|| "menu anchor is missing".to_string())?;
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    let menu = app
        .get_webview_window("menu")
        .ok_or_else(|| "menu window not found".to_string())?;

    let (w, h) = menu_size(mode);
    menu.set_size(Size::Logical(LogicalSize::new(w, h)))
        .map_err(|e| e.to_string())?;

    let (x, y) = compute_menu_position(&main, anchor, mode)?;
    menu.set_position(Position::Logical(LogicalPosition::new(x, y)))
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn hide_pet_windows(app: &AppHandle) -> Result<(), String> {
    overlay::apply_visibility(app, false, false, false, false)?;

    let overlay_state: State<Mutex<OverlayState>> = app.state();
    let mut state = overlay_state.lock().unwrap_or_else(|e| e.into_inner());
    state.menu_visible = false;
    state.menu_mode = MenuMode::Buttons;

    Ok(())
}

pub fn toggle_pet_windows(app: &AppHandle) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    let bubble = app
        .get_webview_window("bubble")
        .ok_or_else(|| "bubble window not found".to_string())?;
    let chat = app
        .get_webview_window("chat")
        .ok_or_else(|| "chat window not found".to_string())?;
    let menu = app
        .get_webview_window("menu")
        .ok_or_else(|| "menu window not found".to_string())?;

    let current_visibility = WindowVisibilityState {
        main: main.is_visible().map_err(|e| e.to_string())?,
        chat: chat.is_visible().map_err(|e| e.to_string())?,
        bubble: bubble.is_visible().map_err(|e| e.to_string())?,
        menu: menu.is_visible().map_err(|e| e.to_string())?,
    };

    let overlay_state: State<Mutex<OverlayState>> = app.state();
    if current_visibility.any_visible() {
        {
            let mut state = overlay_state.lock().unwrap_or_else(|e| e.into_inner());
            state.tray_restore_visibility = Some(current_visibility);
            state.menu_visible = false;
        }
        overlay::apply_visibility(app, false, false, false, false)?;
        return Ok(());
    }

    let restore_visibility = {
        let mut state = overlay_state.lock().unwrap_or_else(|e| e.into_inner());
        state.tray_restore_visibility.take()
    };

    let mut restore_visibility = restore_visibility.unwrap_or(WindowVisibilityState {
        main: true,
        chat: true,
        bubble: false,
        menu: false,
    });
    restore_visibility.menu = false;

    overlay::apply_visibility(
        app,
        restore_visibility.main,
        restore_visibility.chat,
        restore_visibility.bubble,
        false,
    )?;

    {
        let mut state = overlay_state.lock().unwrap_or_else(|e| e.into_inner());
        state.menu_visible = restore_visibility.menu;
    }

    let _ = sync_bubble_position(app);
    let _ = sync_chat_position(app);
    let _ = sync_menu_position(app);

    Ok(())
}

pub fn bootstrap_desktop_pet(app: &AppHandle) {
    if let Err(error) = overlay::configure_runtime(app) {
        eprintln!("overlay runtime configuration failed: {error}");
    }

    if let Some(main) = app.get_webview_window("main") {
        let app_handle = app.clone();
        let _ = sync_bubble_position(&app_handle);
        let _ = sync_chat_position(&app_handle);
        let _ = sync_menu_position(&app_handle);

        main.on_window_event(move |event| {
            if matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_)) {
                let _ = sync_bubble_position(&app_handle);
                let _ = sync_chat_position(&app_handle);
                let _ = sync_menu_position(&app_handle);
            }
        });
    }

}

pub fn open_settings_window(app: &AppHandle) -> Result<(), String> {
    let settings = ensure_settings_window(app)?;

    if !settings.is_visible().map_err(|e| e.to_string())? {
        settings.show().map_err(|e| e.to_string())?;
    }
    settings.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

fn ensure_settings_window(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    if let Some(settings) = app.get_webview_window(SETTINGS_LABEL) {
        return Ok(settings);
    }

    WebviewWindowBuilder::new(app, SETTINGS_LABEL, WebviewUrl::App(SETTINGS_URL.into()))
        .title(SETTINGS_TITLE)
        .inner_size(SETTINGS_WIDTH, SETTINGS_HEIGHT)
        .min_inner_size(SETTINGS_MIN_WIDTH, SETTINGS_MIN_HEIGHT)
        .resizable(true)
        .always_on_top(false)
        .decorations(true)
        .transparent(false)
        .skip_taskbar(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())
}
