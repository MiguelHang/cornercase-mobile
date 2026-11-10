# 031 — Android: identidad y secretos en el Keystore ⚠

- Estado: hecha, salvo la prueba en un Android real (queda para el usuario: no había uno a mano)
- Depende de: 030, 009
- Riesgo: **alto** — `keyring` no cubre Android; hace falta JNI o un plugin de Tauri en Kotlin.
- Rama: `feat/031-android-keystore` · PR: `feat: keep the identity and tokens in the android keystore`

## Objetivo
Implementar `Secrets` en Android para que la identidad Ed25519 y los tokens de las rutas queden protegidos por el
Keystore del sistema.

## Criterios de aceptación
- [x] CA1 — Plugin de Tauri en Kotlin (`src-tauri/plugins/secrets` con parte Android): una clave AES-GCM en
  `AndroidKeyStore` (no exportable) cifra cada secreto; el texto cifrado va a `SharedPreferences` de la app.
  (Ed25519 no está en todas las versiones del Keystore: se guarda la semilla cifrada, no la clave en el Keystore.)
  Una clave AES-256 sin padding, el nombre del secreto como AAD y `commit()` (`design/architecture/android.md` ›
  The Keystore); por qué un plugin propio y no `EncryptedSharedPreferences`, stronghold ni JNI a mano, en
  `design/architecture/plugins.md`.
- [x] CA2 — Misma interfaz `Secrets` que iOS; la identidad creada en Android sobrevive a cerrar la app y a reiniciar
  el teléfono; desinstalar la borra. Comprobado en el emulador (Android 17, API 37) el 2026-11-10: la misma huella
  tras `am force-stop` y tras `adb reboot`, otra tras desinstalar.
- [x] CA3 — Migración desde el almacén provisional de 030, si lo hubo. Los tokens de `route-secrets.json` pasan al
  Keystore en el primer arranque y el fichero se borra; si algo falla, el fichero sigue y se reintenta en el siguiente
  (`design/connection/identity.md`). Comprobado en el emulador actualizando desde la 030 con tres rutas.
- [x] CA4 — Probado en emulador y en un dispositivo real (anotar modelo y versión de Android).
  Sin marcar: emulador comprobado (`design/architecture/android.md` › Checked live), dispositivo pendiente del
  usuario. Pasos en un Android real, con un bloqueo de pantalla (PIN, patrón o contraseña) y la depuración USB
  activados:
  1. Anotar el modelo, la versión de Android y su parche de seguridad (Ajustes › Información del teléfono).
  2. Si se quiere probar también la migración: instalar antes la 030 (`git checkout 0a2d191`,
     `pnpm tauri android dev "<dispositivo>"`), añadir la máquina con el código del puente y volver a esta rama.
  3. `pnpm tauri android dev "<dispositivo>"`. En el log, `cornercase-mobile: identity <huella>` (y, si venía de la
     030, `moved <n> route secrets into the store` antes); la misma huella en Machines › This device. Sin la 030,
     añadir la máquina con el código. Debe conectar por Tailscale.
  4. Cerrar la app a la fuerza (`adb shell am force-stop dev.usecornercase.mobile.debug`) y abrirla: la misma huella,
     y la máquina conecta sin el código.
  5. Reiniciar el teléfono, desbloquearlo y abrir la app: la misma huella, y conecta.
  6. Con la app abierta, bloquear la pantalla un par de minutos y desbloquear: la conexión vuelve sola y el log no
     tiene errores del almacén (`adb logcat -d | grep "cornercase-mobile:"`).
  7. Opcional: cambiar el bloqueo de pantalla (otro PIN) y abrir la app. Si sale `no identity: …`, el fabricante
     borró la clave del Keystore: anotarlo; se arregla borrando los datos de la app (`design/connection/identity.md`
     › A secret that no longer opens).
  8. `adb uninstall dev.usecornercase.mobile.debug` y volver a instalar: otra huella.

## Plan B
`tauri-plugin-stronghold` o `EncryptedSharedPreferences` (deprecado: anotar riesgo) si el plugin propio se atasca.

## Referencias
- IDEA.md › Riesgos (Keystore de Android desde Rust). Tauri 2: plugins móviles en Kotlin.
- Orca: `mobile/` (almacenamiento seguro en Android).

## design/ que actualiza
- `design/connection/identity.md` (nuevo; Identity salió de `design/connection.md`, que pasaba de 40 KB): Android,
  por qué semilla cifrada y no clave en el Keystore, y la migración de los tokens de la 030.
- `design/architecture.md` y `design/architecture/plugins.md` (nuevo, por el mismo motivo): el plugin, solo en
  Android y con un stub en el resto (iOS sigue con el Keychain), y su dependencia.
- `design/architecture/android.md`: The Keystore, Backups y Checked live.

## Verificación
La huella de la identidad es la misma tras reiniciar el teléfono.
