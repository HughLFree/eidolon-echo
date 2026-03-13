//! macOS floating-window implementation backed by NSPanel for Space/fullscreen support.

use crate::desktop_pet::window_labels::{
    BUBBLE_LABEL, CHAT_LABEL, CHILD_WINDOW_LABELS, MAIN_LABEL, MENU_LABEL,
    STARTUP_HIDDEN_WINDOW_LABELS, OVERLAY_WINDOW_LABELS,
};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::mpsc,
};

use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_nspanel::{
    tauri_panel, CollectionBehavior, ManagerExt, PanelLevel, StyleMask, WebviewWindowExt,
};
use tracing::info;

tauri_panel! {
    panel!(DesktopPetPanel {
        config: {
            can_become_key_window: true,
            can_become_main_window: true,
            is_floating_panel: true
        }
    })
}

fn configure_panel(window: &WebviewWindow, initial_level: PanelLevel) -> Result<(), String> {
    let label = window.label().to_string();
    let panel = window
        .to_panel::<DesktopPetPanel>()
        .map_err(|e| format!("failed to convert '{}' to NSPanel: {e}", window.label()))?;
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .can_join_all_spaces()
            .stationary()
            .full_screen_auxiliary()
            .into(),
    );
    panel.set_hides_on_deactivate(false);
    if label != CHAT_LABEL {
        panel.set_style_mask(StyleMask::empty().nonactivating_panel().resizable().into());
    } else {
        panel.set_becomes_key_only_if_needed(false);
    }
    panel.set_level(initial_level.value());
    info!(
        "overlay panel configured: label={}, behavior=CanJoinAllSpaces|Stationary|FullScreenAuxiliary, level={}",
        window.label(),
        initial_level.value()
    );
    Ok(())
}

fn apply_panel_level(app: &AppHandle, level: PanelLevel) -> Result<(), String> {
    for label in OVERLAY_WINDOW_LABELS {
        let panel = app
            .get_webview_panel(label)
            .map_err(|_| format!("panel '{label}' not found"))?;
        panel.set_level(level.value());
    }
    Ok(())
}

fn configure_all_panels(app: &AppHandle) -> Result<(), String> {
    for label in OVERLAY_WINDOW_LABELS {
        let window: WebviewWindow = app
            .get_webview_window(label)
            .ok_or_else(|| format!("window '{label}' not found"))?;
        configure_panel(&window, PanelLevel::MainMenu)?;
    }

    Ok(())
}

fn attach_child_panels(app: &AppHandle) -> Result<(), String> {
    let main_panel = app
        .get_webview_panel(MAIN_LABEL)
        .map_err(|_| format!("panel '{MAIN_LABEL}' not found"))?;

    for label in CHILD_WINDOW_LABELS {
        let child_panel = app
            .get_webview_panel(label)
            .map_err(|_| format!("panel '{label}' not found"))?;

        if child_panel.as_panel().parentWindow().is_none() {
            // SAFETY: NSWindow child attachment must run on the main thread; caller guarantees this.
            unsafe {
                main_panel.as_panel().addChildWindow_ordered(
                    child_panel.as_panel(),
                    tauri_nspanel::objc2_app_kit::NSWindowOrderingMode::Above,
                );
            }
        }
    }

    Ok(())
}

fn apply_visibility_inner(
    app: &AppHandle,
    show_main: bool,
    show_chat: bool,
    show_bubble: bool,
    show_menu: bool,
) -> Result<(), String> {
    let main_panel = app
        .get_webview_panel(MAIN_LABEL)
        .map_err(|_| format!("panel '{MAIN_LABEL}' not found"))?;
    let chat_panel = app
        .get_webview_panel(CHAT_LABEL)
        .map_err(|_| format!("panel '{CHAT_LABEL}' not found"))?;
    let bubble_panel = app
        .get_webview_panel(BUBBLE_LABEL)
        .map_err(|_| format!("panel '{BUBBLE_LABEL}' not found"))?;
    let menu_panel = app
        .get_webview_panel(MENU_LABEL)
        .map_err(|_| format!("panel '{MENU_LABEL}' not found"))?;

    if show_main {
        main_panel.show_and_make_key();
        main_panel.order_front_regardless();
    } else {
        main_panel.hide();
    }

    if show_chat {
        chat_panel.show();
        chat_panel.order_front_regardless();
    } else {
        chat_panel.hide();
    }

    if show_bubble {
        bubble_panel.show();
        bubble_panel.order_front_regardless();
    } else {
        bubble_panel.hide();
    }

    if show_menu {
        menu_panel.show();
        menu_panel.order_front_regardless();
    } else {
        menu_panel.hide();
    }

    Ok(())
}

