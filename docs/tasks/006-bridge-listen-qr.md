# 006 — Puente: solo LAN/Tailscale, token, URL y QR

- Estado: hecha
- Depende de: 005
- Riesgo: medio (no exponerse nunca en una IP pública)
- Rama: `feat/006-bridge-listen` · PR: `feat: listen on lan and tailscale only and print a qr code`

## Objetivo
Arrancar el puente sin opciones y que escuche solo donde debe, con un token y un QR por interfaz para la app.

## Criterios de aceptación
- [x] CA1 — Función pura `eligible(addr) -> Option<Kind>` (`Lan` | `Tailscale`): IPv4 privadas RFC 1918
  (10/8, 172.16/12, 192.168/16), Tailscale 100.64.0.0/10 y `fd7a:115c:a1e0::/48`; rechaza loopback (salvo
  `--loopback` para pruebas), link-local, `0.0.0.0`, `::` y cualquier dirección pública. Tests con tabla de casos.
- [x] CA2 — Al arrancar enumera interfaces (`if-addrs` o similar), abre un listener por dirección elegible en el mismo
  puerto (por defecto fijo, p. ej. 8765, o `--port`) y nunca hace bind a una dirección comodín. Sin direcciones
  elegibles: no arranca y lo dice.
- [x] CA3 — Token aleatorio de 32 bytes (`getrandom`), en base64url, guardado en `~/.config/cornercase-bridge/token`
  (`0600`) para que la URL no cambie entre arranques; `--new-token` lo rota.
- [x] CA4 — Imprime por interfaz su tipo, `ws://<ip>:<puerto>/?token=…` y un QR en la terminal (crate `qrcode`,
  medios bloques, zona tranquila de 4 módulos, blanco y negro fijos).
- [x] CA5 — Desaparece la opción temporal `--listen` de 005 (o queda solo junto a `--loopback` para tests).

## Notas técnicas
- Una IP que aparece después (te unes a Tailscale con el puente arrancado) no se escucha hasta reiniciar: anotarlo.
- Formato de la URL pensado para 008 (una URL = una ruta) y 027 (el QR contiene exactamente esa URL).

## Referencias
- cornercase-terminal (`origin/main`): `design/browser-access.md` (QR en medios bloques, zona tranquila, colores
  fijos), `src/web/tailscale.rs`, `src/web/mod.rs`.
- t3code: `docs/user/remote-access.md` (rutas por máquina, emparejamiento por QR).

## design/ que actualiza
- `design/bridge.md`: rangos elegibles, puerto, dónde vive el token y por qué persiste, límite de interfaces nuevas.

## Verificación
Tests verdes; a mano: el puente en el Mac imprime la URL de la LAN y la de Tailscale; `websocat` desde otra máquina
de la tailnet ejecuta `contract --json`.

## Notas de la review de 005
- La plaza de conexión se toma al aceptar el TCP, antes de autenticar: con LAN, tomarla tras el upgrade (o un tope aparte para handshakes) y acotar las conexiones que reciben 503.
- `--token` se ve en `ps`; el token persistente lo resuelve.
- Memoria en el peor caso por conexión (~16 × 16 MiB de canal + frame de hasta ~96 MiB): anotarlo si el puente deja de ser solo de desarrollo.
