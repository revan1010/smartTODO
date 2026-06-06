#[cfg(target_os = "macos")]
use tauri::{Manager, Runtime, WebviewWindow};
#[cfg(target_os = "macos")]
use tauri_nspanel::{
    tauri_panel, CollectionBehavior, ManagerExt, PanelHandle, PanelLevel, StyleMask,
    WebviewWindowExt as WebviewPanelExt,
};

#[cfg(target_os = "macos")]
use crate::PANEL_LABEL;

#[cfg(target_os = "macos")]
tauri_panel! {
    panel!(SmartTodoPanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true,
        }
    })

    panel_event!(SmartTodoPanelEventHandler {
        window_did_become_key(notification: &NSNotification) -> (),
        window_did_resign_key(notification: &NSNotification) -> (),
    })
}

#[cfg(target_os = "macos")]
pub trait WebviewWindowExt<R: Runtime> {
    fn to_spotlight_panel(&self) -> tauri::Result<PanelHandle<R>>;
    fn center_at_cursor_monitor(&self) -> tauri::Result<()>;
}

#[cfg(target_os = "macos")]
impl<R: Runtime> WebviewWindowExt<R> for WebviewWindow<R> {
    fn to_spotlight_panel(&self) -> tauri::Result<PanelHandle<R>> {
        let panel = self
            .to_panel::<SmartTodoPanel<R>>()
            .map_err(|_| tauri::Error::Anyhow(anyhow::anyhow!("failed to convert to panel")))?;

        // Floating level — above normal windows
        panel.set_level(PanelLevel::Floating.value());

        // Appear on fullscreen spaces + follow active space
        panel.set_collection_behavior(
            CollectionBehavior::new()
                .full_screen_auxiliary()
                .move_to_active_space()
                .value(),
        );

        // Non-activating: don't steal app activation
        panel.set_style_mask(StyleMask::empty().nonactivating_panel().into());

        // Event handler: hide panel when it loses key window status
        let handler = SmartTodoPanelEventHandler::new();

        handler.window_did_become_key(|_| {
            println!("[smartTODO] panel became key");
        });

        let app_handle = self.app_handle().clone();
        handler.window_did_resign_key(move |_| {
            println!("[smartTODO] panel resigned key");
            // Only hide if NOT recording
            let is_recording = app_handle
                .state::<crate::commands::RecorderState>()
                .0
                .lock()
                .is_some();
            if !is_recording {
                if let Ok(p) = app_handle.get_webview_panel(PANEL_LABEL) {
                    if p.is_visible() {
                        p.hide();
                    }
                }
            }
        });

        panel.set_event_handler(Some(handler.as_ref()));

        println!("[smartTODO] panel created with fullscreen support");

        Ok(panel)
    }

    fn center_at_cursor_monitor(&self) -> tauri::Result<()> {
        let mon = monitor::get_monitor_with_cursor()
            .ok_or_else(|| tauri::Error::Anyhow(anyhow::anyhow!("no monitor with cursor")))?;

        let scale = mon.scale_factor();
        let size = mon.size().to_logical::<f64>(scale);
        let pos = mon.position().to_logical::<f64>(scale);

        let panel = self
            .get_webview_panel(self.label())
            .map_err(|_| tauri::Error::Anyhow(anyhow::anyhow!("panel not found")))?;

        let p = panel.as_panel();
        let frame = p.frame();

        use tauri_nspanel::objc2_foundation::{NSPoint, NSRect};
        let rect = NSRect {
            origin: NSPoint {
                x: (pos.x + (size.width / 2.0)) - (frame.size.width / 2.0),
                y: (pos.y + (size.height / 2.0)) - (frame.size.height / 2.0),
            },
            size: frame.size,
        };

        p.setFrame_display(rect, true);

        Ok(())
    }
}

// No-op for non-macOS
#[cfg(not(target_os = "macos"))]
pub trait WebviewWindowExt<R: tauri::Runtime> {}

#[cfg(not(target_os = "macos"))]
impl<R: tauri::Runtime> WebviewWindowExt<R> for tauri::WebviewWindow<R> {}
