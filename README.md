# Notes Plus

A fast, offline-first AsciiDoc notes app for **Sailfish OS** with an AI assistant, speech-to-text, and a built-in web editor accessible from any browser on your network.

## Features

- **AsciiDoc Notes** — Full parser with headings, lists, tables, code blocks, admonitions, cross-references, and image support
- **AI Assistant** — Chat with an LLM (Ollama or OpenAI-compatible) to edit, summarize, beautify, and create notes with tool-calling and diff previews
- **Import Assistant** — Convert external text, URLs, clipboard, or files into structured AsciiDoc notes via AI
- **Speech-to-Text** — Offline Whisper-based voice input for dictating prompts and notes directly on device
- **Web Editor** — Embedded HTTPS server with a Vue 3 split-pane editor, live preview, and full AI assistant access from any browser
- **Full-Text Search** — Instant SQLite FTS5 search across all notes
- **Journal** — Daily notes with checkbox task lists
- **Authentication** — Verification code challenge with approve/deny on the phone for secure remote access

## Screenshots

The app icon is at `rpm/icons/128x128/harbour-notesplus.png`.

## Installation

### Sailfish OS

1. Download the latest `.rpm` from the Releases page
2. Transfer to your phone and tap to install (or use `devel-su pkcon install harbour-notesplus.rpm`)
3. Launch **Notes Plus** from the app grid

### Web UI

1. Enable the web server in **Settings > Services > Server Active**
2. Open the displayed URL (e.g. `http://192.168.1.100:8080`) from any browser on the same network
3. Authenticate if required (configured in Settings > Services)

## Building from Source

### Prerequisites

- [Sailfish OS SDK](https://docs.sailfishos.org/Tools/Sailfish_SDK/) with Docker engine
- `sfdk` in PATH
- Build target: `SailfishOS-5.1.0.11-aarch64` (configurable via `SAILFISH_TARGET` env var)

### Build

```bash
./build-sailfish.sh
```

### Deploy to Phone

```bash
./deploy-sailfish.sh
```

### Run Tests (Host)

```bash
cargo test -p notesplus-core
```

See [DEVELOPMENT.md](DEVELOPMENT.md) for detailed architecture, testing, and CI documentation.

## Project Structure

```
├── notesplus-core/          # Pure Rust core engine
│   ├── src/
│   │   ├── parser/              # AsciiDoc AST parser
│   │   ├── agent/               # LLM client + tool-calling agent
│   │   ├── stt/                 # Whisper STT engine + model downloader
│   │   ├── server/              # Embedded HTTP/HTTPS server + auth
│   │   ├── db.rs                # SQLite + FTS5
│   │   └── page.rs              # Page indexing & management
│   └── assets/web/              # Vue 3 web UI (static files)
├── notesplus-sailfish/      # Sailfish OS GUI application
│   ├── src/bridge/              # Rust ↔ QML bridge objects
│   └── qml/                     # Sailfish Silica QML pages
├── build-sailfish.sh            # Cross-compile for Sailfish OS
└── deploy-sailfish.sh           # Deploy RPM to phone via SSH
```

## License

See individual source files for license information.
