<div align="center">
<img src="./assets/showcase.png" alt="AriaType ショーケース" width="100%"/>

<br/><br/>

### AriaType

AriaType - オープンソースの AI 音声入力 | タイピングを置き換えるパワフルな選択肢

[English](README.md) | [简体中文](README-cn.md) | 日本語 | [한국어](README-ko.md) | [Español](README-es.md)

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE) [![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-pink)](https://github.com/SparklingSynapse/AriaType/releases) [![Version](https://img.shields.io/badge/version-0.1.0--beta.8-orange)](https://github.com/SparklingSynapse/AriaType/releases)

[ダウンロード](https://github.com/SparklingSynapse/AriaType/releases) • [ドキュメント](docs/README.md) • [ディスカッション](https://github.com/SparklingSynapse/AriaType/discussions) • [Webサイト](https://ariatype.com)

</div>

---

## これは何か

AriaType は、macOS 向けのローカル優先な音声入力アプリです。

バックグラウンドで常駐し、入力したいときだけ使えます。グローバルホットキーを押しながら自然に話して、離すだけ。話した内容がそのまま今使っているアプリに文字として入ります。ドキュメント作成、チャット返信、メモ、コーディング中の補助など、「打つより話したほうが速い」場面で毎日使える AI 音声キーボードです。

## 主な機能と強み

- 🎙 グローバルホットキー入力: デフォルトは `Shift+Space`。押して話し、離すだけで入力できます。
- ↔️ アプリをまたいでそのまま入力: VS Code、Slack、Notion、ブラウザなど、今アクティブなアプリに直接テキストを送れます。
- 🔒 ローカル優先でプライベート: 音声認識もテキスト整形も、標準では手元のマシンで動きます。
- ⚡ 2 つのローカル STT エンジン: `Whisper` と `SenseVoice` を、言語や速度、精度に応じて使い分けられます。
- 🌍 100 以上の言語に対応: 自動判定にも、出力言語の手動指定にも対応しています。
- 🇨🇳 中国語・CJK に強い: `SenseVoice` は中国語、繁体字、広東語、CJK 中心の利用に特に相性が良いです。
- ✨ 文字起こしだけで終わらない: 句読点の補完、フィラー除去、語調の整理、表現の圧縮までまとめて行えます。
- 🧩 テンプレートで整文: `Remove Fillers`、`Formal Style`、`Make Concise`、`Agent Prompt` に加え、自分用テンプレートも作れます。
- ☁️ 必要なときだけクラウド強化: `Cloud Services` で `Cloud STT` と `Cloud Polish` を個別に有効化できます。
- 📡 ストリーミング中間結果: 対応するクラウド STT では、話し終える前から部分結果を受け取れます。
- 🧠 分野設定と用語集: ドメイン、サブドメイン、初期プロンプト、用語集で専門用語の認識を強化できます。
- 🧭 言語ベースのモデル推薦: 使う言語に合わせて、より向いたモデル候補を提案できます。
- 📍 常に見えるカプセル UI: 録音、文字起こし、整文、音量の状態がリアルタイムで分かります。
- ⚙️ カプセルの表示方法を調整可能: 常時表示、録音時のみ表示、非表示、位置プリセットに対応しています。
- 🎛 音声前処理を調整可能: ノイズ除去や無音トリミングで、部屋やマイクに合わせた調整ができます。
- 📝 テキスト入力が安定: まずはキーボード風に入力し、必要ならクリップボード貼り付けに切り替え、内容も復元します。
- 🔎 ローカル履歴と検索: 文字起こし結果を保存し、あとから検索や再利用ができます。
- 📊 利用ダッシュボード: 利用回数、処理時間、ローカル/クラウド比率、継続利用日数などを確認できます。
- ⬇️ モデル管理: ローカルモデルのダウンロード、削除、状態確認、進捗表示に対応しています。
- 🎨 デスクトップ向けの使いやすさ: テーマ切り替え、起動時自動実行、ホットキー変更、押して録音/トグル録音に対応しています。

## スクリーンショット

<table>
  <tr>
    <td width="50%"><img src="./assets/features/homepage-light.png" alt="AriaType ホーム画面 ライトテーマ" width="100%"/></td>
    <td width="50%"><img src="./assets/features/homepage-dark.png" alt="AriaType ホーム画面 ダークテーマ" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>ホーム画面（ライト）</strong><br/>主要な設定、モデル状態、最近の利用状況をひと目で確認できます。</td>
    <td><strong>ホーム画面（ダーク）</strong><br/>長時間の作業にもなじみやすいダークテーマです。</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/hotkey.png" alt="ホットキーと録音モード設定" width="100%"/></td>
    <td width="50%"><img src="./assets/features/general-vad.png" alt="ノイズ除去と無音トリミング設定" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>ホットキーと録音モード</strong><br/>ショートカットの変更や、押して録音 / トグル録音の切り替えができます。</td>
    <td><strong>音声前処理</strong><br/>ノイズ除去と無音トリミングを調整して、環境に合わせた最適化ができます。</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/private-model-stt.png" alt="ローカル STT モデル管理" width="100%"/></td>
    <td width="50%"><img src="./assets/features/private-model-polish.png" alt="ローカル整文モデル管理" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>ローカル STT モデル</strong><br/>`Whisper` と `SenseVoice` のモデルをダウンロードして、オフラインで文字起こしできます。</td>
    <td><strong>ローカル整文モデル</strong><br/>`Qwen`、`LFM`、`Gemma` を使って、ローカルで整文や書き換えができます。</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/cloud-service-stt.png" alt="Cloud STT 設定画面" width="100%"/></td>
    <td width="50%"><img src="./assets/features/cloud-service-polish.png" alt="Cloud Polish 設定画面" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>Cloud STT</strong><br/>自分の API Key を使って、必要な場面だけクラウド文字起こしを有効化できます。</td>
    <td><strong>Cloud Polish</strong><br/>自分のプロバイダーをつないで、より強力な整文・書き換えを使えます。</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/polish-template.png" alt="整文テンプレート管理" width="100%"/></td>
    <td width="50%"><img src="./assets/features/home-dashboard.png" alt="利用ダッシュボード" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>整文テンプレート</strong><br/>標準テンプレートから始めることも、自分専用テンプレートを作ることもできます。</td>
    <td><strong>利用ダッシュボード</strong><br/>利用頻度や処理時間を見ながら、音声入力を習慣化しやすくなります。</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/home-dashboard-2.png" alt="ダッシュボード詳細統計" width="100%"/></td>
    <td width="50%"><img src="./assets/features/history.png" alt="検索できる履歴画面" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>より細かな統計</strong><br/>ローカル/クラウド比率や継続利用日数なども確認できます。</td>
    <td><strong>検索できる履歴</strong><br/>過去の文字起こしを一覧し、ソース別に絞り込み、再利用したい文をすばやく見つけられます。</td>
  </tr>
</table>

## 使いこなしのヒント

- オフライン中心で中国語をよく使うなら、まずは `SenseVoice` から試すのがおすすめです。中国語、繁体字、広東語、CJK 寄りの用途に特に向いています。
- 英語やそれ以外の多言語用途が中心なら、まずは `Whisper` がおすすめです。対応言語が広く、モデルサイズの選択肢も豊富です。
- まずはローカルモデルを入れて安定した環境を作り、必要な場面だけクラウド機能を有効にすると、使い勝手のバランスが取りやすくなります。
- すでに自分の AI サービス契約があるなら、`Cloud Services` に `API Key` を追加して、`Cloud STT` と `Cloud Polish` を必要に応じて有効化すると便利です。
- 話し言葉が多いときは、最初から完璧に話そうとするより、先に文字起こししてから `Remove Fillers` や `Make Concise` をかけるほうが楽です。
- 専門用語が多い分野では、出力言語、ドメイン、サブドメイン、用語集を先に設定しておくと結果が安定しやすくなります。
- カプセル UI は視界に入るけれど邪魔にならない位置に置くと使いやすく、ヘビーユーザーなら常時表示が合うことが多いです。

## ライセンス

AriaType は [AGPL-3.0](LICENSE) で公開しています。

- AGPL-3.0 の条件に従って、利用、改変、再配布ができます。
- 詳細な条件と義務は `LICENSE` を確認してください。
