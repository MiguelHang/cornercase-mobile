# 032 — Android: teclado del WebView y cambio de red ⚠

- Estado: pendiente
- Depende de: 030, 021, 028
- Riesgo: **alto** — IME de Android (GBoard) en un `textarea` oculto, `ConnectivityManager`.
- Rama: `feat/032-android-input-network` · PR: `feat: type in the terminal and follow network changes on android`

## Objetivo
Que la terminal y el chat funcionen con los teclados de Android y que el cambio de red se detecte como en iOS.

## Criterios de aceptación
- [ ] CA1 — `interactive-widget=resizes-content` en el viewport; la última fila de la terminal queda encima del teclado.
- [ ] CA2 — GBoard y SwiftKey: escribir, borrar (los IME de Android mandan `beforeinput`/composición en vez de
  `keydown`: se traducen bien a `input`), predicción desactivada en la terminal.
- [ ] CA3 — Botón atrás de Android: cierra sheets y vuelve de pantalla con el mismo `router.back()` que la barra de navegación de la 037 (en Android, `←` sin título anterior y sin gesto de borde propio, que es del sistema); en la terminal no se manda a la PTY.
- [ ] CA4 — El plugin de red de 028 gana parte Android (`ConnectivityManager.NetworkCallback`) con el mismo evento.
- [ ] CA5 — Probado en un dispositivo real: criterios de éxito 2, 3 y 6 en Android.

## Plan B
El campo "componer" del plan B de 021 como modo por defecto en Android si el IME no se deja domar.

## Referencias
- cornercase-terminal (`origin/main`): `design/browser-access.md` (`interactive-widget=resizes-content`).
- Orca: `mobile/modules/orca-mobile-web-shell/android/`.

## design/ que actualiza
- `design/terminal.md` (Android), `design/connection.md` (red en Android).

## Verificación
Notas de la prueba en dispositivo en la PR.

## Notas de prueba (emulador)

### T1 — ¿encoge la WebView con el teclado? (2026-11-10)
Emulador `cornercase_api37` (Android 17, API 37), WebView 145.0.7632.218, GBoard 17.2.2 (el IME por defecto de la
imagen `google_apis`), APK debug con el frontend embebido, contra el puente por la IP del Mac. Medido en CSS px con
DevTools remoto (`adb forward tcp:… localabstract:webview_devtools_remote_<pid>`), ancho 411, DPR 2,625.

- **Sin tocar `MainActivity.kt`** (solo `interactive-widget=resizes-content`): la WebView **no** encoge. Sin teclado
  `innerHeight` 914 y `visualViewport.height` 914; con GBoard abierto en el chat `innerHeight` 914,
  `visualViewport.height` 578, `offsetTop` 29 y la página desplazada (`scrollY` 251): el campo del chat queda debajo
  del teclado. El IME mide 883 px físicos (336 CSS px). Con `enableEdgeToEdge()` y `targetSdk` 37 la ventana no se
  redimensiona y `resizes-content` no tiene nada que redimensionar.
- **Con los insets del IME en `MainActivity.kt`** (P5 a: padding inferior del `android.R.id.content` igual al inset
  `ime()`, y a los hijos les llegan los insets recortados en esa altura): con el teclado abierto `innerHeight` 578 y
  `visualViewport.height` 578, `offsetTop` 0, `scrollY` 0, `env(safe-area-inset-bottom)` 0 (sin él, 24 px). En el chat
  el campo y su fila de teclas quedan pegados encima del teclado; en la terminal, la fila de teclas también. Al
  cerrar el teclado (atrás) vuelve a 914 / 914 con `scrollY` 0 y `safe-area-inset-bottom` 24: no se queda desplazada.
  `adjustResize` no se probó: con edge-to-edge desde Android 15 no tiene efecto.
- **Para T2/T3**: en Android encogen los dos (`innerHeight` y `visualViewport`), así que hoy `open` es `false` y
  `covered` sale `true` solo por decimales (`innerHeight` 578 entero frente a `visualViewport.height` 577,905). Con la
  terminal abierta, abrir y cerrar el teclado reabre el `surface` cada vez (cuatro `runs surface` en el log del
  puente para una apertura y un cierre) y en una de ellas salió el toast "the cornercase server stopped before
  answering". Es el fallo que cubre CA2.
