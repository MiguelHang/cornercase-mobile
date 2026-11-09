# 021 — Pantalla 3: terminal — entrada, teclas que faltan, desplazar y teclado del WebView ⚠

- Estado: hecha, salvo la prueba en un iPhone (teclado en pantalla real, dictado, IME, pegar en release)
- Depende de: 020
- Riesgo: **alto** — teclado virtual de iOS sobre la WebView (foco, altura, IME), gestos.
- Rama: `feat/021-terminal-input` · PR: `feat: type and scroll in a live terminal from the phone`

## Objetivo
Escribir en la terminal en vivo desde el teclado del móvil, con una fila de teclas que le faltan, y desplazar el
historial con el dedo.

## Criterios de aceptación
- [x] CA1 — Un `textarea` oculto recibe el foco al tocar la terminal; lo que escribe el teclado (también dictado, IME
  y autocorrección desactivada: `autocapitalize=off`, `autocorrect=off`, `spellcheck=false`) se convierte en eventos
  de `input --follow --json` (typist de 011). Por `beforeinput`, con un espacio centinela para que borrar con el campo
  «vacío» llegue, y el foco en el `click` que sigue al toque (en `pointerup` iOS lo quitaba con su `click`). Un toque
  solo enfoca: no manda clics al programa (P2 a). Dictado e IME, sin probar: el simulador no los da.
- [x] CA2 — Fila sobre el teclado: `Esc`, `Tab`, `Ctrl` (modificador pegajoso para la siguiente tecla), `↑ ↓ ← →`,
  `|`, `~`, `/`, `-`. Mapeo en `src/lib/terminal/keys.ts` con vitest. Más `Paste` (CA5). Las teclas actúan en
  `pointerup` y cancelan su `touchend`, no en `pointerdown` con `preventDefault` como se planeó: eso cerraba el
  teclado en iOS y no cuenta como gesto para leer el portapapeles. Un `click` sin puntero (`detail` 0: VoiceOver, o
  Enter o Espacio de un teclado físico) también actúa. `Paste` se dibuja con el icono `clipboard-paste` de Lucide
  (`aria-label="Paste"`), porque la palabra no cabía en los 420 pt del iPhone Air. También el teclado físico
  (flechas, Esc, Tab, Enter, Backspace, Delete, Home, End, Page Up, Page Down y F1–F12, con Ctrl, Alt y Mayús, y
  `Ctrl+letra`), sin probar en el simulador.
- [x] CA3 — Con el teclado abierto, la terminal ocupa la altura de `visualViewport` y la última fila queda justo encima
  del teclado (iOS pone el teclado encima sin redimensionar la página). Encima de la fila de teclas, que va sobre la
  barra de accesorios de WKWebView: `visualViewport` la descuenta y se deja, sin plugin Swift (P3 a). El tamaño que se
  pide se mide sin teclado, así que abrirlo o cerrarlo no reabre el `surface`. **Desviación**: la barra de atajos de
  un teclado físico (Bluetooth) no cuenta como teclado, así que el hueco que deja sí se pide y reinicia el `surface`
  una vez al aparecer y otra al quitarse, sin redimensionar el Mac. Una rejilla más baja que el hueco (50 filas al
  ancho ajustado) queda arriba, con un margen debajo.
- [x] CA4 — Deslizar vertical = `scroll --pane P --up/--down N` (coalescido en 011), no el scroll de la página; un
  toque sigue siendo foco/clic. **Desviación**: `scroll` solo fuera de la pantalla alterna y sin ratón, que es lo que
  upstream admite; si no, la rueda en la celda del dedo o flechas según los `modes`, como gpui. Con zoom, la rejilla
  se mueve primero en el DOM hasta su borde y luego sigue el historial (P1 a). El toque no manda clic (P2 a).
- [x] CA5 — Pegar desde el portapapeles del móvil: un botón que pega (input bracketed si el programa lo pide, lo
  decide cornercase). Con `navigator.clipboard.readText()` y el globo «Paste» de iOS, sin plugin (P4 a); vacío o
  rechazado, un toast. Probado en `ios dev`; falta el build de release (`tauri://`).
- [ ] CA6 — Probado en simulador **y** en iPhone real: escribir un prompt largo, borrar, flechas, `Ctrl-C`; anotar lo
  que no funcionó y cómo se resolvió. Hecho en el simulador iPhone Air (iOS 27.2, cornercase 0.13.19, 2026-11-09), en
  una shell y en Claude Code 2.1.319: tres fallos encontrados y arreglados (foco del toque, la fila cerraba el teclado,
  pegar rechazado), en `design/terminal.md` › Typing. Falta el iPhone real, con su lista en el mismo sitio.

## Plan B
Si el `textarea` oculto da problemas con el IME de iOS: un campo de texto visible de "componer" que manda la línea
entera con `send` y deja la terminal solo para teclas sueltas.

## Referencias
- cornercase-terminal (`origin/main`): `design/browser-access.md` (viewport, swipe → rueda en la celda del dedo,
  `visualViewport`, teclado sobre la página, fila de teclas que falta), `design/keys-and-mouse.md`,
  `design/commands-for-scripts-and-agents/driving-terminals.md`.
- cornercase-gpui: `src/terminal/input.rs`, `src/terminal/mouse.rs`, `src/cli/typist.rs`.
- Orca: `mobile/modules/orca-mobile-web-shell/ios/MobileWebShellKeyboardAccessory.swift`,
  `…/MobileWebShellKeyboardWarmup.swift` (fila de teclas nativa y arranque del teclado).

## design/ que actualiza
- `design/terminal.md`: cómo se captura el teclado, la fila de teclas, el swipe, lo probado en qué iOS.
- `design/architecture.md`: los comandos `terminal_input` y `terminal_scroll`, `modes` en `terminal-changed`, y
  `serde_json` (ya no solo de test) y `futures-util` (test) en `src-tauri`.

## Verificación
En iPhone real: escribir y desplazar sin cambiar el tamaño de la ventana del Mac (criterio de éxito 3).
