<div align="center">
<img src="./assets/showcase-0.3.png" alt="Demostración de AriaType" width="100%"/>

<br/><br/>

### AriaType

AriaType - Entrada por voz con IA y codigo abierto | Una potente alternativa a Typeless

[English](README.md) | [简体中文](README-cn.md) | [日本語](README-ja.md) | [한국어](README-ko.md) | Español

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE) [![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-pink)](https://github.com/joe223/AriaType/releases) [![Windows](https://img.shields.io/badge/Windows-WIP-yellow)](https://github.com/joe223/AriaType) [![Version](https://img.shields.io/badge/version-0.3-green)](https://github.com/joe223/AriaType/releases)

[Descargar](https://github.com/joe223/AriaType/releases) • [Docs](context/README.md) • [Discusiones](https://github.com/joe223/AriaType/discussions) • [Web](https://ariatype.com)

</div>

> [!TIP]
> **Novedades de v0.3**
> - **Reintentar transcripciones fallidas** – las entradas fallidas del historial pueden reintentarse con audio guardado
> - **Cancelar con ESC** – pulsa ESC durante la grabación para cancelar sin crear entradas inválidas
> - **Grabaciones largas más estables** – problemas de truncamiento en sesiones largas corregidos
> - **Soporte Fn key** – los shortcuts personalizados ahora soportan combinaciones con Fn

---

## Qué es

AriaType es una app de dictado por voz para macOS, con un enfoque claramente local-first.

Se queda en segundo plano y aparece justo cuando la necesitas. Mantienes pulsada una hotkey global, hablas con naturalidad y sueltas. Tu voz se convierte en texto dentro de la app activa.

## Funciones principales

- ⚡️ **Rápido** – tiempo medio de transcripción bajo 500ms, acelera tu coding/writing
- 🔒 **Privacidad primero** – STT/Polish local por defecto, tu voz no sale del dispositivo
- 🎙 **Hotkey global** – pulsa `Shift+Space`, habla, suelta, texto aparece en cualquier app
- 🇨🇳 **CJK-friendly** – SenseVoice optimizado para chino, japonés, coreano
- ✨ **Polish inteligente** – elimina muletillas, corrige puntuación, limpia frases
- 🧩 **Plantillas custom** – crea tus propios estilos de Polish para tareas recurrentes
- 🌍 **100+ idiomas** – detección automática o idioma de salida manual
- ☁️ **Cloud opcional** – activa mejora cloud con tu API Key cuando lo necesites

## Consejos de uso

- Para chino/CJK, usa `SenseVoice` – mejor para mandarín, cantonés, japonés.
- Para inglés/internacionales, usa `Whisper` – cobertura más amplia.
- ¿Hablas con muletillas? Transcribe primero y luego aplica `Remove Fillers` o `Make Concise`.
- ¿Términos técnicos? Configura dominio y glosario antes.

## Plataformas

| Plataforma | Estado | Requisitos |
|------------|--------|------------|
| macOS (Apple Silicon) | ✅ Estable | macOS 12.0+, chip M-series |
| macOS (Intel) | ✅ Estable | macOS 12.0+, Intel Core i5+ |
| Windows | 🔧 WIP | Próximamente |

## Instalación y uso

Descarga desde [ariatype.com](https://ariatype.com), instala la app y autoriza permisos de micrófono y accesibilidad. No necesitas cuenta ni configuración inicial.

## Licencia

AriaType usa la licencia [AGPL-3.0](LICENSE).

- Puedes usar, modificar y distribuir libremente bajo AGPL-3.0.
- Detalles en el archivo `LICENSE`.