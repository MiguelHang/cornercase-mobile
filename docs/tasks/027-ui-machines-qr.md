# 027 — Pantalla 6: máquinas — lista, QR y varias rutas

- Estado: hecha (el escaneo real, con cámara, queda para el usuario en un iPhone)
- Depende de: 014, 008
- Riesgo: medio (permiso de cámara en iOS, plugin `barcode-scanner`)
- Rama: `feat/027-machines` · PR: `feat: add machines by qr code and give them several routes`

## Objetivo
Gestionar las máquinas: añadirlas escaneando el QR del puente o pegando la URL, darles varias rutas (LAN, Tailscale)
y ver por cuál se está conectado.

## Criterios de aceptación
- [x] CA1 — Lista de máquinas con estado (conectada por ruta X, reintentando, sin respuesta), cambiar de máquina activa.
  Las no activas se prueban al abrir la pantalla y con `Check again`, sin sondeo; la activa se recuerda entre
  arranques (`design/connection/machines.md`).
- [x] CA2 — Añadir: escanear QR con `@tauri-apps/plugin-barcode-scanner` (`NSCameraUsageDescription` en Info.plist)
  o pegar URL; si la URL es de una máquina que ya existe (mismo `boot` al probarla o mismo nombre), se ofrece añadirla
  como ruta nueva en vez de máquina nueva. Además de `boot` y nombre, la misma dirección (reemplaza el token, lo que
  arregla un token rechazado) y el mismo token del puente, por este orden: dirección, token, `boot`, nombre.
- [x] CA3 — Editar: renombrar, añadir/quitar/reordenar rutas, borrar máquina (con confirmación; borra su token del
  Keychain).
- [x] CA4 — La huella de la identidad del dispositivo (009) visible en "Este dispositivo".
- [x] CA5 — vitest de la validación del formulario; test del núcleo de "misma máquina, ruta nueva".

## Referencias
- cornercase-gpui: `design/machines.md`, `src/panels/machines/form.rs`, `src/panels/machines/rows.rs`,
  `src/dialogs/pairing.rs`, `src/ui/qr.rs`.
- t3code: `docs/user/remote-access.md`. Orca: `mobile/app/pair-scan.tsx`, `mobile/app/pair.tsx`.

## design/ que actualiza
- `design/connection/machines.md` (tema nuevo de `design/connection.md`, que pasa de 40 KB): añadir máquinas y rutas,
  cómo se detecta la misma máquina, la activa recordada, probar las demás.
- `design/look.md` (pantallas de máquinas, tono de peligro, capa del escáner, navegación),
  `design/architecture.md` (comandos nuevos, el plugin y su capability), `design/connection/bar.md` (`unauthorized`).

## Verificación
En iPhone: escanear el QR del puente (LAN) y añadir la URL de Tailscale como segunda ruta (criterio 6, parte máquinas).
