# Changelog

## 0.3.0 (2026-09-20)

### Added

- System Prompt & Persona and Action Templates as standalone entries on the main settings page
- Whisperfish attribution on the About page as a build reference inspiration
- AI action template properties and management functions consolidated into AppSettings component

### Changed

- Use chat icon for System Prompt & Persona settings entry
- Use robot icon for AI Assistant settings entry
- Narrow AI Assistant settings description to tool permissions

## 0.2.1 (2026-09-19)

### Added

- Automatic provisioning and synchronization of bundled documentation on app launch
- Package architecture decision records (ADRs) and user documentation directly in application RPM
- Precompiled Vue templates eliminating runtime eval and enabling strict CSP headers

### Changed

- Restructure Sailfish QML directory layout into dedicated subdirectories
- Validate AsciiDoc format for Architecture Decision Records in release tooling

### Fixed

- Restore original AsciiDoc ADR numbering (001–013)
- Update test suite paths for restructured QML components

## 0.2.0 (2026-09-19)

### Added

- Rename project from notesplusplus to notesplus
- AI assistant with tool execution: the agent can create, rename, move, and delete notes directly from chat
- Speech-to-text input via Whisper (on-device STT engine with push-to-talk)
- External source import: bring in content from clipboard, URLs, or files
- Block-level editing: move, delete, and re-render individual AsciiDoc blocks
- TLS support with self-signed certificate generation for HTTPS access
- Web access authentication with device-approve/deny challenge
- Translations for German and Spanish

### Fixed

- AsciiDoc parser: inline formatting now works inside list items and block titles
- AsciiDoc parser: nested sidebar and quote blocks render correctly
- Server: large notes no longer truncate during save
- Search: FTS index rebuilds correctly after bulk imports

## 0.1.0 (2026-07-15)

### Added

- Initial release of Notes Plus for Sailfish OS
- Daily journal with auto-managed pages
- AsciiDoc editing with live preview
- Page linking and full-text search
- HTML5 export
- Built-in web server for document access
- AI assistant integration (Ollama / MiMoCode)
- External source import (clipboard, URLs, files)
- German and Spanish translations
