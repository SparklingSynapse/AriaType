<div align="center">
<img src="./assets/showcase.jpg" alt="AriaType 展示" width="100%"/>

<br/><br/>

<img src="./assets/ariatype.png" alt="AriaType Logo" height="128" />


### 你的本地私密语音键盘

**按住说话，松开即输入。本地优先，隐私优先。**

[English](README.md) | 简体中文 | [日本語](README-ja.md) | [한국어](README-ko.md) | [Español](README-es.md)

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-pink)](https://github.com/SparklingSynapse/AriaType/releases)
[![Version](https://img.shields.io/badge/version-0.1.0--beta.8-orange)](https://github.com/SparklingSynapse/AriaType/releases)

[下载](https://github.com/SparklingSynapse/AriaType/releases) • [文档](#快速开始) • [社区](https://github.com/SparklingSynapse/AriaType/discussions) • [官网](https://ariatype.com)

</div>

---

## ✨ AriaType 是什么？

AriaType 是一款**本地优先的语音键盘**，会安静地在后台运行。需要输入时，只要按住快捷键（默认 `Shift+Space`），自然说话，然后松开。AriaType 会立即将你的语音转写成文字，并输入到任何正在使用的应用中——无论是 VS Code、Slack、Notion，还是浏览器。

它由**精挑细选并做过优化的本地 AI 模型**驱动，负责语音识别与文本润色——不做“随便挑模型”的随机组合，只用适合的工具完成工作。

**你的语音数据永远不会离开设备：100% 私密，100% 本地。**

---

## 🚀 快速开始

### 安装

**macOS（Apple Silicon）**

1. 下载最新的 [.dmg 文件](https://github.com/SparklingSynapse/AriaType/releases)
2. 打开 .dmg，并将 AriaType 拖拽到 Applications
3. 从 Applications 启动 AriaType

**Windows** 🚧 开发中

Windows 支持正在开发中。可以[关注本仓库](https://github.com/SparklingSynapse/AriaType)或[参与讨论](https://github.com/SparklingSynapse/AriaType/discussions)获取更新。

### 首次设置

1. **授予权限**：按提示允许“麦克风”和“辅助功能（Accessibility）”权限
2. **下载模型**：选择 **Base** 模型以平衡速度与准确度
3. **设置语言**：自动检测通常效果很好，或选择你的主要语言
4. **试用**：打开任意文本编辑器，按住 `Shift+Space`，说“Hello world”，松开即可看到文字输入

### 基础用法

```
1. 按住 → Shift+Space（或你自定义的热键）
2. 说话 → 说出你想输入的内容
3. 松开 → 文字立即出现
```

---

## 🎯 关键特性

### 🔒 隐私优先

你的语音数据**永远不会离开电脑**。所有处理都在本地完成，使用的是**精心选择、深度优化**的语音识别与文本润色模型。无云端、无服务器、无数据收集（除非你选择加入匿名统计）。

### 🎙️ 智能降噪

提供三种模式，自动过滤环境噪音：

- **Auto**：自动识别并适配噪音水平
- **Always On**：最强降噪
- **Off**：原始音频输入

### ✨ AI 文本润色

使用**精选的本地 AI 模型**自动清理你的口语表达：

- 去除口头禅与语气词（如 “um”、“uh”、“like”）
- 修正语法与标点
- 自然排版与格式化
- 全程在设备本地处理，隐私最大化

### 🌍 100+ 语言支持

完整支持：

- 英语、中文（简体/繁体）
- 日语、韩语、西班牙语、法语
- 德语、意大利语、葡萄牙语、俄语
- 以及其他 90+ 语言

### ⚡ 智能功能

- **全局热键**：在任何应用中都可用
- **Smart Pill**：极简悬浮指示器，可显示音量
- **速度/准确度模式**：按你的优先级优化
- **一键重写**：将文本变得更正式、更简洁，或修复语法
- **可自定义**：调整热键、语言与行为

---

## 📋 系统要求

- **系统**：macOS 12.0（Monterey）或更高
- **芯片**：Apple Silicon（M1、M2、M3、M4）
- **内存**：至少 8GB（推荐 16GB）
- **存储**：模型占用约 2-5GB

---

## 🛠️ 高级配置

### 自定义热键

进入 Settings → Hotkeys，自定义触发按键组合。

### 模型选择

AriaType 在语音转写与文本润色上使用**精心选择并做过优化的模型**：

**语音识别模型（Whisper 系列）**：

- **Tiny**：最快，准确度较低（~75MB）
- **Base**：平衡（推荐）（~150MB）
- **Small**：更高准确度（~500MB）
- **Medium**：最高准确度（~1.5GB）

**文本润色**：由本地 LLM 驱动，针对语法修正与自然格式化做过优化。

所有模型均在设备本地运行——模型下载完成后无需联网。

### 语言设置

- **自动检测**：自动识别你所说语言
- **固定语言**：锁定到指定语言以获得更佳准确度

---

## 💬 社区与支持

- **Issues**：在 [GitHub Issues](https://github.com/SparklingSynapse/AriaType/issues) 提交问题或需求
- **Discussions**：在 [GitHub Discussions](https://github.com/SparklingSynapse/AriaType/discussions) 参与社区讨论
- **官网**：访问 [ariatype.com](https://ariatype.com) 了解更多信息

---

## 🤝 贡献

欢迎贡献！包括但不限于：

- 🐛 Bug 报告
- 💡 功能建议
- 📝 文档改进
- 🔧 代码贡献

请在 [GitHub](https://github.com/SparklingSynapse/AriaType) 提交 issue 或 pull request。

---

## 📄 许可证

本项目使用 **GNU Affero General Public License v3.0（AGPL-3.0）** 授权。

这意味着：

- ✅ 你可以自由使用、修改与分发
- ✅ 永远开源
- ⚠️ 如果你修改后再分发，需要公开你的改动
- ⚠️ 如果你将修改版作为服务运行，也必须公开源代码

详见 [LICENSE](LICENSE)。

---

## 🌟 支持项目

如果 AriaType 帮你提升了效率，欢迎：

- ⭐ 给仓库点个 Star
- 🐦 分享给更多人
- 💬 参与社区讨论
- 🐛 反馈问题，帮助改进

---

<div align="center">

**Made with ❤️ for developers, writers, and anyone who thinks faster than they type**

[立即下载](https://github.com/SparklingSynapse/AriaType/releases) • [开始使用](#快速开始) • [加入社区](https://github.com/SparklingSynapse/AriaType/discussions)

</div>
