<div align="center">
<img src="./assets/showcase.jpg" alt="Demostración de AriaType" width="100%"/>

<br/><br/>

<img src="./assets/ariatype.png" alt="Logo de AriaType"  height="128" />


### Tu teclado de voz privado y local

**Mantén pulsado para hablar. Suelta para escribir. Local primero. Privacidad primero.**

[English](README.md) | [简体中文](README-cn.md) | [日本語](README-ja.md) | [한국어](README-ko.md) | Español

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-pink)](https://github.com/SparklingSynapse/AriaType/releases)
[![Version](https://img.shields.io/badge/version-0.1.0--beta.8-orange)](https://github.com/SparklingSynapse/AriaType/releases)

[Descargar](https://github.com/SparklingSynapse/AriaType/releases) • [Documentación](#inicio-rápido) • [Comunidad](https://github.com/SparklingSynapse/AriaType/discussions) • [Sitio web](https://ariatype.com)

</div>

---

## ✨ ¿Qué es AriaType?

AriaType es un **teclado de voz local-first** que se ejecuta silenciosamente en segundo plano. Cuando quieras escribir, mantén pulsada una tecla de acceso rápido (por defecto `Shift+Space`), habla con naturalidad y suelta. AriaType transcribe al instante y escribe el texto en cualquier aplicación activa—ya sea VS Code, Slack, Notion o el navegador.

Está impulsado por **modelos de IA locales cuidadosamente seleccionados y optimizados** para reconocimiento de voz y pulido de texto—sin elecciones aleatorias de modelos: solo las mejores herramientas para el trabajo.

**Tus datos de voz nunca salen del dispositivo. 100% privado. 100% local.**

---

## 🚀 Inicio rápido

### Instalación

**macOS (Apple Silicon)**

1. Descarga el último [archivo .dmg](https://github.com/SparklingSynapse/AriaType/releases)
2. Abre el .dmg y arrastra AriaType a Applications
3. Inicia AriaType desde Applications

**Windows** 🚧 En progreso

La compatibilidad con Windows está en desarrollo. [Sigue este repositorio](https://github.com/SparklingSynapse/AriaType) o [únete a las discusiones](https://github.com/SparklingSynapse/AriaType/discussions) para novedades.

### Configuración inicial

1. **Concede permisos**: permite acceso al micrófono y a Accesibilidad cuando se solicite
2. **Descarga un modelo**: elige el modelo **Base** para un equilibrio entre velocidad y precisión
3. **Configura tu idioma**: la detección automática funciona muy bien, o selecciona tu idioma principal
4. **Pruébalo**: abre cualquier editor, mantén `Shift+Space` y di “Hello world”

### Uso básico

```
1. Mantén → Shift+Space (o tu hotkey personalizada)
2. Habla → Di lo que quieras escribir
3. Suelta → El texto aparece al instante
```

---

## 🎯 Funciones clave

### 🔒 Privacidad primero

Tus datos de voz **nunca salen del ordenador**. Todo el procesamiento sucede localmente usando **modelos cuidadosamente seleccionados y optimizados** para reconocimiento de voz y pulido de texto. Sin nube. Sin servidores. Sin recolección de datos (a menos que actives analíticas anónimas).

### 🎙️ Reducción inteligente de ruido

Filtra automáticamente el ruido de fondo con tres modos:

- **Auto**: detecta y se adapta al nivel de ruido
- **Always On**: máxima supresión de ruido
- **Off**: entrada de audio sin procesar

### ✨ Pulido con IA

Limpia automáticamente tu discurso con **modelos de IA locales curados**:

- elimina muletillas (“um”, “uh”, “like”)
- corrige gramática y puntuación
- formatea el texto de manera natural
- todo el procesamiento es en el dispositivo para máxima privacidad

### 🌍 100+ idiomas

Compatibilidad completa con:

- inglés, chino (simplificado/tradicional)
- japonés, coreano, español, francés
- alemán, italiano, portugués, ruso
- y 90+ más

### ⚡ Funciones inteligentes

- **Hotkey global**: funciona en cualquier aplicación
- **Smart Pill**: indicador flotante mínimo con niveles de audio
- **Modos velocidad/precisión**: optimiza según lo que más te importe
- **Reescritura en un toque**: Formal, Concise o Fix Grammar al instante
- **Personalizable**: ajusta hotkeys, idiomas y comportamiento

---

## 📋 Requisitos del sistema

- **OS**: macOS 12.0 (Monterey) o posterior
- **Chip**: Apple Silicon (M1, M2, M3, M4)
- **RAM**: mínimo 8GB (recomendado 16GB)
- **Almacenamiento**: 2-5GB para modelos

---

## 🛠️ Configuración avanzada

### Hotkeys personalizadas

Ve a Settings → Hotkeys para personalizar tu combinación de teclas.

### Selección de modelos

AriaType utiliza **modelos cuidadosamente seleccionados y optimizados** tanto para voz a texto como para el pulido con IA:

**Modelos de reconocimiento de voz (basados en Whisper)**:

- **Tiny**: el más rápido, menor precisión (~75MB)
- **Base**: equilibrio (recomendado) (~150MB)
- **Small**: mayor precisión (~500MB)
- **Medium**: máxima precisión (~1.5GB)

**Pulido de texto**: impulsado por un LLM local optimizado para corrección gramatical y formato natural.

Todos los modelos se ejecutan completamente en tu dispositivo—no se requiere internet después de la descarga.

### Ajustes de idioma

- **Auto-detect**: identifica automáticamente el idioma que hablas
- **Idioma fijo**: fija un idioma específico para mejorar la precisión

---

## 💬 Comunidad y soporte

- **Issues**: reporta errores o solicita funciones en [GitHub Issues](https://github.com/SparklingSynapse/AriaType/issues)
- **Discussions**: únete a la comunidad en [GitHub Discussions](https://github.com/SparklingSynapse/AriaType/discussions)
- **Sitio web**: visita [ariatype.com](https://ariatype.com) para más información

---

## 🤝 Contribuir

¡Agradecemos las contribuciones! Por ejemplo:

- 🐛 reportes de bugs
- 💡 solicitudes de funciones
- 📝 mejoras de documentación
- 🔧 contribuciones de código

Abre un issue o pull request en [GitHub](https://github.com/SparklingSynapse/AriaType).

---

## 📄 Licencia

Licenciado bajo **GNU Affero General Public License v3.0** (AGPL-3.0).

Esto significa:

- ✅ libre para usar, modificar y distribuir
- ✅ código abierto para siempre
- ⚠️ si modificas y distribuyes, debes compartir tus cambios
- ⚠️ si ejecutas una versión modificada como servicio, debes compartir el código fuente

Consulta [LICENSE](LICENSE) para más detalles.

---

## 🌟 Apoya el proyecto

Si AriaType te ayuda a ser más productivo:

- ⭐ dale una estrella al repositorio
- 🐦 compártelo con otras personas
- 💬 únete a las discusiones de la comunidad
- 🐛 reporta bugs para ayudarnos a mejorar

---

<div align="center">

**Made with ❤️ for developers, writers, and anyone who thinks faster than they type**

[Descargar ahora](https://github.com/SparklingSynapse/AriaType/releases) • [Empezar](#inicio-rápido) • [Unirse a la comunidad](https://github.com/SparklingSynapse/AriaType/discussions)

</div>
