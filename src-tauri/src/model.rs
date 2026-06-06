use std::path::PathBuf;

use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

const MODEL_FILE: &str = "ggml-base.en.bin";
const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin";

#[derive(Serialize, Clone)]
pub struct ModelStatus {
    pub ready: bool,
    pub path: String,
    pub size_bytes: Option<u64>,
}

#[derive(Serialize, Clone)]
struct DownloadProgress {
    downloaded: u64,
    total: u64,
}

pub fn model_path<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no app data dir: {e}"))?;
    Ok(base.join("models").join(MODEL_FILE))
}

#[tauri::command]
pub fn model_status<R: tauri::Runtime>(app: AppHandle<R>) -> Result<ModelStatus, String> {
    let path = model_path(&app)?;
    let size = std::fs::metadata(&path).ok().map(|m| m.len());
    Ok(ModelStatus {
        ready: size.map(|s| s > 1_000_000).unwrap_or(false),
        path: path.to_string_lossy().into_owned(),
        size_bytes: size,
    })
}

#[tauri::command]
pub async fn download_model<R: tauri::Runtime>(app: AppHandle<R>) -> Result<String, String> {
    let dest = model_path(&app)?;
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("mkdir: {e}"))?;
    }

    let client = reqwest::Client::builder()
        .user_agent("smartTODO/0.1")
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(MODEL_URL)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("bad status: {e}"))?;

    let total = resp.content_length().unwrap_or(0);
    let tmp = dest.with_extension("bin.part");
    let mut file = tokio::fs::File::create(&tmp)
        .await
        .map_err(|e| format!("create tmp: {e}"))?;

    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_emit: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("chunk: {e}"))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("write: {e}"))?;
        downloaded += chunk.len() as u64;

        if downloaded - last_emit > 256 * 1024 || downloaded == total {
            last_emit = downloaded;
            let _ = app.emit(
                "model-download-progress",
                DownloadProgress { downloaded, total },
            );
        }
    }

    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);
    tokio::fs::rename(&tmp, &dest)
        .await
        .map_err(|e| format!("rename: {e}"))?;

    Ok(dest.to_string_lossy().into_owned())
}
