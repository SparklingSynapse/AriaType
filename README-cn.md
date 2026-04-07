<div align="center">
<img src="./assets/showcase.png" alt="AriaType 展示" width="100%"/>

<br/><br/>

### AriaType

AriaType - 开源 AI 语音转文字输入工具 | 强大的Typeless替代方案

[English](README.md) | 简体中文 | [日本語](README-ja.md) | [한국어](README-ko.md) | [Español](README-es.md)

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE) [![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-pink)](https://github.com/SparklingSynapse/AriaType/releases) [![Version](https://img.shields.io/badge/version-0.1.0--beta.8-orange)](https://github.com/SparklingSynapse/AriaType/releases)

[下载](https://github.com/SparklingSynapse/AriaType/releases) • [文档](docs/README.md) • [讨论区](https://github.com/SparklingSynapse/AriaType/discussions) • [官网](https://ariatype.com)

</div>

---

## 它是什么

AriaType 是一个面向 macOS 的本地优先语音输入工具。

它常驻后台运行。需要输入时，按住全局热键，说话，松开，文字就会直接进入当前应用。你可以把它理解成一个真正能在日常工作里高频使用的 AI 语音键盘，适合写文档、回消息、记笔记、写代码注释，或者任何“想说得比打得快”的场景。

## 核心功能与卖点

- 🎙 全局热键语音输入：默认 `Shift+Space`，按住说话、松开即写，真正适合高频日常使用。
- ↔️ 跨应用直接输入：文字可直接进入当前应用，适用于 VS Code、Slack、Notion、浏览器等常见 macOS 工作流。
- 🔒 本地优先与隐私保护：默认优先使用本地识别与本地润色，语音内容不必上传到云端。
- ⚡ 双本地识别引擎：同时支持 `Whisper` 和 `SenseVoice`，可按语言、速度、准确率需求自由切换。
- 🌍 100+ 语言支持：支持自动检测和手动指定输出语言，适合中英混用和多语言办公场景。
- 🇨🇳 中文与 CJK 场景优化：`SenseVoice` 对普通话、繁体中文、粤语以及 CJK 使用场景更友好。
- ✨ 语音转文字之外还能顺手润色：可自动补标点、去口头禅、整理语气、压缩表达，让口语更接近可直接发送的文本。
- 🧩 模板化润色：内置 `Remove Fillers`、`Formal Style`、`Make Concise`、`Agent Prompt` 四种模板，也支持自定义模板。
- ☁️ 云端增强按需开启：`Cloud Services` 中可分别启用 `Cloud STT` 和 `Cloud Polish`，兼顾本地优先和云端增强。
- 📡 流式中间结果：支持的云 STT 服务商可在你还没说完时持续返回部分结果，降低等待感。
- 🧠 领域增强与术语词库：支持领域、子领域、初始提示词与 glossary 设置，适合 IT、法律、医疗等专业场景。
- 🧭 按语言推荐模型：首次使用和切换语言时，系统会基于语言给出更合适的模型推荐，降低选择成本。
- 📍 置顶胶囊悬浮窗：录音、转写、润色、音量状态实时可见，不需要来回切窗口确认。
- ⚙️ 胶囊显示与位置可调：支持常显、仅录音显示、隐藏等模式，也支持多种预设位置。
- 🎛 音频前处理可调：支持降噪和静音裁剪（VAD），在嘈杂环境、长停顿、轻声说话等场景下更容易调到合适状态。
- 📝 文本注入更稳：优先键盘模拟，必要时自动走剪贴板粘贴，并在结束后恢复剪贴板内容，减少打断。
- 🔎 本地历史记录与搜索：所有转写结果都可以本地保存、搜索、回看，方便复用常用表达。
- 📊 使用数据面板：可查看录入次数、处理耗时、本地/云端占比、连续使用天数等统计，帮助形成稳定习惯。
- ⬇️ 模型下载与状态管理：本地模型支持下载、删除、状态识别与进度反馈，不需要手动折腾文件。
- 🎨 桌面端体验完善：支持主题切换、开机启动、热键自定义、按住录音/切换录音等基础能力。

## 功能截图

<table>
  <tr>
    <td width="50%"><img src="./assets/features/homepage-light.png" alt="AriaType 首页浅色模式" width="100%"/></td>
    <td width="50%"><img src="./assets/features/homepage-dark.png" alt="AriaType 首页深色模式" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>首页，浅色模式</strong><br/>主工作区一眼就能看到核心设置、模型状态和最近使用信息。</td>
    <td><strong>首页，深色模式</strong><br/>同样的工作流，在深色环境下更适合长时间使用。</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/hotkey.png" alt="热键与录音模式设置" width="100%"/></td>
    <td width="50%"><img src="./assets/features/general-vad.png" alt="降噪与静音裁剪设置" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>热键与录音模式</strong><br/>可以自定义快捷键，并在按住录音和切换录音之间自由选择。</td>
    <td><strong>音频前处理</strong><br/>可调降噪和静音裁剪，适应不同环境噪声、麦克风和说话节奏。</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/private-model-stt.png" alt="本地 STT 模型管理" width="100%"/></td>
    <td width="50%"><img src="./assets/features/private-model-polish.png" alt="本地润色模型管理" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>本地语音识别模型</strong><br/>下载和管理本地 `Whisper`、`SenseVoice` 模型，离线即可完成转写。</td>
    <td><strong>本地润色模型</strong><br/>可使用 `Qwen`、`LFM`、`Gemma` 等本地模型完成整理和改写。</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/cloud-service-stt.png" alt="云端 STT 配置界面" width="100%"/></td>
    <td width="50%"><img src="./assets/features/cloud-service-polish.png" alt="云端润色配置界面" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>Cloud STT</strong><br/>可配置自己的 API Key，只在需要时开启云端转写能力。</td>
    <td><strong>Cloud Polish</strong><br/>接入你自己的云端模型服务，让润色和改写能力更强。</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/polish-template.png" alt="润色模板管理页面" width="100%"/></td>
    <td width="50%"><img src="./assets/features/home-dashboard.png" alt="使用数据仪表盘" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>润色模板</strong><br/>从内置模板起步，也可以创建你自己的高频写作模板。</td>
    <td><strong>使用数据面板</strong><br/>查看录入次数、处理速度和习惯变化，帮助你把语音输入真正用起来。</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/home-dashboard-2.png" alt="仪表盘详细统计卡片" width="100%"/></td>
    <td width="50%"><img src="./assets/features/history.png" alt="可搜索的历史记录页面" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>更细的统计视角</strong><br/>可以继续看本地/云端占比、连续使用天数等信息，帮助优化自己的工作流。</td>
    <td><strong>可搜索的历史记录</strong><br/>支持浏览过往转写内容、按来源筛选，并快速找到想复用的文本。</td>
  </tr>
</table>

## 使用技巧

- 中文用户如果倾向离线使用，优先推荐 `SenseVoice`。它对中文场景更友好，项目里也把它作为 CJK 方向的强项模型；如果你常用普通话、繁体中文、粤语，通常值得先试它。
- 英文以及其他多语言用户，优先推荐 `Whisper`。它覆盖语言更多，模型档位也更完整，适合英文和跨语言输入场景。
- 如果你更看重完全本地、足够稳定的体验，先把本地模型下载好，再只在特定任务里开启云服务，这样更省心。
- 如果你已经有自己的 AI 服务订阅，可以直接去 `Cloud Services` 里配置自己的 `API Key`，按需开启 `Cloud STT` 和 `Cloud Polish`。
- 口语内容很多时，先直接转写，再套用 `Remove Fillers` 或 `Make Concise`，通常比一开始就追求“说得很标准”更高效。
- 专业术语较多时，建议提前设置输出语言、领域、子领域和 glossary，识别结果会更稳。
- 胶囊悬浮窗建议放在你视线边缘但不挡内容的位置；如果你是重度用户，常显模式通常更顺手。

## 许可证

AriaType 使用 [AGPL-3.0](LICENSE) 开源协议。

- 你可以在遵守 AGPL-3.0 条款的前提下使用、修改和分发本项目。
- 如果你需要了解完整的授权和义务范围，请直接阅读仓库中的 `LICENSE` 文件。
