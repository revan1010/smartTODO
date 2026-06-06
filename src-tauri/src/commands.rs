use std::sync::Arc;

use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use crate::audio::Recorder;
use crate::model::model_path;
use crate::whisper;

#[derive(Default)]
pub struct RecorderState(pub Arc<Mutex<Option<Recorder>>>);

#[tauri::command]
pub fn capture_input(text: String) {
    println!("capture_input: {}", text);
}

#[tauri::command]
pub fn hide_panel_cmd(app: AppHandle) {
    crate::hide_panel(&app);
}

#[tauri::command]
pub fn start_recording(state: State<'_, RecorderState>) -> Result<(), String> {
    let mut guard = state.0.lock();
    if guard.is_some() {
        return Ok(());
    }
    let rec = Recorder::start()?;
    *guard = Some(rec);
    Ok(())
}

#[tauri::command]
pub async fn stop_recording_and_transcribe<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RecorderState>,
) -> Result<String, String> {
    let rec = state.0.lock().take();
    let Some(rec) = rec else {
        return Ok(String::new());
    };
    let samples = rec.stop_and_take();
    let _ = app.emit("transcribing", true);

    let path = model_path(&app)?;
    let text =
        tokio::task::spawn_blocking(move || whisper::transcribe(&path, samples))
            .await
            .map_err(|e| format!("join: {e}"))??;

    let _ = app.emit("transcribing", false);
    Ok(text)
}

#[tauri::command]
pub fn cancel_recording(state: State<'_, RecorderState>) {
    if let Some(rec) = state.0.lock().take() {
        let _ = rec.stop_and_take();
    }
}
