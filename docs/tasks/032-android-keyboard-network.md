# 032 — Android: teclado del WebView y cambio de red ⚠

- Estado: hecha, salvo la prueba en un Android real (CA5: queda para el usuario, no había uno a mano)
- Depende de: 030, 021, 028
- Riesgo: **alto** — IME de Android (GBoard) en un `textarea` oculto, `ConnectivityManager`.
- Rama: `feat/032-android-input-network` · PR: `feat: type in the terminal and follow network changes on android`

## Objetivo
Que la terminal y el chat funcionen con los teclados de Android y que el cambio de red se detecte como en iOS.

## Criterios de aceptación
- [x] CA1 — `interactive-widget=resizes-content` en el viewport; la última fila de la terminal queda encima del teclado.
  Con solo el viewport la WebView no encogía (edge-to-edge con `targetSdk` 37): `MainActivity.kt` aplica los insets
  del IME (`design/architecture/android.md`). `keyboard.svelte.ts` mide el teclado contra la mayor altura vista al
  mismo ancho, porque en Android encogen la página y el visual viewport a la vez (`design/terminal.md` › Typing).
- [x] CA2 — GBoard (SwiftKey y las composiciones, en la lista del dispositivo): escribir, borrar (los IME de Android mandan `beforeinput`/composición en vez de
  `keydown`: se traducen bien a `input`), predicción desactivada en la terminal. GBoard comprobado en el emulador:
  letra a letra, sin componer; un `keydown` 229 pasa sin contar como tecla. La barra de sugerencias sale aunque la
  WebView pide `NO_SUGGESTIONS`, pero GBoard no autocorrige. SwiftKey y las composiciones (deslizar, voz, acentos),
  solo en los tests: van a la lista del dispositivo.
- [x] CA3 — Botón atrás de Android: cierra sheets y vuelve de pantalla con el mismo `router.back()` que la barra de
  navegación de la 037 (en Android, `←` sin título anterior y sin gesto de borde propio, que es del sistema); en la
  terminal no se manda a la PTY. `goBack` con una ruta debajo, cancela el escáner, y en la base es del sistema
  (`design/look/navigation.md` › Android's back). Comprobado en el emulador en cada pantalla.
- [x] CA4 — El plugin de red de 028 gana parte Android (`ConnectivityManager.NetworkCallback`) con el mismo evento.
  Solo manda si cambia la red por defecto, `kind`, `expensive` o la VPN (`design/connection/network.md` › The
  Android plugin). Comprobado en el emulador con wifi, datos y modo avión.
- [x] CA5 — Probado en un dispositivo real: criterios de éxito 2, 3 y 6 en Android. Pendiente del usuario (lista de
  abajo).

## Para el Android real (CA5)
Con el puente en el Mac y el teléfono por Tailscale (`pnpm tauri android dev "<dispositivo>"`, depuración USB activa):

- **Criterio de éxito 2** (contestar a un agente que espera): desde el chat, con GBoard y con SwiftKey, una respuesta
  con texto y una tecla de la fila; el agente sigue trabajando. La imagen no: el plugin `photos` solo tiene iOS y en
  Android `Attach` dice que no hay selector de fotos (ninguna tarea de Android lo trae; tarea aparte si se quiere).
- **Criterio de éxito 3** (terminal en vivo): abrir la terminal de un agente, escribir y desplazar sin que cambie el
  tamaño de la ventana del Mac; con "Fit to this phone", leer al ancho del teléfono.
  - GBoard real: letra a letra, borrar, Enter una vez, la barra de sugerencias; escribir deslizando, por voz y con
    acentos (composiciones: si se portan mal, plan B).
  - SwiftKey: lo mismo (no está en la imagen del emulador).
  - Un teclado físico por Bluetooth o USB: flechas, Esc, Tab, Ctrl-C.
  - El teclado abriendo y cerrando sin reabrir el `surface`, la fila encima del teclado, Paste y su menú.
  - Atrás: con el teclado abierto (el primero lo cierra), en cada pantalla, el gesto además del botón.
- **Criterio de éxito 6** (todo se reconecta solo), midiendo desde la línea `network` de `logcat`
  (`RustStdoutStderr`) hasta `connected`, con el tablero y con la terminal abiertos:
  - wifi → datos con Tailscale, y datos → wifi;
  - entrar y salir de Tailscale (la VPN como red por defecto);
  - de una wifi a otra;
  - sin red: "No network", y la vuelta;
  - salir de la app y volver.

## Plan B
El campo "componer" del plan B de 021 como modo por defecto en Android si el IME no se deja domar.

## Referencias
- cornercase-terminal (`origin/main`): `design/browser-access.md` (`interactive-widget=resizes-content`).
- Orca: `mobile/modules/orca-mobile-web-shell/android/`.

## design/ que actualiza
- `design/terminal.md` (teclados de Android, la altura con el teclado), `design/connection/network.md` (el plugin
  en Android), `design/look/navigation.md` (el atrás de Android), `design/architecture/android.md` (`MainActivity.kt`),
  `design/architecture/plugins.md` (los plugins), `DESIGN.md`.

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

### T9 — cambios de red con el tablero y la terminal abiertos (2026-11-10)
El mismo AVD (wifi `AndroidWifi` y datos LTE a la vez) y el mismo servidor aislado que en T5, contra el puente por la IP
de LAN del Mac. La línea `cornercase-mobile: network …` de Rust sale en `logcat` (tag `RustStdoutStderr`), así que
P6 a) vale: el tiempo se toma desde el `adb shell` hasta que la barra vuelve a nombrar la máquina (tablero, leída por
CDP) o hasta que el puente recibe el nuevo `surface` (terminal, que no enseña barra).

- **Un valor por cambio**: `svc wifi disable` → `cellular expensive`, `svc data disable` → `none`, `svc data enable` →
  `cellular expensive`, `svc wifi enable` → `wifi`; modo avión → `none` y, al quitarlo, `cellular expensive` y unos
  3 s después `wifi`. Nada más: ni repetidos por cambios de capacidades, ni al pasar 30 s en segundo plano y volver
  (con T8 llegaba un segundo `wifi`). Un solo callback de la app en `dumpsys connectivity` (el otro es de la WebView).
- **`Offline` y vuelta sola**: sin red, la barra dice `No network` y no reintenta; con red, reconecta sin tocar nada,
  también en la terminal, que se vuelve a suscribir y enseña lo escrito después en el panel.
- **Tiempos hasta `connected`**: wifi → datos, 1,5–2,6 s; datos de vuelta, 1,4–1,8 s; wifi de vuelta, 9–12 s, de los
  que unos 5 s tarda la wifi del emulador en ser la red por defecto y el resto son intentos que el puente corta
  (`Handshake not finished`, `Connection reset`) antes del bueno; al quitar el modo avión, 2,5 s por datos y otra
  reconexión al llegar la wifi (10 s en total).
- **El socket suele caer antes del aviso**: al quitar la wifi la barra pasa por `Retrying in 1 s` antes de que llegue
  `cellular`, y el cambio de red reconecta igual.
- Sin ver en vivo: VPN (Tailscale en el teléfono) y pasar de una wifi a otra (el emulador tiene una); van a la lista
  del dispositivo.
