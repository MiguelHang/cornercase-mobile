# 018 — Pantalla 2: chat — adjuntar imágenes con `paste-file` ⚠

- Estado: hecha, salvo la prueba en vivo (T9)
- Depende de: 017 · decisión **D1** aplicada en 003/004
- Riesgo: medio (selector de fotos en iOS desde Tauri; bytes por el puente)
- Rama: `feat/018-paste-image` · PR: `feat: attach a photo to an agent's pane`

## Objetivo
Mandar una imagen del móvil (foto o captura) al agente: cornercase la guarda en el Mac y pega su ruta en el panel.

## Criterios de aceptación
- [x] CA1 — Botón "adjuntar" que abre el selector de fotos de iOS (plugin `dialog` de Tauri o el que sirva en iOS;
  justificar) y devuelve los bytes a Rust (nunca por la red desde la WebView).
  Con el plan B por adelantado: un plugin propio, `photos`, con `PHPickerViewController`; `tauri-plugin-dialog` se
  descartó por HEIC, la copia en Caches y la ruta en la WebView (`design/architecture.md` › Native code).
- [x] CA2 — Rust reduce imágenes grandes (lado mayor ≤ 2.048 px, JPEG/PNG según origen; justificar la dependencia de
  imagen o usar el sistema) antes de mandarlas.
  Las reduce el sistema (ImageIO, en el plugin), sin dependencia nueva de Rust; Rust valida el tamaño y el nombre
  (`design/chat.md` › Attaching an image).
- [x] CA3 — `paste-file --pane P --name <nombre> -` por la cola con los bytes como fije D1; la respuesta (`path`,
  `bytes`) se muestra como chip en el chat; el agente recibe la ruta en su entrada.
  Que el agente la recibe y lee la imagen queda para la prueba en vivo (Verificación).
- [x] CA4 — Errores (demasiado grande, servidor viejo sin `paste-file`) en un aviso.
  Un servidor sin `paste-file` no llega a conectar (está en `REQUIRED`); lo que sí sale es el rechazo de un CLI más
  viejo que su servidor.
- [x] CA5 — Test del núcleo: un `paste-file` con bytes llega al `FakeTransport` en el formato de D1, en orden detrás de
  las teclas anteriores.

## Plan B
Si no hay selector de fotos usable: plugin propio mínimo en Swift con `PHPickerViewController`.

## Referencias
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents/pasting-files.md`.
- cornercase-gpui: `src/terminal/pasting.rs`, `design/connection.md` (Commands that carry bytes).
- Orca: `mobile/rpc-foundation/goldens/matrix-nativechat.image-paste-terminal.*`.

## design/ que actualiza
- `design/chat.md`: selector, reducción de tamaño, límites.
- `design/bridge.md`: cómo viajan los bytes (si D1 se ajusta durante la tarea).

## Verificación
En un iPhone real: adjuntar una captura a Claude Code, que la lee (criterio de éxito 2, parte imagen).