- GBoard en la terminal (campo oculto) enseña la fila de herramientas, no la barra de sugerencias, y las minúsculas
  (sin mayúscula inicial); en el chat, mayúscula inicial. Sin escribir nada: es de T5.

### T3 — altura y desplazamiento con el teclado (2026-11-10)
Mismo emulador y APK con T2, en un AVD propio (`cornercase_api37_032`), contra el puente por la IP del Mac, en la
terminal y el chat de un agente parado (Claude Code sin trabajar), sin escribir nada.

- **Terminal**: con GBoard abierto `innerHeight` 578 y `visualViewport.height` 577,905, la pantalla con la clase
  `keyboard`, `top` 0 y `height` 577,905: la fila de teclas pegada encima del teclado y la última fila de la rejilla
  encima de la fila. Al cerrarlo (atrás) vuelve a 914, sin `keyboard` ni `top`/`height`, sin otro toque. `scrollY` 0
  siempre. Cuatro aperturas y cierres: ningún `runs surface` nuevo en el log del puente.
- **Con "Fit to this phone"**: al encenderlo un `runs view` y un `runs surface` (el ajuste); dos aperturas y cierres
  del teclado después, nada más; la rejilla ajustada queda desplazada a su última fila encima de la fila de teclas.
  Al apagarlo, un `runs surface` (vuelta al modo pasivo).
- **Chat**: el campo y su fila de teclas pegados encima del teclado (campo en 518–562 de 578); al cerrarlo vuelve
  abajo (830–874 de 914) con `scrollY` 0.
- **Abrir la terminal con el teclado del chat abierto**: el teclado se cierra al perder el foco y la terminal sale a
  toda la altura, con un solo `runs surface`.
- **Girar con el teclado abierto**: a 914 de ancho con el teclado, `innerHeight` 150 y la pantalla sigue tratándolo
  como abierto; un `runs surface` por el giro (el tamaño cambia) y ninguno al cerrar el teclado después; otro al
  volver a vertical.
- **Sin cambios de código**: el `top`/`height` de `Terminal.svelte` ya vale en Android (`offsetTop` 0 y la altura del
  visual viewport, la misma que la del layout), y la página no se queda desplazada al cerrar el teclado, así que no
  se añade la vuelta del scroll a 0: lo visto en la 030 era sin los insets del IME de `MainActivity.kt` (T1).

### T5 — escribir con GBoard y con teclas de hardware (2026-11-10)
Mismo AVD y APK con T4, contra un servidor cornercase 0.13.19 aislado (`HOME`, estado y socket propios) detrás de un
segundo puente, en la terminal de un agente falso (un `bash` real lanzado como `claude`): nada llegó al servidor
del usuario. Eventos del campo oculto leídos por CDP; lo recibido, con `cornercase read`.

- **GBoard no compone**: cada letra llega como `keydown` `Unidentified` (229) y un `beforeinput` `insertText` no
  composición de un carácter, así que el Mac la recibe al momento, no por palabras (P2: queda la a), sin cambios).
- **La barra de sugerencias sale igual**, aunque la WebView ya pide `NO_SUGGESTIONS` (`inputType` 0x2a00a1:
  `MULTI_LINE | NO_SUGGESTIONS | WEB_EDIT_TEXT`); GBoard la enseña y no autocorrige (`teh ` llega tal cual). Tocar una
  sugerencia manda solo lo que falta (`echo` + `insertText` `es `), así que tampoco rompe nada. El flag que la
  quitaría ya está puesto, así que no se añaden `inputmode` ni `enterkeyhint` (sin probar: no controlan ese flag) y
  `Typist.svelte` queda igual, sin riesgo para iOS.
- **Borrar** es un `keydown` `Backspace` (8) por pulsación, sin `beforeinput`: un `backspace` cada uno, la línea
  vacía en el Mac. **Enter** es un `keydown` `Enter` (13): un solo `enter`, la línea se ejecuta una vez. No se vio
  ningún `insertText` `'\n'`.
- **Teclas de hardware** (`input text`, `input keyevent`, como el teclado del host): las letras pasan igual por el
  IME (229 + `insertText`); `Backspace`, `Enter`, `ArrowUp` (historial) y Ctrl-C llegan como `keydown` con su
  `keyCode` y funcionan.
- Sin ver en vivo: una composición (escritura deslizando, voz, acentos) y SwiftKey (no está en la imagen); quedan en
  la lista del dispositivo y cubiertas por los tests.
