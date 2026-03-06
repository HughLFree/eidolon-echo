//! macOS floating-window implementation backed by NSPanel for Space/fullscreen support.

use std::sync::mpsc;

use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_nspanel::{
    CollectionBehavior, ManagerExt, PanelLevel, StyleMask, WebviewWindowExt, tauri_panel,
};

tauri_panel! {
    panel!(DesktopPetPanel {
        config: {
            can_become_key_window: true,
            can_become_main_window: true,
            is_floating_panel: true
        }
    })
}

const PANEL_WINDOW_LABELS: &[&str] = &["main", "chat", "bubble", "menu"];
const CHILD_WINDOW_LABELS: &[&str] = &["chat", "bubble", "menu"];

fn configure_panel(window: &WebviewWindow, initial_level: PanelLevel) -> Result<(), String> {
    let label = window.label().to_string();
    let panel = window.to_panel::<DesktopPetPanel>().map_err(|e| {
        format!(
            "failed to convert '{}' to NSPanel: {e}",
            window.label()
        )
    })?;
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .can_join_all_spaces()
            .stationary()
            .full_screen_auxiliary()
            .into(),
    );
    if label != "chat" {
        panel.set_style_mask(StyleMask::empty().nonactivating_panel().resizable().into());
    } else {
        panel.set_becomes_key_only_if_needed(false);
        panel.set_hides_on_deactivate(false);
    }
    panel.set_level(initial_level.value());
    eprintln!(
        "overlay panel configured: label={}, behavior=CanJoinAllSpaces|Stationary|FullScreenAuxiliary, level={}",
        window.label(),
        initial_level.value()
    );
    Ok(())
}

fn apply_panel_level(app: &AppHandle, level: PanelLevel) -> Result<(), String> {
    for label in PANEL_WINDOW_LABELS {
        let panel = app
            .get_webview_panel(label)
            .map_err(|_| format!("panel '{label}' not found"))?;
        panel.set_level(level.value());
    }
    Ok(())
}

fn configure_all_panels(app: &AppHandle) -> Result<(), String> {
    for label in PANEL_WINDOW_LABELS {
        if let Ok(panel) = app.get_webview_panel(label) {
            panel.set_collection_behavior(
                CollectionBehavior::new()
                    .can_join_all_spaces()
                    .stationary()
                    .full_screen_auxiliary()
                    .into(),
            );
            if *label != "chat" {
                panel.set_style_mask(StyleMask::empty().nonactivating_panel().resizable().into());
            } else {
                panel.set_becomes_key_only_if_needed(false);
                panel.set_hides_on_deactivate(false);
            }
            panel.set_level(PanelLevel::MainMenu.value());
            eprintln!(
                "overlay panel reconfigured: label={}, behavior=CanJoinAllSpaces|Stationary|FullScreenAuxiliary, level={}",
                label,
                PanelLevel::MainMenu.value()
            );
            continue;
        }

        let window: WebviewWindow = app
            .get_webview_window(label)
            .ok_or_else(|| format!("window '{label}' not found"))?;
        configure_panel(&window, PanelLevel::MainMenu)?;
    }

    Ok(())
}

fn attach_child_panels(app: &AppHandle) -> Result<(), String> {
    let main_panel = app
        .get_webview_panel("main")
        .map_err(|_| "panel 'main' not found".to_string())?;

    for label in CHILD_WINDOW_LABELS {
        let child_panel = app
            .get_webview_panel(label)
            .map_err(|_| format!("panel '{label}' not found"))?;

        if *label == "menu" {
            child_panel.hide();
        }

        // SAFETY: NSWindow child attachment must run on the main thread; caller guarantees this.
        unsafe {
            main_panel
                .as_panel()
                .addChildWindow_ordered(
                    child_panel.as_panel(),
                    tauri_nspanel::objc2_app_kit::NSWindowOrderingMode::Above,
                );
        }

        if *label == "menu" {
            child_panel.hide();
        }

        eprintln!("overlay child attached: parent=main, child={label}");
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
        let result = op(app_handle);
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
        attach_child_panels(&app_handle)
    });
    if result.is_ok() {
        eprintln!("overlay bootstrap succeeded on macOS");
    }
    result
}

pub fn set_overlay_always_on_top(app: &AppHandle, always_on_top: bool) -> Result<(), String> {
    let level = if always_on_top {
        PanelLevel::MainMenu
    } else {
        PanelLevel::Normal
    };

    run_on_main_thread_sync(app, move |app_handle| apply_panel_level(&app_handle, level))
}
