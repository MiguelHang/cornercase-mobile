# 063 — Pulido de la UI, el teclado que sigue al dedo en el chat y enlaces que se abren

- Estado: hecha (el teclado y los enlaces, comprobados en el iPhone del usuario el 2026-11-15; el pulido, en Chromium a
  390 × 844)
- Depende de: 058
- Rama: `clear-ui` · PR: `feat: polish the UI, let the keys follow the finger in the chat and open links`

## Objetivo
Del iPhone del usuario (2026-11-15): la interfaz pedía un repaso de contraste, densidad y movimiento; en el chat, el
teclado de iOS no bajaba con el dedo como en WhatsApp o Mensajes y sobre él iba la barra ‹ › ✓ del WKWebView, que el
chat no usa; y los enlaces de las respuestas de los agentes no se podían tocar.

## Criterios de aceptación
- [x] CA1 — Sin la barra ‹ › ✓ sobre el teclado en iOS: un plugin propio, `src-tauri/plugins/keyboard`, la quita del
  WKWebView (`design/architecture/plugins.md` › `keyboard`).
- [x] CA2 — En el chat, el teclado baja con el dedo (`keyboardDismissMode = .interactive`) y el composer y los mensajes
  bajan con él sin hueco; el hilo no se mueve mientras el teclado se va, el campo pierde el foco al irse y la posición
  del hilo se conserva al cerrarlo. Solo en el chat: la terminal y los formularios siguen como estaban
  (`design/chat.md` › The screen).
- [x] CA3 — Los enlaces `http` y `https` del Markdown se dibujan como enlaces y un toque los abre en el navegador del
  sistema con `tauri-plugin-opener`, limitado a esos dos esquemas en `capabilities/default.json`; cualquier otro enlace
  se queda en texto (`design/chat.md` › Markdown).
- [x] CA4 — Pulido de la UI: barra de conexión en dos líneas, `--muted` en vez de `--faint` donde había texto, cabecera
  compacta, `Close agent` como botón quiet, degradado primario con contraste AA, icono `image-plus` para adjuntar y el
  movimiento de la hoja, el botón pulsado y el switch (`design/look.md`, `design/look/launch.md`).
- [x] CA5 — Tests de la lógica nueva (`dropped`, `followKeys`, `fromEnd`, `links`, el plugin en Rust), `design/`
  actualizado y los checks de CI en verde.

## Decisiones
Tomadas el 2026-11-15 en la review de la rama:
- El plugin del teclado actúa solo en el chat, que lo marca en el documento (`data-keyboard="follows"`) mientras está
  abierto: es la única pantalla que se mueve con el teclado. Rechazado: toda la app, sin probar en la terminal.
- `tauri-plugin-opener` sin su script que abre los `<a target="_blank">`, `mailto:` y `tel:`, que la app no usa.
- La URL se normaliza antes de abrirla (esquema y host en minúsculas), como la compara el scope del plugin.
