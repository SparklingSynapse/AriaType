<div align="center">
<img src="./assets/showcase.png" alt="AriaType Showcase" width="100%"/>

<br/><br/>

### AriaType

Local voice typing for macOS. Hold a hotkey, speak, release, and the text goes into the current app.

English | [简体中文](README-cn.md) | [日本語](README-ja.md) | [한국어](README-ko.md) | [Español](README-es.md)

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-pink)](https://github.com/SparklingSynapse/AriaType/releases)
[![Version](https://img.shields.io/badge/version-0.1.0--beta.8-orange)](https://github.com/SparklingSynapse/AriaType/releases)

[Download](https://github.com/SparklingSynapse/AriaType/releases) • [Docs](docs/README.md) • [Discussions](https://github.com/SparklingSynapse/AriaType/discussions) • [Website](https://ariatype.com)

</div>

---

## What It Is

AriaType is a desktop app for voice typing.

It stays in the background. When you want to type, hold a shortcut key, speak naturally, and release. AriaType transcribes your speech and types into the active app, including VS Code, Slack, Notion, and browsers.

## Why Use It

- Local-first: speech recognition and text cleanup run on your machine by default.
- Private: your voice data does not need to leave your computer.
- Simple: hold `Shift+Space`, talk, release.
- Useful: works across apps and supports 100+ languages.
- Flexible: pick speed or accuracy, change hotkeys, and rewrite text when needed.

## Quick Start

### Install

- macOS (Apple Silicon): download the latest [.dmg](https://github.com/SparklingSynapse/AriaType/releases), drag AriaType to Applications, and open it.
- Windows: support is in progress.

### First Run

1. Grant Microphone and Accessibility permissions.
2. Download a speech model. `Base` is a good default.
3. Pick a language or leave auto-detect on.
4. Open any editor and try it.

## How It Works

1. Hold your hotkey. The default is `Shift+Space`.
2. Speak.
3. Release to insert text.

AriaType can also clean up filler words, punctuation, and grammar before it types the final text.

## System Requirements

- macOS 12 or later
- Apple Silicon Mac
- 8 GB RAM minimum, 16 GB recommended
- 2-5 GB free disk space for models

## For Developers

This repo is a monorepo:

- `apps/desktop`: the Tauri desktop app
- `packages/website`: the marketing site
- `packages/shared`: shared TypeScript types and constants

### Setup

```bash
pnpm install
pnpm tauri:dev
pnpm --filter @ariatype/website dev
```

### Start Here

- [`AGENTS.md`](AGENTS.md): workflow, verification commands, and repo rules
- [`docs/README.md`](docs/README.md): documentation index
- [`apps/desktop/CONTRIBUTING.md`](apps/desktop/CONTRIBUTING.md): desktop app guide
- [`packages/website/CONTRIBUTING.md`](packages/website/CONTRIBUTING.md): website guide

## Community

- Bugs and feature requests: [GitHub Issues](https://github.com/SparklingSynapse/AriaType/issues)
- Questions and discussion: [GitHub Discussions](https://github.com/SparklingSynapse/AriaType/discussions)

## License

AriaType is licensed under [AGPL-3.0](LICENSE).
