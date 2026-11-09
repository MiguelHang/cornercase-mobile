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
