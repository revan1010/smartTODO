# smartTODO

A system-wide AI productivity assistant for macOS that turns spoken thoughts into organized tasks and calendar events with a single hotkey.

Hold **Option+X** from anywhere on your Mac — even over fullscreen apps — speak a request, and smartTODO transcribes it offline using Whisper and fills an input for review before capture. No cloud required for speech-to-text.

## Current Status

**Phase 1 (complete):** Menu bar app, global hotkey, spotlight-style input window, NSPanel overlay (works over fullscreen apps).

**Phase 2 (complete):** Offline speech-to-text via Whisper.cpp with Metal GPU acceleration, push-to-talk recording, model download flow.

**Upcoming:** LLM-powered intent parsing (Claude API), Apple Reminders integration (EventKit), calendar event creation, Todoist/Notion integrations.

## Features

- **Global hotkey** — `Option+X` activates from any app, any Space, including fullscreen
- **Spotlight-style UI** — minimal floating panel, appears centered on the active monitor
- **Offline speech-to-text** — Whisper.cpp (base.en model) running locally with Metal acceleration on Apple Silicon
- **Push-to-talk** — hold `Option+X` to record, release to transcribe
- **Text fallback** — start typing during recording to cancel voice and switch to keyboard input
- **Menu bar app** — no dock icon, lives in the system tray
- **Auto-hide** — panel hides when it loses focus (except during recording)
- **First-run model download** — downloads the Whisper model (~148 MB) on first launch with progress bar

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop framework | [Tauri 2](https://tauri.app/) |
| Frontend | React + TypeScript + Vite |
| Backend | Rust |
| Speech recognition | [whisper-rs](https://github.com/tazz4843/whisper-rs) (whisper.cpp with Metal) |
| Audio capture | [cpal](https://github.com/RustAudio/cpal) |
| Window overlay | [tauri-nspanel](https://github.com/ahkohd/tauri-nspanel) (NSPanel for fullscreen support) |
| Global hotkey | [tauri-plugin-global-shortcut](https://github.com/tauri-apps/plugins-workspace) |

## Prerequisites

- **macOS** (Apple Silicon recommended for Metal acceleration)
- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (v20+)
- [pnpm](https://pnpm.io/)
- [CMake](https://cmake.org/) (required to build whisper.cpp)

Install prerequisites with Homebrew:

```bash
brew install pnpm cmake
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Getting Started

```bash
# Clone the repo
git clone https://github.com/your-username/smartTODO.git
cd smartTODO

# Install JS dependencies
pnpm install

# Run in development mode
pnpm tauri dev
```

The first Rust build takes several minutes (compiling whisper.cpp from source with Metal support). Subsequent builds are fast.

### First Launch

1. The app starts with no dock icon — look for the **smartTODO icon in the menu bar** (top-right of screen).
2. Press `Option+X` — the input window appears.
3. On first launch, you'll see a **"Download base.en (~148 MB)"** button. Click it to download the Whisper speech model.
4. After download completes, the app is ready for voice input.

### macOS Permissions

The app needs two permissions (macOS will prompt automatically):

- **Accessibility** — for the global hotkey to work across apps
  - System Settings > Privacy & Security > Accessibility > enable your terminal (or smartTODO)
- **Microphone** — for voice recording
  - System Settings > Privacy & Security > Microphone > enable your terminal (or smartTODO)

In dev mode, grant these permissions to the **terminal app** you run `pnpm tauri dev` from (Terminal, iTerm, Cursor, etc.).

## Usage

| Action | How |
|--------|-----|
| **Voice input** | Hold `Option+X`, speak, release to transcribe |
| **Text input** | Press `Option+X`, type your text, press `Enter` |
| **Switch to text during recording** | Start typing any key (cancels recording) |
| **Dismiss** | Press `Escape` or click away from the panel |
| **Quit** | Click the menu bar icon > Quit smartTODO |

## Project Structure

```
smartTODO/
├── src/                          # React frontend
│   ├── App.tsx                   # Main UI — 4 states: needs_model/downloading/idle/recording/transcribing
│   ├── App.css                   # Spotlight-style dark blur styling
│   └── main.tsx                  # React entry point
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── main.rs               # Tauri entry point
│   │   ├── lib.rs                # App setup: tray, hotkey, panel show/hide, recording lifecycle
│   │   ├── panel.rs              # NSPanel configuration (fullscreen overlay, auto-hide)
│   │   ├── commands.rs           # Tauri commands: capture_input, recording controls
│   │   ├── audio.rs              # Microphone capture via cpal, downmix + resample to 16kHz
│   │   ├── whisper.rs            # Whisper model loading + transcription
│   │   └── model.rs              # Model file management + download with progress
│   ├── tauri.conf.json           # Window config, bundle settings
│   ├── capabilities/default.json # Permission grants for window/shortcut APIs
│   └── Cargo.toml                # Rust dependencies
├── package.json
├── vite.config.ts
└── tsconfig.json
```

## How It Works

1. **Hotkey press (`Option+X`)** — Rust immediately starts recording audio via cpal and shows the NSPanel overlay
2. **Recording** — Audio is captured as f32 samples from the default input device, stored in a shared buffer
3. **Hotkey release** — Recording stops, samples are downmixed to mono and resampled to 16kHz
4. **Transcription** — whisper-rs processes the audio on a background thread using Metal GPU acceleration
5. **Result** — Transcript fills the input field; user can edit and press Enter to capture
6. **Capture** — Currently prints to stdout; future phases will parse intent and create tasks/events

## Development

```bash
# Run dev server (frontend hot-reload + Rust rebuild on change)
pnpm tauri dev

# TypeScript type check
pnpm exec tsc --noEmit

# Rust check (from src-tauri/)
cd src-tauri && cargo check

# Build production binary
pnpm tauri build
```

## Roadmap

- [ ] **Phase 3** — Intent parsing via Claude API (text to structured task/event JSON)
- [ ] **Phase 4** — Apple Reminders integration via EventKit
- [ ] **Phase 5** — Calendar event creation via EventKit
- [ ] **Phase 6** — Todoist and Notion integrations
- [ ] **Phase 7** — History, settings UI, model picker, onboarding

## License

MIT
