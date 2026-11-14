# 060 — Primera versión: 0.1.0 por TestFlight y APK

- Estado: pendiente
- Depende de: 059, y que upstream publique la versión con `/cli` (057)
- Rama: `release/060-0.1.0` · PR: `chore: release 0.1.0`

## Objetivo
Publicar la 0.1.0 (decidido por el usuario el 2026-11-14): iOS por TestFlight (luego App Store), Android como APK
firmado en GitHub Releases.

## Criterios de aceptación
- [ ] CA1 — Versión 0.1.0 en `Cargo.toml`, `package.json` y `tauri.conf.json`, y un `CHANGELOG.md`.
- [ ] CA2 — iOS: cuenta de pago del Apple Developer Program (la da de alta el usuario), firma de distribución, build
  subida a App Store Connect y disponible en TestFlight.
- [ ] CA3 — Android: keystore de release fuera del repo (secretos de CI), APK firmado y con `applicationId` definitivo,
  adjunto a un GitHub Release `v0.1.0`.
- [ ] CA4 — Un workflow de release que construye y firma ambos al crear la etiqueta `v0.1.0`.
- [ ] CA5 — `README.md` para usuarios: qué es, versión mínima de cornercase, cómo emparejar (Settings › Remote ›
  phone app) y cómo instalar.
- [ ] CA6 — Política de privacidad (la pide TestFlight para pruebas externas): la app no recoge datos y solo habla con
  el Mac emparejado.
- [ ] CA7 — Repo público (también desbloquea las issues de la 033).

## Decisiones del usuario que hacen falta
- Alta en el Apple Developer Program (99 $/año) y el nombre de la app en App Store Connect.
- `applicationId` y bundle id definitivos (hoy `dev.usecornercase.mobile`: ¿se queda?).
