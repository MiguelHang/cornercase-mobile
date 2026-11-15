# cornercase-mobile — idea y requisitos

## Problema
Los agentes de cornercase corren en el Mac. Fuera del escritorio no hay forma de ver cuáles esperan,
contestarles ni lanzar trabajo nuevo. Los avisos de `away` (Telegram/Discord) dicen que algo pasa,
pero no dejan actuar.

## Usuario
El propio autor (y después, quien use cornercase) con cornercase corriendo en uno o varios Macs, que
quiere seguir y conducir sus agentes desde el iPhone o un Android.

## Qué es
App móvil (iOS + Android) en Tauri 2. Es un cliente del servidor de cornercase que habla con él
**solo a través de su CLI**, el contrato público, igual que `usecornercase/cornercase-gpui`.

- Repo `cornercase-mobile`, privado en la cuenta personal; más adelante se transfiere a
  `usecornercase` conservando el histórico.
- Bundle id definitivo desde el inicio: `dev.usecornercase.mobile`.
- Licencia: `MIT OR Apache-2.0`, como cornercase-terminal.

## Referencias obligatorias
- cornercase-gpui (`usecornercase/cornercase-gpui`):
  - Aspecto: `design/widgets.md` y `theme.rs` (colores de usecornercase.dev, cristal sobre un fondo,
    formas redondeadas con aire, cobalto, el logo, animaciones).
  - Conexión: `design/connection.md` (el CLI como único contrato, streams, cola ordenada, Store).
  - Terminal: `design/session-model.md` (JSON de `view` y `surface`, aplicar frames y patches).
- cornercase-terminal (`usecornercase/cornercase-terminal`, rama `main` de upstream; la copia local
  `~/me/cornercase-terminal` está por detrás): `src/cli/reach.rs`, `design/remote-windows.md`
  (--remote, PATH, quoting, banner), `design/browser-access.md` (emparejamiento, Tailscale, teclado y
  swipe en móvil), `design/commands-for-scripts-and-agents.md` y sus temas (contract, surfaces, views,
  driving-terminals, status, events, reading, pasting-files, changes, files…).
- t3code (`pingdotgg/t3code`: `docs/user/remote-access.md`, rutas por máquina y emparejamiento).
- Orca (`stablyai/orca`: la app móvil en `mobile/`).

## Fase 0 — puente de desarrollo (`bridge/`)
Binario Rust en el workspace que corre en el Mac. Solo para desarrollo. Retirado en 059: desde 058 la app habla con
el `/cli` de cornercase, que sirve este protocolo (`design/bridge.md`).

- WebSocket ↔ `cornercase <args>`. **Una conexión WebSocket por comando.**
- Protocolo (JSON, un objeto por mensaje):
  1. Cliente → `{"argv": [...]}` (primer mensaje, obligatorio).
  2. Cliente → `{"stdin": "<línea>"}`; puente → `{"stdout": "<línea>"}`, línea a línea.
  3. Puente → `{"exit": n}` al terminar y cierra.
  - Puente → `{"stderr": "<línea>"}`, también línea a línea, para que la app muestre los errores
    del CLI.
  - Cliente → `{"stdin_base64": "…"}` para bytes (imágenes de `paste-file -`) y `{"eof": true}`
    para cerrar el stdin (`send -`, `paste-file -`).
- Lista permitida: los comandos `Kind::There` de `src/cli/reach.rs` (subcomandos de control, `stats`
  y los `plugin` que consultan al servidor). **Fuera**: `kill-server`, `update`, `restart`, `logs`,
  `cornercase` sin subcomando (abre la TUI) y `start-server`. Un argv fuera de la lista se rechaza
  con un `{"exit": n}` y un mensaje en `stderr`, sin lanzar nada.
- Si el móvil se desconecta, mata el proceso (y su grupo).
- Autenticación: token fijo en la URL (`?token=`); sin él, se rechaza antes del upgrade.
- Escucha solo en las interfaces LAN o Tailscale (nunca en `0.0.0.0` ni en una IP pública).
  Sin TLS ni emparejamiento.
- Al arrancar imprime un QR y la URL `ws://<ip>:<puerto>/?token=…` de cada interfaz.
- Objetivo: fijar el formato que luego se implementará como `/cli` en `src/web/` de
  cornercase-terminal. **Tarea final**: documentar ese formato como propuesta para upstream.

## Núcleo Rust de la app
- **Toda la red vive en Rust**, nunca en el WebView. El WebView recibe datos con tipos y eventos.
- **Máquinas guardadas**, cada una con varias rutas (en la fase 0: URLs del puente; después SSH, iroh
  y Cloudflare como rutas nuevas). Usa la primera ruta que responde; vuelve a probar al cambiar de
  red o al volver la app a primer plano.
- **Transporte detrás de una interfaz**: `run(argv, stdin, stdout) → código de salida`, con stdin y
  stdout como streams de líneas. El puente WebSocket es la primera implementación.
- **Identidad Ed25519 desde el inicio**, guardada en Keychain (iOS) o Keystore (Android), para
  reutilizarla luego como identidad de iroh. En la fase 0 se genera y guarda, pero el puente no la usa.
- Al conectar pide `contract --json` y guarda `boot`; un `boot` distinto = servidor reiniciado →
  se reconstruye el Store.
- Comandos con tipos sobre la salida `--json` del CLI, expuestos al WebView como comandos y eventos
  de Tauri con tipos TypeScript generados.
- Conexión según `connection.md` de gpui: streams, cola ordenada de peticiones, Store único.

