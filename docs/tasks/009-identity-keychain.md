# 009 — Identidad Ed25519 en el Keychain de iOS ⚠

- Estado: hecha, salvo la prueba en un iPhone (CA3)
- Depende de: 001
- Riesgo: **alto** — `keyring` en iOS desde Tauri (acceso al Keychain, simulador sin entitlements de grupo).
- Rama: `feat/009-identity` · PR: `feat: create the device identity and keep it in the keychain`

## Objetivo
Generar en el primer arranque un par de claves Ed25519 que identifica al dispositivo y guardarlo en el Keychain, para
reutilizarlo como identidad de iroh más adelante. En la fase 0 el puente no lo usa.

## Criterios de aceptación
- [x] CA1 — `crates/core/src/identity.rs`: `Identity` (ed25519-dalek, `rand_core` del SO), `load_or_create(secrets)`
  sobre el trait `Secrets` (semilla de 32 bytes), `public_key()` en base64url y huella corta para mostrar. Tests con
  un `Secrets` en memoria: se crea una vez, la segunda vez se lee la misma. Hecho con `getrandom::fill`: `rand_core`
  0.10, el de ed25519-dalek 3, ya no trae `OsRng`.
- [x] CA2 — `src-tauri/src/secrets/ios.rs`: `Secrets` con el Keychain (`keyring` con `apple-native`, servicio
  `dev.usecornercase.mobile`), accesibilidad `AfterFirstUnlockThisDeviceOnly` si el crate lo permite (si no, anotar).
  Hecho con `keyring-core` 1 y `apple-native-keyring-store` 1 (`protected`), que sí la permite: `keyring` 4 ya no
  tiene `apple-native`.
- [ ] CA3 — Comprobado en el simulador de iOS y en un iPhone: la clave sobrevive a cerrar y abrir la app; la huella se
  ve en un log o en una pantalla de depuración temporal que no se mergea. Simulador hecho el 2026-11-09 (iPhone 17,
  iOS 27.2, la huella en el log); **pendiente: la prueba en un iPhone**, que necesita un equipo de firma.
- [x] CA4 — Si el Keychain falla, la app arranca igualmente y avisa (la identidad no se usa en la fase 0); nunca se
  guarda la semilla en un fichero.

## Plan B
Si `keyring` no funciona en iOS: plugin propio de Tauri en Swift (`src-tauri/plugins/secrets` o `tauri-plugin-…`) con
`SecItemAdd`/`SecItemCopyMatching`, misma interfaz `Secrets`. Valorar también `tauri-plugin-stronghold` (rechazable:
guarda en fichero con contraseña, no en el Keychain).

## Referencias
- IDEA.md › Riesgos (Keystore/Keychain); `design/connection.md` (Identity).
- Orca: `mobile/` (cómo guarda credenciales; buscar `SecureStore`).

## design/ que actualiza
- `design/connection.md`: crate elegido, accesibilidad del Keychain, qué se probó y en qué versión de iOS.
- `design/architecture.md`: el trait `Secrets` y dónde se implementa por plataforma.

## Verificación
Tests verdes; prueba manual en simulador y dispositivo anotada en la PR.