fn run_on_main_thread_sync(
    app: &AppHandle,
    op: impl FnOnce(AppHandle) -> Result<(), String> + Send + 'static,
) -> Result<(), String> {
    let app_handle = app.clone();
    let (tx, rx) = mpsc::channel();
    app.run_on_main_thread(move || {
        let result = catch_unwind(AssertUnwindSafe(|| {
            match objc2::exception::catch(AssertUnwindSafe(|| op(app_handle))) {
                Ok(result) => result,
                Err(exception) => Err(format!(
                    "objective-c exception in main-thread operation: {:?}",
                    exception
                )),
            }
        }))
        .unwrap_or_else(|_| Err("panic in main-thread operation".to_string()));
        let _ = tx.send(result);
    })
    .map_err(|e| e.to_string())?;

    rx.recv()
        .map_err(|e| format!("main-thread operation canceled: {e}"))?
}

pub fn bootstrap(app: &AppHandle) -> Result<(), String> {
    app.plugin(tauri_nspanel::init())
        .map_err(|e| e.to_string())?;
    let result = run_on_main_thread_sync(app, |app_handle| {
        configure_all_panels(&app_handle)?;
        attach_child_panels(&app_handle)?;
        apply_visibility_inner(&app_handle, true, true, false, false)
    });
    if result.is_ok() {
        info!("overlay bootstrap succeeded on macOS");
    }
    result
}

pub fn prepare_startup(app: &AppHandle) -> Result<(), String> {
    for label in STARTUP_HIDDEN_WINDOW_LABELS {
        if let Some(window) = app.get_webview_window(label) {
            window.hide().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn configure_runtime(app: &AppHandle) -> Result<(), String> {
    app.set_activation_policy(tauri::ActivationPolicy::Accessory)
        .map_err(|e| e.to_string())?;

    if let Some(chat) = app.get_webview_window(CHAT_LABEL) {
        chat.set_ignore_cursor_events(false)
            .map_err(|e| e.to_string())?;
    }
    if let Some(bubble) = app.get_webview_window(BUBBLE_LABEL) {
        bubble
            .set_ignore_cursor_events(true)
            .map_err(|e| e.to_string())?;
    }
    if let Some(menu) = app.get_webview_window(MENU_LABEL) {
        menu.set_ignore_cursor_events(false)
            .map_err(|e| e.to_string())?;
        menu.hide().map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn set_overlay_always_on_top(app: &AppHandle, always_on_top: bool) -> Result<(), String> {
    let level = if always_on_top {
        PanelLevel::MainMenu
    } else {
        PanelLevel::Normal
    };

    run_on_main_thread_sync(app, move |app_handle| apply_panel_level(&app_handle, level))
}

pub fn apply_visibility(
    app: &AppHandle,
    show_main: bool,
    show_chat: bool,
    show_bubble: bool,
    show_menu: bool,
) -> Result<(), String> {
    run_on_main_thread_sync(app, move |app_handle| {
        // Child relation may be dropped by some macOS hide/order/space transitions.
        // Re-attach only when showing main/chat, and only if parent is missing.
        if show_main || show_chat {
            attach_child_panels(&app_handle)?;
        }
        apply_visibility_inner(&app_handle, show_main, show_chat, show_bubble, show_menu)
    })
}
