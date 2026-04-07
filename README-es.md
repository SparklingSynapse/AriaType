<div align="center">
<img src="./assets/showcase.png" alt="Demostración de AriaType" width="100%"/>

<br/><br/>

### AriaType

Escritura por voz en macOS. Mantén pulsada una hotkey, habla, suelta y el texto entra en la app actual.

[English](README.md) | [简体中文](README-cn.md) | [日本語](README-ja.md) | [한국어](README-ko.md) | Español

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-pink)](https://github.com/SparklingSynapse/AriaType/releases)
[![Version](https://img.shields.io/badge/version-0.1.0--beta.8-orange)](https://github.com/SparklingSynapse/AriaType/releases)

[Descargar](https://github.com/SparklingSynapse/AriaType/releases) • [Docs](docs/README.md) • [Discusiones](https://github.com/SparklingSynapse/AriaType/discussions) • [Web](https://ariatype.com)

</div>

---

## Qué es

AriaType es una app de escritorio para escribir con la voz.

Se queda en segundo plano y solo aparece cuando la necesitas. Mantén pulsada una tecla rápida, habla con naturalidad y suelta. AriaType transcribe tu voz y escribe en la app activa, como VS Code, Slack, Notion o el navegador.

## Por qué usarlo

- Local-first: el reconocimiento de voz y la limpieza del texto se ejecutan en tu equipo por defecto.
- Privado: tus datos de voz no tienen que salir del ordenador.
- Simple: mantén `Shift+Space`, habla y suelta.
- Útil: funciona en distintas apps y soporta más de 100 idiomas.
- Flexible: puedes ajustar velocidad o precisión, cambiar hotkeys y reescribir texto cuando haga falta.

## Inicio rápido

### Instalar

- macOS (Apple Silicon): descarga el último [.dmg](https://github.com/SparklingSynapse/AriaType/releases), arrastra AriaType a Applications y ábrelo.
- Windows: el soporte sigue en desarrollo.

### Primer uso

1. Concede permisos de micrófono y Accesibilidad.
2. Descarga un modelo de voz. `Base` es un buen punto de partida.
3. Elige un idioma o deja activada la detección automática.
4. Abre cualquier editor y pruébalo.

## Cómo funciona

1. Mantén pulsada tu hotkey. Por defecto es `Shift+Space`.
2. Habla.
3. Suelta para insertar el texto.

Si quieres, AriaType también puede limpiar muletillas, puntuación y gramática antes de escribir.

## Requisitos del sistema

- macOS 12 o posterior
- Mac con Apple Silicon
- 8 GB de RAM como mínimo, 16 GB recomendados
- 2-5 GB libres para los modelos

## Para desarrolladores

Este repo es un monorepo:

- `apps/desktop`: la app de escritorio con Tauri
- `packages/website`: el sitio web
- `packages/shared`: tipos y constantes compartidos en TypeScript

### Puesta en marcha

```bash
pnpm install
pnpm tauri:dev
pnpm --filter @ariatype/website dev
```

### Empieza aquí

- [`AGENTS.md`](AGENTS.md): flujo de trabajo, comandos de verificación y reglas del repo
- [`docs/README.md`](docs/README.md): índice de documentación
- [`apps/desktop/CONTRIBUTING.md`](apps/desktop/CONTRIBUTING.md): guía de la app de escritorio
- [`packages/website/CONTRIBUTING.md`](packages/website/CONTRIBUTING.md): guía del sitio web

## Comunidad

- Bugs y peticiones de funciones: [GitHub Issues](https://github.com/SparklingSynapse/AriaType/issues)
- Preguntas y discusión: [GitHub Discussions](https://github.com/SparklingSynapse/AriaType/discussions)

## Licencia

AriaType se distribuye bajo [AGPL-3.0](LICENSE).
