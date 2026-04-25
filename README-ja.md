<div align="center">
<img src="./assets/showcase-0.3.png" alt="AriaType ショーケース" width="100%"/>

<br/><br/>

### AriaType

AriaType - オープンソースの AI 音声入力 | Typeless の強力な代替案

[English](README.md) | [简体中文](README-cn.md) | 日本語 | [한국어](README-ko.md) | [Español](README-es.md)

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE) [![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-pink)](https://github.com/joe223/AriaType/releases) [![Windows](https://img.shields.io/badge/Windows-WIP-yellow)](https://github.com/joe223/AriaType) [![Version](https://img.shields.io/badge/version-0.3-green)](https://github.com/joe223/AriaType/releases)

[ダウンロード](https://github.com/joe223/AriaType/releases) • [ドキュメント](context/README.md) • [ディスカッション](https://github.com/joe223/AriaType/discussions) • [Webサイト](https://ariatype.com)

</div>

> [!TIP]
> **v0.4 の新機能**
> - **Riffモード** – 自由に話して、AIが自然な文章に整える
> - **3つのホットキーモード** – Dictate（原文）、Riff（整形）、Custom（カスタム）
> - **プロファイル別テンプレート** – 各ショートカットに独自の整形スタイルを設定

---

## これは何か

AriaType は、MacOS 向けのローカル優先な音声入力アプリです。

バックグラウンドで常駐し、入力したいときだけ使えます。グローバルホットキーを押しながら自然に話して、離すだけ。話した内容がそのまま今使っているアプリに文字として入ります。

## 主な機能

- ⚡️ **高速処理** – 平均文字起こし時間 500ms 未満、コーディング/執筆を加速
- 🔒 **プライバシー優先** – デフォルトでローカル STT/Polish、音声は端末外に出ない
- 🎙 **2つのショートカット** – `Cmd+/` で聴写（原文）、`Opt+/` で自動整形
- 🇨🇳 **CJK対応** – SenseVoice が中国語・日本語・韓国語に最適化
- ✨ **スマートな整文** – フィラー除去、句読点補完、表現整理を自動実行
- 🧩 **カスタムテンプレート** – 定型作業向けに独自の整文スタイルを作成
- 🌍 **100+ 言語** – 自動判定または出力言語を指定
- ☁️ **クラウド併用** – 必要なときだけ API Key でクラウド強化を有効化

## 使い方のヒント

- 中国語/CJK なら `SenseVoice` が最適。北京語、広東語、日本語に強い。
- 英語/国際言語なら `Whisper` を。より広い言語カバー。
- フィラーが多い？文字起こし後に `Remove Fillers` または `Make Concise` を適用。
- 専門用語がある？分野と用語集を事前に設定。

## 対応プラットフォーム

| プラットフォーム | 状態 | 要件 |
|-----------------|------|------|
| macOS (Apple Silicon) | ✅ 安定 | macOS 12.0+, M シリーズ |
| macOS (Intel) | ✅ 安定 | macOS 12.0+, Intel Core i5+ |
| Windows | 🔧 WIP | 近日公開 |

## インストールと使い方

[ariatype.com](https://ariatype.com) からダウンロード、インストール後、マイクとアクセシビリティ権限を許可すればすぐ使えます。アカウント登録不要、セットアップウィザードもなし。

## ライセンス

AriaType は [AGPL-3.0](LICENSE) ライセンスです。

- AGPL-3.0 条件下で自由に使用・修正・配布できます。
-詳細は `LICENSE` ファイルを参照。