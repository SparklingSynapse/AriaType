# Changelog

All notable changes to the desktop application will be documented in this file.

## Unreleased

### Features

- Add window context capture via OCR to improve polish accuracy (e749271)
- Optimize UI for better experience (6f4936c)
- Optimize onboarding guide (db9e682)
- Optimize text inject performance (c92c4ce)

### Bug Fixes

- Add default value for pill_size field (f027349)

## v0.4.0 (2026-04-25)

### Features

- Implement multi-shortcut profiles and update UI (6814403)
- Add changelog viewer in about page (0c72a43)
- Add audio command boundary (b4e7a53)
- Add single-instance plugin and handle silent recordings (a37e33c)
- Add inhouse dev variant with custom icons and hotkey labels (8b43163)

### Bug Fixes

- Update changelog page layout (8380247)
- Shortcut not working when no permission (ed322f3)
- Prevent changelog fetch loop and improve UI (254476d)

## v0.3.0 (2026-04-13)

### Features

- Improve audio chunking (d9e6cf6)
- Add recording cancellation and enhanced retry experience (e12a97e)
- Add transcription retry functionality (c274588)
- Refactor hotkey (d01a7b7)

## v0.2.0 (2026-04-11)

### Features

- Add model file size validation and metal headers (151a7dd)

## v0.1.2 (2026-04-08)

### Features

- Add custom template management and improve VAD (e4618d9)

### Bug Fixes

- Return committed transcript from ElevenLabs finish() and add query params (64ede6b)

## v0.1.1 (2026-04-06)

### Features

- Add Gemma 2B IT local model support (590cc77)
- Add history dashboard, cloud service UI and improve design (9007d47)

### Bug Fixes

- Eliminate flash of unstyled content on app startup (b56424a)

## v0.1.0-beta.8 (2026-03-09)

### Bug Fixes

- Embed beep audio files at compile time (a766981)
