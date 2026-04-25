<div align="center">
<img src="./assets/showcase-0.3.png" alt="AriaType 展示" width="100%"/>

<br/><br/>

### AriaType

AriaType - 开源 AI 语音转文字输入工具 | 强大的Typeless替代方案

[English](README.md) | 简体中文 | [日本語](README-ja.md) | [한국어](README-ko.md) | [Español](README-es.md)

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE) [![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-pink)](https://github.com/joe223/AriaType/releases) [![Windows](https://img.shields.io/badge/Windows-WIP-yellow)](https://github.com/joe223/AriaType) [![Version](https://img.shields.io/badge/version-0.3-green)](https://github.com/joe223/AriaType/releases)

[下载](https://github.com/joe223/AriaType/releases) • [文档](context/README.md) • [讨论区](https://github.com/joe223/AriaType/discussions) • [官网](https://ariatype.com)

</div>

> [!TIP]
> **v0.3 更新内容**
> - **重试失败转录** – 历史记录中的失败条目可使用保存的音频重试
> - **ESC 取消录音** – 录音时按 ESC 取消，不会产生无效记录
> - **长录音更稳定** – 修复了长时间录音被截断的问题
> - **支持 Fn 键** – 自定义快捷键支持 Fn 组合键

---

## 它是什么

AriaType 是一个本地优先的语音输入工具，支持 macOS 和 Windows。

它常驻后台运行。需要输入时，按住全局热键，说话，松开，文字就会直接进入当前应用。你可以把它理解成一个真正能在日常工作里高频使用的 AI 语音键盘。

## 核心功能

- ⚡️ **快速响应** – 平均转录耗时低于 500ms，提升你的编码/写作效率
- 🔒 **本地优先** – 默认本地 STT/润色，语音内容不上传云端
- 🎙 **双快捷键** – `Cmd+/` 听写（原样输出），`Opt+/` 智能润色
- 🇨🇳 **中文优化** – SenseVoice 针对中文、日语、韩语深度优化
- ✨ **智能润色** – 自动去除口头禅、修正标点、整理表达
- 🧩 **自定义模板** – 为常用场景创建专属润色模板
- 🌍 **100+ 语言** – 自动检测或指定输出语言
- ☁️ **可选云端** – 需要时自带 API Key 开启云端增强

## 使用建议

- 中文/CJK 场景优先用 `SenseVoice`，普通话、粤语、日语体验更好。
- 英文/国际语言用 `Whisper`，语言覆盖更广。
- 口语多口头禅？先转录再应用 `Remove Fillers` 或 `Make Concise`。
- 专业术语多？提前设置领域和术语词库。

## 支持平台

| 平台 | 状态 | 系统要求 |
|------|------|----------|
| macOS (Apple Silicon) | ✅ 稳定 | macOS 12.0+, M 系列芯片 |
| macOS (Intel) | ✅ 稳定 | macOS 12.0+, Intel Core i5+ |
| Windows | 🔧 开发中 | 即将推出 |

## 安装与使用

从 [ariatype.com](https://ariatype.com) 下载，安装后授权麦克风和辅助功能权限即可使用。无需注册账号，无需配置向导。

## 许可证

AriaType 采用 [AGPL-3.0](LICENSE) 许可证。

- 可在 AGPL-3.0 条款下自由使用、修改和分发。
- 详细条款见 `LICENSE` 文件。