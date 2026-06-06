import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

type AppMode = "checking" | "needs_model" | "downloading" | "idle" | "recording" | "transcribing";

interface DownloadProgress {
  downloaded: number;
  total: number;
}

export default function App() {
  const [mode, setMode] = useState<AppMode>("checking");
  const [text, setText] = useState("");
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    invoke("model_status").then((status: any) => {
      setMode(status.ready ? "idle" : "needs_model");
    });
  }, []);

  useEffect(() => {
    const unlisteners: Promise<() => void>[] = [];

    // Rust starts recording on hotkey press — just update UI
    unlisteners.push(
      listen("recording-started", () => {
        setMode("recording");
      })
    );

    // Rust stopped recording, now transcribing
    unlisteners.push(
      listen("transcribing-started", () => {
        setMode("transcribing");
      })
    );

    // Transcript ready from Rust
    unlisteners.push(
      listen<string>("transcription-done", (ev) => {
        const transcript = ev.payload;
        if (transcript) {
          setText(transcript);
          setMode("idle");
          setTimeout(() => inputRef.current?.focus(), 50);
        } else {
          setMode("idle");
        }
      })
    );

    unlisteners.push(
      listen("recording-error", () => {
        setMode("idle");
      })
    );

    unlisteners.push(
      listen("model-download-progress", (ev) => {
        setProgress(ev.payload as DownloadProgress);
      })
    );

    const win = getCurrentWindow();
    unlisteners.push(
      win.onFocusChanged(({ payload: focused }) => {
        if (focused) {
          inputRef.current?.focus();
        }
      })
    );

    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()));
    };
  }, []);

  function onKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    // Typing during recording: cancel voice, switch to text
    if (mode === "recording" && e.key !== "Escape") {
      invoke("cancel_recording");
      setMode("idle");
      return;
    }

    if (e.key === "Enter") {
      e.preventDefault();
      const value = text.trim();
      if (!value) return;
      invoke("capture_input", { text: value });
      setText("");
      invoke("hide_panel_cmd");
    } else if (e.key === "Escape") {
      e.preventDefault();
      if (mode === "recording") {
        invoke("cancel_recording");
      }
      setMode("idle");
      setText("");
      invoke("hide_panel_cmd");
    }
  }

  async function handleDownload() {
    setMode("downloading");
    setProgress(null);
    try {
      await invoke("download_model");
      setMode("idle");
    } catch (e) {
      console.error("download failed:", e);
      setMode("needs_model");
    }
  }

  if (mode === "checking") {
    return <div className="root"><span className="status">Loading…</span></div>;
  }

  if (mode === "needs_model") {
    return (
      <div className="root column">
        <span className="status">Speech model not found</span>
        <button className="download-btn" onClick={handleDownload}>
          Download base.en (~148 MB)
        </button>
      </div>
    );
  }

  if (mode === "downloading") {
    const pct = progress && progress.total > 0
      ? Math.round((progress.downloaded / progress.total) * 100)
      : 0;
    return (
      <div className="root column">
        <span className="status">Downloading model… {pct}%</span>
        <div className="progress-bar">
          <div className="progress-fill" style={{ width: `${pct}%` }} />
        </div>
      </div>
    );
  }

  return (
    <div className="root">
      {mode === "recording" && <div className="recording-dot" />}
      {mode === "transcribing" ? (
        <span className="status">Transcribing…</span>
      ) : (
        <input
          ref={inputRef}
          className="prompt"
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={mode === "recording" ? "Listening… release Option+X to stop" : "What's on your mind?"}
          spellCheck={false}
          autoFocus
        />
      )}
    </div>
  );
}
