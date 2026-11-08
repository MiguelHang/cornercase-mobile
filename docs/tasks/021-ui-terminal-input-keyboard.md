# 021 — Pantalla 3: terminal — entrada, teclas que faltan, desplazar y teclado del WebView ⚠

- Estado: pendiente
- Depende de: 020
- Riesgo: **alto** — teclado virtual de iOS sobre la WebView (foco, altura, IME), gestos.
- Rama: `feat/021-terminal-input` · PR: `feat: type and scroll in a live terminal from the phone`

## Objetivo
Escribir en la terminal en vivo desde el teclado del móvil, con una fila de teclas que le faltan, y desplazar el
historial con el dedo.

## Criterios de aceptación
- [ ] CA1 — Un `textarea` oculto recibe el foco al tocar la terminal; lo que escribe el teclado (también dictado, IME
  y autocorrección desactivada: `autocapitalize=off`, `autocorrect=off`, `spellcheck=false`) se convierte en eventos
  de `input --follow --json` (typist de 011).
- [ ] CA2 — Fila sobre el teclado: `Esc`, `Tab`, `Ctrl` (modificador pegajoso para la siguiente tecla), `↑ ↓ ← →`,
  `|`, `~`, `/`, `-`. Mapeo en `src/lib/terminal/keys.ts` con vitest.
- [ ] CA3 — Con el teclado abierto, la terminal ocupa la altura de `visualViewport` y la última fila queda justo encima
  del teclado (iOS pone el teclado encima sin redimensionar la página).
- [ ] CA4 — Deslizar vertical = `scroll --pane P --up/--down N` (coalescido en 011), no el scroll de la página; un
  toque sigue siendo foco/clic.
- [ ] CA5 — Pegar desde el portapapeles del móvil: un botón que pega (input bracketed si el programa lo pide, lo
  decide cornercase).
- [ ] CA6 — Probado en simulador **y** en iPhone real: escribir un prompt largo, borrar, flechas, `Ctrl-C`; anotar lo
  que no funcionó y cómo se resolvió.

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

## Verificación
En iPhone real: escribir y desplazar sin cambiar el tamaño de la ventana del Mac (criterio de éxito 3).
