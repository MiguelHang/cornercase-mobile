# 009 — Identidad Ed25519 en el Keychain de iOS ⚠

- Estado: pendiente
- Depende de: 001
- Riesgo: **alto** — `keyring` en iOS desde Tauri (acceso al Keychain, simulador sin entitlements de grupo).
- Rama: `feat/009-identity` · PR: `feat: create the device identity and keep it in the keychain`

## Objetivo
Generar en el primer arranque un par de claves Ed25519 que identifica al dispositivo y guardarlo en el Keychain, para
reutilizarlo como identidad de iroh más adelante. En la fase 0 el puente no lo usa.

## Criterios de aceptación
- [ ] CA1 — `crates/core/src/identity.rs`: `Identity` (ed25519-dalek, `rand_core` del SO), `load_or_create(secrets)`
  sobre el trait `Secrets` (semilla de 32 bytes), `public_key()` en base64url y huella corta para mostrar. Tests con
  un `Secrets` en memoria: se crea una vez, la segunda vez se lee la misma.
- [ ] CA2 — `src-tauri/src/secrets/ios.rs`: `Secrets` con el Keychain (`keyring` con `apple-native`, servicio
  `dev.usecornercase.mobile`), accesibilidad `AfterFirstUnlockThisDeviceOnly` si el crate lo permite (si no, anotar).
- [ ] CA3 — Comprobado en el simulador de iOS y en un iPhone: la clave sobrevive a cerrar y abrir la app; la huella se
  ve en un log o en una pantalla de depuración temporal que no se mergea.
- [ ] CA4 — Si el Keychain falla, la app arranca igualmente y avisa (la identidad no se usa en la fase 0); nunca se
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
