<div align="center">
<img src="./assets/showcase.png" alt="AriaType 쇼케이스" width="100%"/>

<br/><br/>

### AriaType

AriaType - 오픈소스 AI 음성 텍스트 입력 | Typeless의 강력한 대안

[English](README.md) | [简体中文](README-cn.md) | [日本語](README-ja.md) | 한국어 | [Español](README-es.md)

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE) [![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-pink)](https://github.com/SparklingSynapse/AriaType/releases) [![Version](https://img.shields.io/badge/version-0.1.0--beta.8-orange)](https://github.com/SparklingSynapse/AriaType/releases)

[다운로드](https://github.com/SparklingSynapse/AriaType/releases) • [문서](docs/README.md) • [토론](https://github.com/SparklingSynapse/AriaType/discussions) • [웹사이트](https://ariatype.com)

</div>

---

## 무엇인가요

AriaType는 macOS용 로컬 우선 음성 입력 앱입니다.

백그라운드에서 대기하다가, 입력이 필요할 때만 꺼내 쓰면 됩니다. 전역 단축키를 누른 채 자연스럽게 말하고 손을 떼면, 말한 내용이 현재 앱에 바로 텍스트로 들어갑니다. 문서 작성, 채팅 답장, 메모, 코딩 보조처럼 “타이핑보다 말이 더 빠른” 순간에 매일 쓸 수 있는 AI 음성 키보드에 가깝습니다.

## 핵심 기능과 장점

- 🎙 전역 단축키 음성 입력: 기본값은 `Shift+Space`이며, 누르고 말한 뒤 놓는 흐름이 아주 빠릅니다.
- ↔️ 앱을 넘나드는 직접 입력: VS Code, Slack, Notion, 브라우저 등 현재 활성 앱에 바로 텍스트를 넣을 수 있습니다.
- 🔒 로컬 우선과 프라이버시: 음성 인식과 텍스트 정리가 기본적으로 내 기기에서 실행됩니다.
- ⚡ 2개의 로컬 STT 엔진: `Whisper`와 `SenseVoice`를 언어, 속도, 정확도에 맞게 골라 쓸 수 있습니다.
- 🌍 100개 이상의 언어 지원: 자동 감지와 출력 언어 직접 지정 모두 지원합니다.
- 🇨🇳 중국어와 CJK에 강함: `SenseVoice`는 중국어, 번체 중국어, 광둥어, CJK 중심 사용에 특히 잘 맞습니다.
- ✨ 단순한 받아쓰기 그 이상: 구두점 보정, 군더더기 제거, 어조 정리, 표현 압축까지 한 번에 처리할 수 있습니다.
- 🧩 템플릿 기반 polish: `Remove Fillers`, `Formal Style`, `Make Concise`, `Agent Prompt` 기본 템플릿에 더해 사용자 템플릿도 만들 수 있습니다.
- ☁️ 필요할 때만 클라우드 강화: `Cloud Services`에서 `Cloud STT`와 `Cloud Polish`를 각각 켤 수 있습니다.
- 📡 스트리밍 중간 결과: 지원되는 클라우드 STT는 말이 끝나기 전에도 부분 결과를 계속 보여줍니다.
- 🧠 도메인과 용어집 지원: 도메인, 서브도메인, 초기 프롬프트, 용어집 설정으로 전문 용어 인식을 더 안정적으로 만들 수 있습니다.
- 🧭 언어 기반 모델 추천: 사용 언어에 맞춰 더 잘 맞는 모델을 추천받을 수 있습니다.
- 📍 항상 위에 떠 있는 캡슐 UI: 녹음, 전사, 문장 다듬기, 음량 상태를 실시간으로 확인할 수 있습니다.
- ⚙️ 캡슐 표시 방식과 위치 조절: 항상 표시, 녹음 중만 표시, 숨김, 위치 프리셋을 지원합니다.
- 🎛 조절 가능한 오디오 전처리: 노이즈 제거와 무음 잘라내기로 환경에 맞는 세팅을 만들 수 있습니다.
- 📝 안정적인 텍스트 주입: 키보드 방식 입력을 우선하고, 필요하면 클립보드 붙여넣기로 전환한 뒤 클립보드도 복원합니다.
- 🔎 로컬 기록과 검색: 전사 결과를 저장하고 나중에 검색하거나 재사용할 수 있습니다.
- 📊 사용 대시보드: 입력 횟수, 처리 시간, 로컬/클라우드 비중, 연속 사용 일수 등을 볼 수 있습니다.
- ⬇️ 모델 관리: 로컬 모델 다운로드, 삭제, 상태 확인, 진행률 표시를 지원합니다.
- 🎨 데스크톱 사용성: 테마 전환, 로그인 시 자동 실행, 단축키 변경, 누르고 녹음/토글 녹음까지 갖추고 있습니다.

## 스크린샷

<table>
  <tr>
    <td width="50%"><img src="./assets/features/homepage-light.png" alt="AriaType 홈 화면 라이트 테마" width="100%"/></td>
    <td width="50%"><img src="./assets/features/homepage-dark.png" alt="AriaType 홈 화면 다크 테마" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>홈 화면, 라이트 테마</strong><br/>핵심 설정, 모델 상태, 최근 사용 흐름을 한눈에 볼 수 있습니다.</td>
    <td><strong>홈 화면, 다크 테마</strong><br/>오래 작업할 때 편안한 다크 환경에서도 같은 흐름으로 사용할 수 있습니다.</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/hotkey.png" alt="단축키와 녹음 모드 설정" width="100%"/></td>
    <td width="50%"><img src="./assets/features/general-vad.png" alt="노이즈 제거와 무음 잘라내기 설정" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>단축키와 녹음 모드</strong><br/>단축키를 바꾸고 누르고 녹음 / 토글 녹음 모드를 선택할 수 있습니다.</td>
    <td><strong>오디오 전처리</strong><br/>노이즈 제거와 무음 잘라내기를 조정해 방 환경과 마이크에 맞출 수 있습니다.</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/private-model-stt.png" alt="로컬 STT 모델 관리" width="100%"/></td>
    <td width="50%"><img src="./assets/features/private-model-polish.png" alt="로컬 polish 모델 관리" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>로컬 STT 모델</strong><br/>`Whisper`와 `SenseVoice` 모델을 내려받아 오프라인 전사를 구성할 수 있습니다.</td>
    <td><strong>로컬 Polish 모델</strong><br/>`Qwen`, `LFM`, `Gemma` 기반으로 로컬에서 문장을 다듬고 다시 쓸 수 있습니다.</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/cloud-service-stt.png" alt="Cloud STT 설정 화면" width="100%"/></td>
    <td width="50%"><img src="./assets/features/cloud-service-polish.png" alt="Cloud Polish 설정 화면" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>Cloud STT</strong><br/>자신의 API Key를 넣고, 필요할 때만 클라우드 전사를 켤 수 있습니다.</td>
    <td><strong>Cloud Polish</strong><br/>자신의 제공자를 연결해 더 강한 문장 정리와 리라이트를 활용할 수 있습니다.</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/polish-template.png" alt="polish 템플릿 관리" width="100%"/></td>
    <td width="50%"><img src="./assets/features/home-dashboard.png" alt="사용 대시보드" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>Polish 템플릿</strong><br/>기본 템플릿으로 시작하거나, 자주 쓰는 글쓰기 작업용 템플릿을 직접 만들 수 있습니다.</td>
    <td><strong>사용 대시보드</strong><br/>얼마나 자주 쓰는지, 처리 속도는 어떤지 보면서 음성 입력 습관을 만들 수 있습니다.</td>
  </tr>
  <tr>
    <td width="50%"><img src="./assets/features/home-dashboard-2.png" alt="대시보드 상세 통계" width="100%"/></td>
    <td width="50%"><img src="./assets/features/history.png" alt="검색 가능한 기록 화면" width="100%"/></td>
  </tr>
  <tr>
    <td><strong>더 자세한 통계</strong><br/>로컬/클라우드 비율, 연속 사용 일수 같은 세부 지표도 확인할 수 있습니다.</td>
    <td><strong>검색 가능한 기록</strong><br/>지난 전사 결과를 둘러보고, 소스별로 필터링하고, 다시 쓰고 싶은 문장을 빠르게 찾을 수 있습니다.</td>
  </tr>
</table>

## 사용 팁

- 오프라인 위주로 쓰고 중국어를 많이 말한다면, 먼저 `SenseVoice`부터 써 보는 것을 권합니다. 중국어, 번체 중국어, 광둥어, CJK 중심 사용에 특히 잘 맞습니다.
- 영어나 더 넓은 다국어 환경이 중심이라면 `Whisper`부터 시작하는 편이 좋습니다. 지원 언어가 넓고 모델 선택 폭도 큽니다.
- 가장 안정적인 환경을 원하면 먼저 로컬 모델을 내려받아 두고, 꼭 필요한 작업에서만 클라우드 기능을 켜는 방식이 편합니다.
- 이미 사용하는 AI 서비스가 있다면 `Cloud Services`에서 `API Key`를 넣고 `Cloud STT`와 `Cloud Polish`를 필요에 따라 활성화하면 됩니다.
- 말할 때 군더더기가 많다면 처음부터 완벽하게 말하려 하기보다, 먼저 전사한 다음 `Remove Fillers`나 `Make Concise`를 적용하는 편이 더 효율적입니다.
- 전문 용어가 많은 분야라면 출력 언어, 도메인, 서브도메인, glossary를 먼저 잡아 두는 편이 결과가 더 안정적입니다.
- 캡슐 UI는 눈에는 들어오되 작업을 가리지 않는 위치에 두는 것이 좋고, 자주 쓰는 사람은 항상 표시 모드를 선호하는 경우가 많습니다.

## 라이선스

AriaType는 [AGPL-3.0](LICENSE) 라이선스로 배포됩니다.

- AGPL-3.0 조건에 따라 사용, 수정, 재배포할 수 있습니다.
- 자세한 법적 조건과 의무는 `LICENSE`를 확인해 주세요.