## UI (TypeScript en el WebView) — MVP
1. **Tablero de agentes** de todos los proyectos con su estado (trabajando, esperando, terminado),
   con los que esperan arriba. Fuente: `status --follow --json` y `events`.
2. **Vista de agente en modo chat**: `read --last-message`; responder con `send --enter` y `keys`
   (Esc, Tab, flechas, Ctrl-C, números); adjuntar imágenes con `paste-file`.
3. **Terminal en vivo**: `view` con el tamaño del móvil + `surface --patches` + `input` + `scroll`, y
   una fila con las teclas que le faltan al teclado del móvil.
   - Por defecto **ajustada al móvil** (043): al abrirse reclama la pestaña con una `view` del tamaño del
     móvil, que redimensiona la ventana del Mac mientras se mira; al salir, la suelta. Los frames se
     filtran por `projection`.
   - Botón **"ajustar a este móvil"**, encendido al abrir; apagado, la vista es **pasiva**: no reclama la
     pestaña y muestra el PTY con el ancho del Mac, con zoom y desplazamiento horizontal, sin tocar su ventana.
4. **Arrancar un agente**: `start --worktree` con prompt, agente y modelo; desde una issue
   (`issue list` / `issue read`); `new-workspace`.
5. **Cambios del workspace** (`changes list/read`, marcar como visto) y **ficheros**
   (`files list/read/search`).
6. **Máquinas**: añadir escaneando el QR del puente o pegando la URL; varias rutas por máquina.

## Fuera del MVP
- Notificaciones push (en las fases 0-1 los avisos van por Telegram/Discord con `away`).
- Relay (iroh o Cloudflare) y rutas SSH.
- El endpoint `/cli` dentro de cornercase (solo se documenta la propuesta).
- Emparejamiento (el puente usa un token fijo).
- Historial completo de la conversación y aprobaciones como tarjetas (cornercase solo da el último
  mensaje).
- Commit/push desde el móvil.

## Requisitos no funcionales
- Código (Rust y TypeScript): sin comentarios; ≤ 1.000 líneas por fichero; funciones pequeñas;
  en Rust `pub(crate)` por defecto.
- Decisiones en `design/` por áreas, con `DESIGN.md` como índice (como gpui y cornercase-terminal).
- La terminal aplica patches sin bloquear la UI; los streams se reabren solos tras perder la red o
  volver de segundo plano.
- El puente nunca escucha fuera de LAN/Tailscale y nunca ejecuta un comando fuera de la lista.
- La app comprueba en `contract` que el cornercase del Mac tiene los comandos que usa y dice cuáles
  faltan, en vez de fallar a ciegas.

## Stack
- **Tauri 2** (iOS + Android), Rust estable.
- **Svelte 5 + Vite + TypeScript** en el WebView: bundle pequeño, runes y transiciones integradas
  para el cristal y las animaciones de gpui. pnpm.
- Workspace Cargo: `crates/core` (lógica pura y testeable: transporte, protocolo, Store, tipos del
  CLI, máquinas y rutas), `src-tauri` (comandos y eventos de Tauri, almacenamiento de claves) y,
  hasta 059, `bridge/` (binario del puente).
- Red: tokio + tokio-tungstenite. Claves: ed25519-dalek. Tipos hacia TS: tauri-specta.
  QR: el plugin oficial `barcode-scanner` en la app y, hasta 059, un crate de QR para la terminal en el puente.
- Plataforma: **primero iOS** (simulador y iPhone); Android en una tarea propia (Keystore, teclado).
- CI: GitHub Actions en Linux en cada PR: fmt, clippy y tests de Rust; svelte-check, lint y vitest.
  Sin compilaciones móviles en CI.

## Criterios de éxito del MVP
Desde un iPhone, por Tailscale y con el puente corriendo en el Mac:
1. El tablero muestra en vivo los agentes de todos los proyectos, con los que esperan arriba.
2. Se contesta a un agente que espera (texto, tecla o imagen) y se ve que sigue trabajando.
3. Se abre su terminal en vivo, ya al ancho del móvil, y se escribe y se desplaza; apagando "ajustar a
   este móvil" se ve al ancho del Mac sin cambiar el tamaño de su ventana.
4. Se arranca un agente en un worktree desde una issue.
5. Se ven los cambios y los ficheros de un workspace y se marcan cambios como vistos.
6. Al salir y volver a la app, o al pasar de wifi a datos, todo se reconecta solo.
7. El formato del puente está documentado como propuesta de `/cli` para cornercase-terminal.

## Riesgos
- **Keystore de Android desde Rust**: `keyring` cubre el Keychain de iOS; Android necesitará JNI o un
  plugin propio de Tauri en Kotlin.
- **Detectar cambios de red** necesita código nativo (NWPathMonitor en iOS, ConnectivityManager en
  Android). Plan B: reintentar al volver a primer plano y al fallar un stream.
- **iOS suspende los sockets en segundo plano**: hay que reabrir los streams al volver y
  reconstruir el Store con `boot`.
- **Teclado y gestos en el WebView** (foco, teclado virtual, swipe): `browser-access.md` de
  cornercase-terminal ya documenta problemas parecidos.
- **Versión de cornercase en el Mac**: los comandos del MVP están en el `main` de upstream; una
  versión anterior no los tiene (mitigado con la comprobación de `contract`).
- **Perfil de Apple gratuito**: sin cuenta de pago, la app instalada en el iPhone caduca cada 7 días.
- **Token en claro sin TLS**: aceptable solo porque el puente es de desarrollo y escucha en
  LAN/Tailscale.
