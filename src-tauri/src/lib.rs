mod audio;
mod commands;
mod model;
mod panel;
mod whisper;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::commands::RecorderState;
use crate::panel::WebviewWindowExt;

pub const PANEL_LABEL: &str = "main";

fn show_panel(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(PANEL_LABEL) else {
        return;
    };

    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::ManagerExt;

        match app
            .get_webview_panel(PANEL_LABEL)
            .or_else(|_| window.to_spotlight_panel())
        {
            Ok(p) => {
                let _ = window.center_at_cursor_monitor();
                p.show_and_make_key();
            }
            Err(e) => eprintln!("panel error: {:?}", e),
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn hide_panel(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::ManagerExt;
        if let Ok(p) = app.get_webview_panel(PANEL_LABEL) {
            if p.is_visible() {
                p.hide();
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        if let Some(win) = app.get_webview_window(PANEL_LABEL) {
            let _ = win.hide();
        }
    }
}

fn toggle_panel(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::ManagerExt;
        let Some(window) = app.get_webview_window(PANEL_LABEL) else {
            return;
        };
        match app
            .get_webview_panel(PANEL_LABEL)
            .or_else(|_| window.to_spotlight_panel())
        {
            Ok(p) => {
                if p.is_visible() {
                    p.hide();
                } else {
                    let _ = window.center_at_cursor_monitor();
                    p.show_and_make_key();
                }
            }
            Err(e) => eprintln!("panel error: {:?}", e),
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let Some(win) = app.get_webview_window(PANEL_LABEL) else {
            return;
        };
        match win.is_visible() {
            Ok(true) => { let _ = win.hide(); }
            _ => {
                let _ = win.center();
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // Register NSPanel plugin on macOS
    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    #[cfg(desktop)]
    {
        let toggle_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::KeyX);
        let is_holding = Arc::new(AtomicBool::new(false));

        builder = builder.plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if shortcut != &toggle_shortcut {
                        return;
                    }
                    match event.state() {
                        ShortcutState::Pressed => {
                            if is_holding.swap(true, Ordering::SeqCst) {
                                return;
                            }

                            // Start recording immediately
                            let state = app.state::<RecorderState>();
                            let mut guard = state.0.lock();
                            if guard.is_none() {
                                match crate::audio::Recorder::start() {
                                    Ok(rec) => *guard = Some(rec),
                                    Err(e) => {
                                        eprintln!("start_recording: {e}");
                                        let _ = app.emit("recording-error", e);
                                    }
                                }
                            }
                            drop(guard);

                            show_panel(app);
                            let _ = app.emit("recording-started", ());
                        }
                        ShortcutState::Released => {
                            if !is_holding.swap(false, Ordering::SeqCst) {
                                return;
                            }

                            let state = app.state::<RecorderState>();
                            let rec = state.0.lock().take();
                            if let Some(rec) = rec {
                                let _ = app.emit("transcribing-started", ());
                                let app2 = app.clone();
                                std::thread::spawn(move || {
                                    let samples = rec.stop_and_take();
                                    let path = match crate::model::model_path(&app2) {
                                        Ok(p) => p,
                                        Err(e) => {
                                            eprintln!("model_path: {e}");
                                            let _ = app2.emit("transcription-done", "");
                                            return;
                                        }
                                    };
                                    match crate::whisper::transcribe(&path, samples) {
                                        Ok(text) => {
                                            println!("transcribed: {text}");
                                            let _ = app2.emit("transcription-done", text);
                                        }
                                        Err(e) => {
                                            eprintln!("transcribe error: {e}");
                                            let _ = app2.emit("transcription-done", "");
                                        }
                                    }
                                });
                            }
                        }
                    }
                })
                .build(),
        );
    }

    builder
        .manage(RecorderState::default())
        .invoke_handler(tauri::generate_handler![
            commands::capture_input,
            commands::hide_panel_cmd,
            commands::start_recording,
            commands::stop_recording_and_transcribe,
            commands::cancel_recording,
            model::model_status,
            model::download_model,
        ])
        .setup(|app| {
            // Prohibited: no dock icon, no focus stealing
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Prohibited);

            #[cfg(desktop)]
            {
                let toggle_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::KeyX);
                app.global_shortcut().register(toggle_shortcut)?;
            }

            let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let settings_i =
                MenuItem::with_id(app, "settings", "Settings…", false, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit smartTODO", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &settings_i, &quit_i])?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .tooltip("smartTODO — hold Option+X")
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => toggle_panel(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_panel(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
