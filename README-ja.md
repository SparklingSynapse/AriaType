<div align="center">
<img src="./assets/showcase.jpg" alt="AriaType ショーケース" width="100%"/>

<br/><br/>

<img src="./assets/ariatype.png" alt="AriaType ロゴ" height="128" />


### プライベートでローカルな音声キーボード

**押して話す。離して入力。ローカル優先。プライバシー優先。**

[English](README.md) | [简体中文](README-cn.md) | 日本語 | [한국어](README-ko.md) | [Español](README-es.md)

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-pink)](https://github.com/SparklingSynapse/AriaType/releases)
[![Version](https://img.shields.io/badge/version-0.1.0--beta.7-orange)](https://github.com/SparklingSynapse/AriaType/releases)

[ダウンロード](https://github.com/SparklingSynapse/AriaType/releases) • [ドキュメント](#クイックスタート) • [コミュニティ](https://github.com/SparklingSynapse/AriaType/discussions) • [Webサイト](https://ariatype.com)

</div>

---

## ✨ AriaType とは？

AriaType は、バックグラウンドで静かに動作する **ローカルファーストの音声キーボード** です。入力したいときはショートカットキー（デフォルトは `Shift+Space`）を押し続けて自然に話し、離すだけ。AriaType が音声を即座に文字起こしし、VS Code、Slack、Notion、ブラウザなど、アクティブなアプリへそのまま入力します。

音声認識とテキスト整形には、**厳選・最適化されたローカル AI モデル** を使用します。無作為なモデル選定ではなく、目的に合った最適なツールで構成しています。

**音声データはデバイス外に出ません。100% プライベート、100% ローカル。**

---

## 🚀 クイックスタート

### インストール

**macOS（Apple Silicon）**

1. 最新の [.dmg ファイル](https://github.com/SparklingSynapse/AriaType/releases)をダウンロード
2. .dmg を開き、AriaType を Applications にドラッグ
3. Applications から AriaType を起動

**Windows** 🚧 開発中

Windows 対応は現在開発中です。[このリポジトリをウォッチ](https://github.com/SparklingSynapse/AriaType)するか、[ディスカッションに参加](https://github.com/SparklingSynapse/AriaType/discussions)して最新情報を確認してください。

### 初回セットアップ

1. **権限の許可**：プロンプトに従い、マイクとアクセシビリティ権限を許可
2. **モデルのダウンロード**：速度と精度のバランスが良い **Base** を選択
3. **言語設定**：自動検出でも十分ですが、主要言語を選択することも可能
4. **試してみる**：任意のエディタで `Shift+Space` を押しながら「Hello world」と話して離す

### 基本操作

```
1. 押す → Shift+Space（またはカスタムホットキー）
2. 話す → 入力したい内容を話す
3. 離す → テキストが即座に入力される
```

---

## 🎯 主な機能

### 🔒 プライバシー最優先

音声データは **決してコンピュータ外へ送信されません**。音声認識とテキスト整形は、**厳選・最適化されたモデル** によりすべてローカルで処理されます。クラウドなし、サーバーなし、データ収集なし（匿名解析にオプトインした場合を除く）。

### 🎙️ インテリジェントなノイズ低減

環境ノイズを自動で抑制する 3 つのモード：

- **Auto**：ノイズレベルを検出して自動調整
- **Always On**：最大限のノイズ抑制
- **Off**：生の音声入力

### ✨ AI による文章整形

**厳選されたローカル AI モデル** で、話し言葉を自然な文章に整えます：

- つなぎ言葉（"um"、"uh"、"like" など）の除去
- 文法と句読点の修正
- 自然なフォーマット
- 全処理がオンデバイスで完結し、プライバシーを最大化

### 🌍 100 以上の言語

以下を含む多言語に対応：

- 英語、中国語（簡体字/繁体字）
- 日本語、韓国語、スペイン語、フランス語
- ドイツ語、イタリア語、ポルトガル語、ロシア語
- そのほか 90+ 言語

### ⚡ スマート機能

- **グローバルホットキー**：どのアプリでも使用可能
- **Smart Pill**：音量レベルを表示する最小のフローティング UI
- **速度/精度モード**：重視したい点に合わせて最適化
- **ワンタップ書き換え**：フォーマル、簡潔、文法修正を即時適用
- **カスタマイズ**：ホットキー、言語、挙動を調整可能

---

## 📋 動作環境

- **OS**：macOS 12.0（Monterey）以降
- **チップ**：Apple Silicon（M1、M2、M3、M4）
- **メモリ**：8GB 以上（推奨 16GB）
- **ストレージ**：モデル用に 2～5GB

---

## 🛠️ 高度な設定

### ホットキーのカスタマイズ

Settings → Hotkeys からトリガーキーの組み合わせを変更できます。

### モデル選択

AriaType は音声認識と文章整形の両方で、**厳選・最適化されたモデル** を使用します：

**音声認識モデル（Whisper ベース）**：

- **Tiny**：最速、精度低め（~75MB）
- **Base**：バランス型（推奨）（~150MB）
- **Small**：より高精度（~500MB）
- **Medium**：最高精度（~1.5GB）

**テキスト整形**：文法修正と自然なフォーマットに最適化されたローカル LLM によって実行されます。

モデルはすべてオンデバイスで動作し、ダウンロード後はインターネット不要です。

### 言語設定

- **自動検出**：話している言語を自動判定
- **固定言語**：指定言語に固定して精度を改善

---

## 💬 コミュニティとサポート

- **Issues**：不具合報告や機能要望は [GitHub Issues](https://github.com/SparklingSynapse/AriaType/issues)
- **Discussions**：コミュニティ参加は [GitHub Discussions](https://github.com/SparklingSynapse/AriaType/discussions)
- **Webサイト**：[ariatype.com](https://ariatype.com)

---

## 🤝 コントリビュート

貢献歓迎です。例えば：

- 🐛 バグ報告
- 💡 機能提案
- 📝 ドキュメント改善
- 🔧 コード貢献

[GitHub](https://github.com/SparklingSynapse/AriaType) で issue または pull request を作成してください。

---

## 📄 ライセンス

本プロジェクトは **GNU Affero General Public License v3.0（AGPL-3.0）** で提供されます。

つまり：

- ✅ 利用・改変・再配布が可能
- ✅ ずっとオープンソース
- ⚠️ 改変して配布する場合は変更点を公開する必要があります
- ⚠️ 改変版をサービスとして運用する場合もソース公開が必要です

詳細は [LICENSE](LICENSE) を参照してください。

---

## 🌟 プロジェクトを支援する

AriaType が役に立ったら：

- ⭐ このリポジトリに Star
- 🐦 共有して広める
- 💬 コミュニティの議論に参加
- 🐛 バグ報告で改善に協力

---

<div align="center">

**Made with ❤️ for developers, writers, and anyone who thinks faster than they type**

[今すぐダウンロード](https://github.com/SparklingSynapse/AriaType/releases) • [始める](#クイックスタート) • [コミュニティに参加](https://github.com/SparklingSynapse/AriaType/discussions)

</div>
