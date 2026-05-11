<div align="center">
  <img src="./assets/logo.png" alt="AriaType logo" width="96"/>

  <h1>AriaType</h1>

  <h3>Voice-driven writing, input, and cross-app work for your desktop.</h3>

  <p>
    AriaType is the voice layer for your desktop, turning spoken thoughts into context-aware work right where your cursor is.
  </p>

  <p>
    Reply, take notes, draft prompts, and clean up documents without leaving the app you are already using.
  </p>

  <p>
    <a href="https://github.com/joe223/AriaType/releases">Download</a>
    ·
    <a href="https://ariatype.com">Website</a>
    ·
    <a href="context/README.md">Docs</a>
    ·
    <a href="https://github.com/joe223/AriaType/discussions">Discussions</a>
    ·
    <a href="README-cn.md">简体中文</a>
  </p>

  <p>
    <img alt="License" src="https://img.shields.io/badge/license-AGPL--3.0-blue"/>
    <img alt="macOS" src="https://img.shields.io/badge/macOS-supported-pink"/>
    <img alt="Local first" src="https://img.shields.io/badge/local--first-yes-brightgreen"/>
    <img alt="Context aware" src="https://img.shields.io/badge/context--aware-yes-purple"/>
    <img alt="Pause detection" src="https://img.shields.io/badge/pause%20detection-built%20in-orange"/>
    <img alt="Bring your own services" src="https://img.shields.io/badge/BYO-services-black"/>
  </p>

  <img src="./assets/showcase-0.5.png" alt="AriaType Showcase" width="100%"/>
</div>

---

## Starting With Writing

When a thought is ready, you should be able to speak it into the work in front of you.

AriaType starts with the writing you do all day: replies, notes, prompts, rough ideas, and text that needs to land in the app you are already using.

It does not only listen to what you said. It also cares where you are writing, where the text should land, and how people actually speak.

## Highlights

- 🔊 **Noise reduction**: Filter everyday background noise for more stable voice input.
- 🤫 **VAD voice activity detection**: Detect speech, pauses, and silence with less manual control.
- 🧠 **Context awareness**: Use the current window to better match the app, field, and task.
- ✍️ **AI polish**: Remove fillers, fix punctuation, tighten wording, and turn speech into usable text.
- 🌏 **Multilingual**: Support Chinese, English, Japanese, Korean, and more for daily writing.
- 🎯 **Cursor insertion**: No window switching or copy-paste; text lands where you are working.
- 🔒 **Privacy and security**: Local-first by default, so everyday voice content stays on your device.
- 🔌 **Custom services**: Connect your preferred speech or language provider when needed.
- 🌓 **Dark mode**: Light/Dark themes for a more comfortable desktop experience.
- 🖥️ **Custom floating window**: Adjust the Pill Window and shortcuts to fit how you work.

## Use It For

- **Reply faster** in chat apps, email, collaboration tools, and browser fields.
- **Capture ideas** before they disappear, without stopping to type.
- **Draft prompts and instructions** from natural spoken language.
- **Clean up rough speech** by removing fillers, fixing punctuation, and tightening wording.
- **Write across apps** with one consistent voice workflow.
- **Use the current window as context** when the output should fit the task in front of you.
- **Stay private by default** with local processing for everyday work.

## Why It Feels Different

AriaType is built for your desktop, not just a single input box.

- **Works in the current app**: text lands at the cursor instead of forcing you into another tool.
- **Matches natural speech**: pauses, silence, and everyday noise are part of the experience.
- **Fits the task in front of you**: current-window context helps the output match the app, field, and moment.
- **Lets you choose privacy and power**: use local-first defaults or connect your own services.
- **Feels complete on desktop**: themes, multilingual UI, shortcuts, and a customizable Pill Window.

## Designed Around Your Desktop

AriaType is a voice layer for the desktop, not another place you have to move your work into.

It is built around the current app, the current field, and the current cursor. Wherever you are working, that is where your voice becomes usable text.

## How It Works

1. Install AriaType.
2. Grant microphone and accessibility permissions.
3. Hold the shortcut in any app.
4. Speak naturally.
5. Release and the text appears at the cursor.

By default, `Cmd + /` starts raw dictation, and `Opt + /` inserts polished output. You can change the shortcuts in settings.

## Privacy And Permissions

AriaType asks for only the permissions needed to make desktop voice interaction work:

- 🎙️ **Microphone**: Records your speech.
- ⌨️ **Accessibility**: Inserts text into the active app.
- 🪟 **Screen/window context**: Optional, used for context awareness so output can better match the current app, field, and task.

AriaType does not require an account and does not upload your voice by default. Remote services are optional and are used only when you configure and enable them.

## Platforms

|  | Platform | Status | Requirements |
|---|---|---|---|
| <img src="./assets/platform-apple.svg" alt="Apple" width="18"/> | macOS Apple Silicon | Stable | macOS 12.0+ |
| <img src="./assets/platform-apple.svg" alt="Apple" width="18"/> | macOS Intel | Stable | macOS 12.0+ |
| <img src="./assets/platform-windows.svg" alt="Windows" width="18"/> | Windows | In progress | Coming soon |

## Download

Download the latest version from:

- [GitHub Releases](https://github.com/joe223/AriaType/releases)
- [Official website](https://ariatype.com)

After installation, follow the system prompts to grant microphone and accessibility permissions.

## Project Status

AriaType is under active development. The macOS version is usable today, and the Windows version is in progress.

Current focus:

- More accurate speech recognition
- Better Chinese and multilingual workflows
- More reliable cross-app insertion
- More useful text polish and custom templates
- A quieter, more customizable desktop voice experience

If you want voice to become a real layer of desktop work, star the repo to follow the project and support its development.

## Contributing

Issues, discussions, product feedback, and code contributions are welcome.

Useful ways to help:

- Report recognition issues
- Share results across languages, accents, and devices
- Improve onboarding and installation flows
- Refine desktop interaction details
- Add or improve text polish templates
- Improve docs and translations

Developer documentation starts at [context/README.md](context/README.md).

## License

AriaType is licensed under [AGPL-3.0](LICENSE).
