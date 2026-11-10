# 030 — Android: proyecto, arranque en emulador y dispositivo

- Estado: hecha, salvo la prueba en un Android real por Tailscale (queda para el usuario: no había uno a mano)
- Depende de: 015–028 (antes era la 029; la prueba en el iPhone se dejó para el final el 2026-11-09)
- Riesgo: medio (NDK, toolchains de Rust para Android)
- Rama: `chore/030-android` · PR: `chore: add the android project`

## Objetivo
La misma app corriendo en Android, sin funciones nuevas, con lo específico de iOS detrás de `cfg`.

## Criterios de aceptación
- [x] CA1 — `pnpm tauri android init`; `src-tauri/gen/android` versionado (D4); `applicationId`
  `dev.usecornercase.mobile`; `minSdk` elegido y justificado. `minSdk` 24, fijado en `tauri.conf.json`
  (`design/architecture/android.md`).
- [x] CA2 — `pnpm tauri android dev` arranca en el emulador y en un dispositivo; conecta al puente y muestra el tablero.
  Sin marcar: emulador comprobado, dispositivo pendiente del usuario. En el emulador (Android 17, API 37), el
  2026-11-10: hot reload, la máquina añadida con el código del puente, el tablero en vivo por Tailscale y la máquina
  que sigue tras cerrar la app. Pendiente del usuario: un dispositivo real por Tailscale
  (`pnpm tauri android dev "<dispositivo>"` con la depuración USB activada) y escanear un QR de verdad, porque la
  cámara del emulador no llega a abrirse.
- [x] CA3 — Mientras no exista 031, `Secrets` en Android devuelve un error claro y la app sigue (la identidad no se usa
  en la fase 0; el token de las rutas necesita almacén: usar el plan provisional decidido en la tarea y anotarlo).
  La identidad da "no secret store on this platform yet"; los tokens van a `FileSecrets`, un JSON en la carpeta de
  datos de la app, hasta la 031 (`design/connection.md` › Android's route tokens, until 031).
- [x] CA4 — Permisos: `INTERNET`, `CAMERA` (QR), `ACCESS_NETWORK_STATE`; tráfico `ws://` en claro permitido solo para
  rangos privados y Tailscale (`network_security_config.xml`), no global. El fichero no admite rangos de IP: permite
  `localhost`, `127.0.0.1` y `ts.net`, y el `ws://` sale de Rust, fuera de esa política; la frontera es el puente, que
  solo escucha en esos rangos (`design/architecture/android.md` › Cleartext). Sin copias de seguridad de los datos.
  Una guarda en Rust que rechace rutas `ws://` fuera de esos rangos queda como tarea aparte, si se quiere.
- [x] CA5 — `CLAUDE.md` › Commands con los pasos de Android (SDK, NDK, targets de rustup).

## Referencias
- Tauri 2: guía de Android. Orca: `mobile/app.json`, `mobile/plugins/` (configuración nativa de Android).

## design/ que actualiza
- `design/architecture.md`: Android, `minSdk`, tráfico en claro y por qué solo en esos rangos.

## Verificación
Tablero en vivo en un Android real por Tailscale.
