<div align="center">
<img src="./assets/showcase.png" alt="AriaType 쇼케이스" width="100%"/>

<br/><br/>

### AriaType

macOS용 음성 입력 도구입니다. 단축키를 누른 채 말하고 놓으면, 텍스트가 현재 앱에 들어갑니다.

[English](README.md) | [简体中文](README-cn.md) | [日本語](README-ja.md) | 한국어 | [Español](README-es.md)

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-pink)](https://github.com/SparklingSynapse/AriaType/releases)
[![Version](https://img.shields.io/badge/version-0.1.0--beta.8-orange)](https://github.com/SparklingSynapse/AriaType/releases)

[다운로드](https://github.com/SparklingSynapse/AriaType/releases) • [문서](docs/README.md) • [토론](https://github.com/SparklingSynapse/AriaType/discussions) • [웹사이트](https://ariatype.com)

</div>

---

## 무엇인가요

AriaType는 데스크톱용 음성 입력 앱입니다.

백그라운드에 두고 필요할 때만 쓰면 됩니다. 단축키를 누른 채 자연스럽게 말하고 손을 떼면, 음성이 텍스트로 바뀌어 VS Code, Slack, Notion, 브라우저 같은 현재 앱에 바로 입력됩니다.

## 왜 쓰나요

- 로컬 우선: 음성 인식과 텍스트 정리가 기본적으로 내 기기에서 돌아갑니다.
- 프라이버시: 음성 데이터를 밖으로 보낼 필요가 없습니다.
- 단순함: `Shift+Space` 누르기, 말하기, 놓기.
- 실용성: 여러 앱에서 쓸 수 있고 100개가 넘는 언어를 지원합니다.
- 조절 가능: 속도와 정확도, 단축키, 리라이트 동작을 바꿀 수 있습니다.

## 빠른 시작

### 설치

- macOS (Apple Silicon): 최신 [.dmg](https://github.com/SparklingSynapse/AriaType/releases)를 내려받아 Applications로 옮긴 뒤 실행합니다.
- Windows: 아직 개발 중입니다.

### 처음 실행할 때

1. 마이크와 손쉬운 사용 권한을 허용합니다.
2. 음성 모델을 내려받습니다. 기본값으로는 `Base`를 추천합니다.
3. 언어를 고르거나 자동 감지를 켭니다.
4. 편집기를 열고 바로 써 봅니다.

## 사용 방법

1. 단축키를 누릅니다. 기본값은 `Shift+Space`입니다.
2. 말합니다.
3. 손을 떼면 텍스트가 입력됩니다.

원하면 말버릇, 구두점, 문법도 정리한 뒤 입력할 수 있습니다.

## 시스템 요구 사항

- macOS 12 이상
- Apple Silicon Mac
- RAM 8 GB 이상, 권장 16 GB
- 모델용 여유 저장 공간 2-5 GB

## 개발자용 안내

이 저장소는 monorepo입니다.

- `apps/desktop`: Tauri 데스크톱 앱
- `packages/website`: 공식 웹사이트
- `packages/shared`: 공용 TypeScript 타입과 상수

### 시작하기

```bash
pnpm install
pnpm tauri:dev
pnpm --filter @ariatype/website dev
```

### 먼저 볼 문서

- [`AGENTS.md`](AGENTS.md): 작업 방식, 검증 명령, 저장소 규칙
- [`docs/README.md`](docs/README.md): 문서 입구
- [`apps/desktop/CONTRIBUTING.md`](apps/desktop/CONTRIBUTING.md): 데스크톱 앱 개발 가이드
- [`packages/website/CONTRIBUTING.md`](packages/website/CONTRIBUTING.md): 웹사이트 개발 가이드

## 커뮤니티

- 버그 제보와 기능 요청: [GitHub Issues](https://github.com/SparklingSynapse/AriaType/issues)
- 질문과 토론: [GitHub Discussions](https://github.com/SparklingSynapse/AriaType/discussions)

## 라이선스

AriaType는 [AGPL-3.0](LICENSE) 라이선스를 사용합니다.
