<div align="center">
<img src="./assets/showcase.png" alt="AriaType 展示" width="100%"/>

<br/><br/>

### AriaType

macOS 上的语音输入工具。按住快捷键，说话，松开，文字就会进入当前应用。

[English](README.md) | 简体中文 | [日本語](README-ja.md) | [한국어](README-ko.md) | [Español](README-es.md)

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-pink)](https://github.com/SparklingSynapse/AriaType/releases)
[![Version](https://img.shields.io/badge/version-0.1.0--beta.8-orange)](https://github.com/SparklingSynapse/AriaType/releases)

[下载](https://github.com/SparklingSynapse/AriaType/releases) • [文档](docs/README.md) • [讨论区](https://github.com/SparklingSynapse/AriaType/discussions) • [官网](https://ariatype.com)

</div>

---

## 它是什么

AriaType 是一个桌面端语音输入工具。

它常驻后台运行。需要输入时，按住快捷键，自然说话，然后松开。AriaType 会把语音转成文字，并直接输入到当前应用里，比如 VS Code、Slack、Notion 和浏览器。

## 为什么用它

- 本地优先：语音识别和文本整理默认都在本机完成。
- 更私密：语音数据不需要离开你的电脑。
- 很直接：按住 `Shift+Space`，说话，松开。
- 很实用：支持跨应用输入，也支持 100 多种语言。
- 可调整：可以在速度和准确度之间取舍，也能改热键和重写文本。

## 快速开始

### 安装

- macOS（Apple Silicon）：下载最新的 [.dmg](https://github.com/SparklingSynapse/AriaType/releases)，拖到 Applications，再打开。
- Windows：还在开发中。

### 首次使用

1. 按提示打开麦克风和辅助功能权限。
2. 下载语音模型。`Base` 适合作为默认选择。
3. 选择语言，或者直接用自动检测。
4. 打开任意编辑器试一下。

## 怎么用

1. 按住热键，默认是 `Shift+Space`。
2. 开始说话。
3. 松开后插入文字。

AriaType 也可以在输入前顺手整理口头禅、标点和语法。

## 系统要求

- macOS 12 或更高版本
- Apple Silicon Mac
- 至少 8 GB 内存，推荐 16 GB
- 至少 2-5 GB 可用磁盘空间用于模型

## 给开发者

这个仓库是一个 monorepo：

- `apps/desktop`：Tauri 桌面应用
- `packages/website`：官网
- `packages/shared`：共享的 TypeScript 类型和常量

### 本地启动

```bash
pnpm install
pnpm tauri:dev
pnpm --filter @ariatype/website dev
```

### 从这里看起

- [`AGENTS.md`](AGENTS.md)：工作流、校验命令和仓库规则
- [`docs/README.md`](docs/README.md)：文档总入口
- [`apps/desktop/CONTRIBUTING.md`](apps/desktop/CONTRIBUTING.md)：桌面应用开发说明
- [`packages/website/CONTRIBUTING.md`](packages/website/CONTRIBUTING.md)：官网开发说明

## 社区

- Bug 和需求：[GitHub Issues](https://github.com/SparklingSynapse/AriaType/issues)
- 提问和讨论：[GitHub Discussions](https://github.com/SparklingSynapse/AriaType/discussions)

## 许可证

AriaType 使用 [AGPL-3.0](LICENSE) 许可证。
