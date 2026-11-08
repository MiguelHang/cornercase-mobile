# 030 — Android: proyecto, arranque en emulador y dispositivo

- Estado: pendiente
- Depende de: 029
- Riesgo: medio (NDK, toolchains de Rust para Android)
- Rama: `chore/030-android` · PR: `chore: add the android project`

## Objetivo
La misma app corriendo en Android, sin funciones nuevas, con lo específico de iOS detrás de `cfg`.

## Criterios de aceptación
- [ ] CA1 — `pnpm tauri android init`; `src-tauri/gen/android` versionado (D4); `applicationId`
  `dev.usecornercase.mobile`; `minSdk` elegido y justificado.
- [ ] CA2 — `pnpm tauri android dev` arranca en el emulador y en un dispositivo; conecta al puente y muestra el tablero.
- [ ] CA3 — Mientras no exista 031, `Secrets` en Android devuelve un error claro y la app sigue (la identidad no se usa
  en la fase 0; el token de las rutas necesita almacén: usar el plan provisional decidido en la tarea y anotarlo).
- [ ] CA4 — Permisos: `INTERNET`, `CAMERA` (QR), `ACCESS_NETWORK_STATE`; tráfico `ws://` en claro permitido solo para
  rangos privados y Tailscale (`network_security_config.xml`), no global.
- [ ] CA5 — `CLAUDE.md` › Commands con los pasos de Android (SDK, NDK, targets de rustup).

## Referencias
- Tauri 2: guía de Android. Orca: `mobile/app.json`, `mobile/plugins/` (configuración nativa de Android).

## design/ que actualiza
- `design/architecture.md`: Android, `minSdk`, tráfico en claro y por qué solo en esos rangos.

## Verificación
Tablero en vivo en un Android real por Tailscale.
