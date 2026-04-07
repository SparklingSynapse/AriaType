<div align="center">
<img src="./assets/showcase.png" alt="AriaType Showcase" width="100%"/>

<br/><br/>

### AriaType

AriaType – Open-Source AI Voice-to-Text Input | Powerful Typeless Alternative

English | [简体中文](README-cn.md) | [日本語](README-ja.md) | [한국어](README-ko.md) | [Español](README-es.md)

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE) [![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-pink)](https://github.com/SparklingSynapse/AriaType/releases) [![Version](https://img.shields.io/badge/version-0.1.0--beta.8-orange)](https://github.com/SparklingSynapse/AriaType/releases)

[Download](https://github.com/SparklingSynapse/AriaType/releases) • [Docs](docs/README.md) • [Discussions](https://github.com/SparklingSynapse/AriaType/discussions) • [Website](https://ariatype.com)

</div>

---

## What It Is

AriaType is a local-first voice typing app for macOS.

It stays in the background. When you want to type, hold a global hotkey, speak naturally, and release. Your speech becomes text in the current app. Think of it as an AI voice keyboard you can use all day for docs, chat, notes, coding, and any workflow where speaking is faster than typing.

## Core Benefits

- 🎙 Global hotkey voice typing: the default is `Shift+Space`, so you can press, speak, and release without breaking flow.
- ↔️ Direct text insertion across apps: output goes into the current app, including editors, browsers, chat tools, and notes apps.
- 🔒 Local-first privacy: speech recognition and text cleanup run on your machine by default, so your voice does not need to leave your device.
- ⚡ Dual local STT engines: switch between `Whisper` and `SenseVoice` based on language, speed, and accuracy needs.
- 🌍 100+ language support: use auto-detect or choose a specific output language for multilingual workflows.
- 🇨🇳 Better fit for Chinese and CJK use: `SenseVoice` is especially strong for Mandarin, Traditional Chinese, Cantonese, and broader CJK-heavy usage.
- ✨ More than transcription: clean up punctuation, filler words, tone, and phrasing before the final text is inserted.
- 🧩 Template-based polish: use built-in styles like `Remove Fillers`, `Formal Style`, `Make Concise`, and `Agent Prompt`, or create your own templates.
- ☁️ Optional cloud boost: enable `Cloud STT` and `Cloud Polish` separately in `Cloud Services` when a workflow benefits from remote AI.
- 📡 Streaming partial transcription: supported cloud STT providers can return live partial text while you are still speaking.
- 🧠 Domain and glossary guidance: improve recognition with domain presets, subdomains, initial prompts, and custom glossary terms.
- 🧭 Language-based model recommendations: the app can recommend better models for the language you plan to use.
- 📍 Always-on-top capsule: a floating capsule shows recording, transcribing, polishing, and audio activity states in real time.
- ⚙️ Capsule visibility and position controls: choose whether it is always visible, recording-only, hidden, and place it where it fits your workflow.
- 🎛 Tunable audio pipeline: adjust denoise and silence trimming (VAD) for noisy rooms, quiet speech, and long pauses.
- 📝 Reliable text injection: it prefers keyboard-style insertion and falls back to clipboard paste with clipboard restoration when needed.
- 🔎 Local history and search: keep searchable transcription history on your machine for reuse and review.
- 📊 Usage dashboard: track captures, recognition time, local-vs-cloud share, and streak-style usage patterns.
- ⬇️ Model download management: download, remove, and monitor local model status with progress feedback.
- 🎨 Desktop-friendly details: theme switching, launch-on-login, custom hotkeys, and hold/toggle recording modes are built in.

## Screenshot Tour

<table>
  <tr>
    <td width="50%"><img src="./assets/features/homepage-light.png" alt="AriaType home screen in light mode" width="100%"/></td>
    <td width="50%"><img src="./assets/features/homepage-dark.png" alt="AriaType home screen in dark mode" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>Home, Light Theme</strong><br/>The main voice typing workspace with quick access to settings and recent activity.</td>
    <td><strong>Home, Dark Theme</strong><br/>The same workflow in a darker setup for long editing sessions.</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/hotkey.png" alt="Hotkey and recording mode settings" width="100%"/></td>
    <td width="50%"><img src="./assets/features/general-vad.png" alt="Audio processing settings with denoise and VAD" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>Hotkey and Recording Mode</strong><br/>Customize shortcuts and choose between hold-to-record and toggle mode.</td>
    <td><strong>Audio Processing</strong><br/>Tune denoise and silence trimming to match your room, mic, and speaking style.</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/private-model-stt.png" alt="Local STT model management" width="100%"/></td>
    <td width="50%"><img src="./assets/features/private-model-polish.png" alt="Local polish model management" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>Private STT Models</strong><br/>Download and manage local Whisper and SenseVoice models for offline transcription.</td>
    <td><strong>Private Polish Models</strong><br/>Run local cleanup and rewriting with Qwen, LFM, and Gemma model options.</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/cloud-service-stt.png" alt="Cloud STT configuration" width="100%"/></td>
    <td width="50%"><img src="./assets/features/cloud-service-polish.png" alt="Cloud polish configuration" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>Cloud STT</strong><br/>Bring your own API key and turn on cloud transcription only when a workflow needs it.</td>
    <td><strong>Cloud Polish</strong><br/>Connect your own provider for stronger rewrite and cleanup workflows.</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/polish-template.png" alt="Polish template management" width="100%"/></td>
    <td width="50%"><img src="./assets/features/home-dashboard.png" alt="Usage dashboard with capture stats" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>Polish Templates</strong><br/>Start from built-in rewrite styles or create your own templates for recurring writing tasks.</td>
    <td><strong>Usage Dashboard</strong><br/>See how often you use voice typing, how long processing takes, and how your habit evolves.</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/home-dashboard-2.png" alt="Usage dashboard detail cards" width="100%"/></td>
    <td width="50%"><img src="./assets/features/history.png" alt="Searchable transcription history view" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>Deeper Stats</strong><br/>Track local-vs-cloud usage, streaks, and other details that help you refine your workflow.</td>
    <td><strong>Searchable History</strong><br/>Browse past transcriptions, filter by source, and quickly find the text you want to reuse.</td>
  </tr>
</table>

## Usage Tips

- If you prefer an offline workflow and mainly speak Chinese, start with `SenseVoice`. It is the strongest fit for CJK-heavy usage in this project and is usually the first model worth trying for Mandarin, Traditional Chinese, and Cantonese scenarios.
- If you mainly work in English or other international languages, start with `Whisper`. It covers more languages and gives you a wider range of model sizes and trade-offs.
- If you want the most stable local setup, download your preferred local model first and only turn on cloud services for specific tasks that need them.
- If you already pay for your own AI services, go to `Cloud Services`, add your own `API Key`, and enable `Cloud STT` and `Cloud Polish` as needed.
- If your speech is casual and full of filler words, transcribe first and then apply `Remove Fillers` or `Make Concise` instead of trying to speak in “final draft” mode.
- If you use domain-specific terms, set the output language, domain, subdomain, and glossary in advance for more reliable recognition.
- Place the capsule where you can notice it without blocking content; heavy users usually prefer keeping it visible.

## License

AriaType is licensed under [AGPL-3.0](LICENSE).

- You can use, modify, and distribute it under the terms of AGPL-3.0.
- See `LICENSE` for the full legal text and obligations.
