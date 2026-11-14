# 057 — Upstream: el QR para emparejar la app y `/cli` en cornercase-terminal

- Estado: pendiente · issue [usecornercase/cornercase-terminal#838](https://github.com/usecornercase/cornercase-terminal/issues/838), asignada al usuario, que la implementa en upstream como colaborador (decidido el 2026-11-14)
- Depende de: — (comprobado contra upstream `main`, cornercase 0.13.72, el 2026-11-14)
- Repo: cornercase-terminal (no este). Sustituye al borrador 01 de la 033 (`.harness/parked/t033/drafts/01-cli-endpoint.md`).

## Objetivo
Quitar el bridge: que cornercase sirva él mismo lo que hoy sirve `bridge/`, y que el QR para añadir el Mac lo enseñe
cornercase. Del usuario (2026-11-14): "seguir emparejando por QR, que el cornercase terminal ofrezca el QR para
emparejar"; "para escanear desde la app y que ofrezca los enlaces de conexión como el bridge (LAN y Tailscale por
ahora)".

## Lo que hay en upstream (0.13.72)
- **Acceso remoto del navegador** (`src/web/`): Settings › Remote y `cornercase config pair browsers --json` enseñan un QR
  con `https://<nodo>.ts.net:8443/pair?…`. Abre la versión web (`/ws` sirve la ventana a xterm.js, no el CLI), solo por
  Tailscale. No sirve a la app: no hay `/cli` ni LAN.
- **Sin `/cli`**: `src/web/mod.rs::route` solo tiene `/`, `/pair`, `/ws` y los assets; `contract --json` no tiene
  `cli-endpoint`.
- `--boot` en casi todos los comandos (el borrador 07 de la 033 ya está hecho).

## Qué se pide a upstream
1. **Un QR para la app**, aparte del del navegador: una fila **phone app** en Settings › Remote y
   `cornercase config pair app [--json]` (como `config pair browsers`: imprime el código, espera, acaba con
   `{"paired": true}`). El QR lleva un código de máquina con **todas las rutas** (LAN y Tailscale), el puerto, el nombre
   del Mac (`scutil --get ComputerName`) y un código de emparejamiento de un solo uso que caduca a los 10 minutos.
   El formato, el de `design/bridge/machine-code.md` (`ws://<primera>:<puerto>/?…&also=<host>&name=<nombre>`), con el
   código de emparejamiento en lugar del token fijo.
2. **Emparejar**: la app canjea el código por un secreto propio del dispositivo (32 bytes, guardado en su Keychain o
   Keystore). cornercase lo guarda como guarda los navegadores (`browsers.json` o un `apps.json` al lado) y Settings ›
   Remote lista las apps emparejadas y las olvida.
3. **`/cli`**: un WebSocket por comando con el protocolo del bridge (`design/bridge.md` › Protocol: `argv`, `stdin`,
   `stdin_base64`, `eof`, `stdout`, `stderr`, `exit`, parseo estricto, una sola forma de rechazar con exit 2, ping cada
   15 s y cierre a los 45 s de silencio, matar el grupo al cerrar). Lo que puede correr sale de `reach::kind`
   (`Kind::There` menos la ventana y `start-server`, sin `--remote*`), no de una lista a mano.
4. **Dónde escucha**: en las direcciones de la LAN y de Tailscale, en un puerto propio, solo con el acceso de la app
   activado en Settings. `ws://` como el bridge, por ahora; ver Riesgos.
5. **Las rutas tras conectar**: el equivalente de `--bridge-routes` (las direcciones en las que escucha ahora), para que
   la app añada sola una red que aparece después (035). Por ejemplo, `config pair app --routes --json`.
6. **`cli-endpoint` en `contract --json`**.

## Criterios de aceptación (de upstream)
- [ ] El QR de **phone app** y `config pair app --json` dan un código con las rutas de LAN y Tailscale; escanearlo con la
  app la empareja y el diálogo lo dice.
- [ ] `/cli` con el secreto de una app emparejada corre el argv y acaba con un único `{"exit": n}`; sin él, `403` y nada
  corre.
- [ ] Lo que no es `Kind::There`, la ventana, `start-server` y cualquier `--remote*` se rechazan con exit 2.
- [ ] Cerrar, perder la conexión o 45 s de silencio matan el comando y su grupo; un cliente que no lee bloquea el
  comando en vez de hacer crecer la memoria.
- [ ] Olvidar una app en Settings cierra sus conexiones.
- [ ] `contract --json` lista `cli-endpoint`.

## Riesgos
- **`ws://` en la LAN**: el secreto viaja en claro por la wifi, igual que el token del bridge. Por Tailscale va cifrado
  por WireGuard. Siguiente paso (fuera de esta tarea): TLS con certificado autofirmado y su huella en el QR, para que la
  app lo fije sin CA.
- **Que upstream no lo acepte o tarde**: la v0.1.0 no sale sin esto. Mientras, la app se prueba contra el fork.

## Referencias
- Borrador 01 de la 033 (protocolo, límites, alternativas), que se reescribe con este diseño: QR propio de la app, LAN y
  Tailscale, secreto por dispositivo en vez de la cookie del navegador.
- cornercase-mobile: `design/bridge.md`, `design/bridge/machine-code.md`, `crates/core/src/protocol.rs`,
  `crates/core/src/machines/code.rs`.
- cornercase-terminal 0.13.72: `src/web/mod.rs`, `src/web/auth.rs`, `src/web/bridge.rs`, `src/cli/reach.rs`,
  `src/cli/help/pair.rs`, `src/ui/remote_access.rs`.
