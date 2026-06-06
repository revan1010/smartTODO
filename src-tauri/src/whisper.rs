use std::path::Path;
use std::sync::OnceLock;

use parking_lot::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

static CONTEXT: OnceLock<Mutex<Option<WhisperContext>>> = OnceLock::new();

fn ensure_loaded(model_path: &Path) -> Result<(), String> {
    let cell = CONTEXT.get_or_init(|| Mutex::new(None));
    let mut guard = cell.lock();
    if guard.is_some() {
        return Ok(());
    }
    let path_str = model_path
        .to_str()
        .ok_or_else(|| "non-utf8 model path".to_string())?;
    let ctx = WhisperContext::new_with_params(path_str, WhisperContextParameters::default())
        .map_err(|e| format!("whisper load: {e}"))?;
    *guard = Some(ctx);
    Ok(())
}

pub fn transcribe(model_path: &Path, samples: Vec<f32>) -> Result<String, String> {
    if samples.is_empty() {
        return Ok(String::new());
    }
    ensure_loaded(model_path)?;
    let cell = CONTEXT.get().ok_or("no whisper ctx".to_string())?;
    let guard = cell.lock();
    let ctx = guard.as_ref().ok_or("whisper ctx missing".to_string())?;

    let mut state = ctx.create_state().map_err(|e| format!("create_state: {e}"))?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_translate(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);
    params.set_suppress_blank(true);

    state
        .full(params, &samples)
        .map_err(|e| format!("full: {e}"))?;

    let n = state.full_n_segments().map_err(|e| e.to_string())?;
    let mut out = String::new();
    for i in 0..n {
        let seg = state
            .full_get_segment_text(i)
            .map_err(|e| format!("seg: {e}"))?;
        out.push_str(&seg);
    }
    Ok(out.trim().to_string())
}
