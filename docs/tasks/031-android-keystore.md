# 031 — Android: identidad y secretos en el Keystore ⚠

- Estado: pendiente
- Depende de: 030, 009
- Riesgo: **alto** — `keyring` no cubre Android; hace falta JNI o un plugin de Tauri en Kotlin.
- Rama: `feat/031-android-keystore` · PR: `feat: keep the identity and tokens in the android keystore`

## Objetivo
Implementar `Secrets` en Android para que la identidad Ed25519 y los tokens de las rutas queden protegidos por el
Keystore del sistema.

## Criterios de aceptación
- [ ] CA1 — Plugin de Tauri en Kotlin (`src-tauri/plugins/secrets` con parte Android): una clave AES-GCM en
  `AndroidKeyStore` (no exportable) cifra cada secreto; el texto cifrado va a `SharedPreferences` de la app.
  (Ed25519 no está en todas las versiones del Keystore: se guarda la semilla cifrada, no la clave en el Keystore.)
- [ ] CA2 — Misma interfaz `Secrets` que iOS; la identidad creada en Android sobrevive a cerrar la app y a reiniciar
  el teléfono; desinstalar la borra.
- [ ] CA3 — Migración desde el almacén provisional de 030, si lo hubo.
- [ ] CA4 — Probado en emulador y en un dispositivo real (anotar modelo y versión de Android).

## Plan B
`tauri-plugin-stronghold` o `EncryptedSharedPreferences` (deprecado: anotar riesgo) si el plugin propio se atasca.

## Referencias
- IDEA.md › Riesgos (Keystore de Android desde Rust). Tauri 2: plugins móviles en Kotlin.
- Orca: `mobile/` (almacenamiento seguro en Android).

## design/ que actualiza
- `design/connection.md` (Identity): Android, por qué semilla cifrada y no clave en el Keystore.
- `design/architecture.md`: el plugin y sus dos plataformas.

## Verificación
La huella de la identidad es la misma tras reiniciar el teléfono.
