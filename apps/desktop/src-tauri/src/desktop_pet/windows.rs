//! Window utilities: position calculation, workspace behavior and startup window initialization.

use crate::overlay;

use super::state::{
    menu_size, AnchorRect, MenuMode, OverlayState, BUBBLE_GAP_PX, MENU_GAP_PX, SCREEN_MARGIN_PX,
};
use std::sync::Mutex;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, Position, Size, State,
    WindowEvent,
};

const CHAT_GAP_PX: i32 = 2;

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

    let x = (main_pos.x + (main_size.width as i32 - bubble_size.width as i32) / 2).max(0);
    let y = (main_pos.y - bubble_size.height as i32 - BUBBLE_GAP_PX).max(0);

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

    let x = (main_pos.x + (main_size.width as i32 - chat_size.width as i32) / 2).max(0);
    let y = (main_pos.y + main_size.height as i32 + CHAT_GAP_PX).max(0);

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

    menu.hide().map_err(|e| e.to_string())?;
    bubble.hide().map_err(|e| e.to_string())?;
    chat.hide().map_err(|e| e.to_string())?;
    main.hide().map_err(|e| e.to_string())?;

    let overlay_state: State<Mutex<OverlayState>> = app.state();
    let mut state = overlay_state.lock().unwrap_or_else(|e| e.into_inner());
    state.menu_visible = false;
    state.menu_mode = MenuMode::Buttons;

    Ok(())
}

pub fn bootstrap_desktop_pet(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    }

    if let Some(main) = app.get_webview_window("main") {
        let app_handle = app.clone();
        let _ = main.set_visible_on_all_workspaces(true);
        let _ = main.set_always_on_top(true);
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

    if let Some(chat) = app.get_webview_window("chat") {
        let _ = chat.set_visible_on_all_workspaces(true);
        let _ = chat.set_always_on_top(true);
        let _ = chat.set_ignore_cursor_events(false);
    }

    if let Some(bubble) = app.get_webview_window("bubble") {
        let _ = bubble.set_visible_on_all_workspaces(true);
        let _ = bubble.set_ignore_cursor_events(true);
        let _ = bubble.set_always_on_top(true);
    }

    if let Some(menu) = app.get_webview_window("menu") {
        let _ = menu.set_visible_on_all_workspaces(true);
        let _ = menu.set_always_on_top(true);
        let _ = menu.set_ignore_cursor_events(false);
        let _ = menu.hide();
    }

    if let Err(error) = overlay::bootstrap(app) {
        eprintln!("overlay bootstrap failed: {error}");
    }
}
