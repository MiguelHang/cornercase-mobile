# 005 — Puente: servidor WebSocket con token

- Estado: hecha
- Depende de: 004
- Riesgo: medio
- Rama: `feat/005-bridge-websocket` · PR: `feat: serve one cornercase command per websocket`

## Objetivo
El puente acepta WebSockets, exige el token antes del upgrade y une cada conexión a un único comando.

## Criterios de aceptación
- [x] CA1 — `bridge/src/server.rs` con tokio-tungstenite: `accept_hdr_async` comprueba `?token=` en tiempo constante;
  sin token o con uno malo responde `401` y no hay upgrade.
- [x] CA2 — Por conexión: el primer mensaje debe ser `argv` (si no, `stderr` + `exit 2` y cierre); argv no permitido
  → `stderr` + `exit` sin lanzar nada; permitido → `spawn`, se reenvían `stdin` al proceso y `stdout`/`stderr` al
  cliente, `exit` al terminar y cierre normal.
- [x] CA3 — Si el cliente cierra o la conexión cae, el proceso y su grupo mueren (de 004). Ping cada 15 s y se da por
  muerta tras 45 s sin respuesta (valores de `browser-access.md` para datos móviles).
- [x] CA4 — Contrapresión: si el cliente no lee, el puente deja de leer el stdout del hijo (cola acotada), no acumula
  memoria sin límite.
- [x] CA5 — Tests de integración en `bridge/tests/` con el `cornercase` falso y un cliente tungstenite: token ausente y
  malo → 401; eco completo con `exit 0`; argv rechazado; primer mensaje que no es `argv`; cerrar el cliente mata el
  proceso colgado; dos conexiones a la vez no se mezclan.
- [x] CA6 — `cargo run -p cornercase-bridge -- --listen 127.0.0.1:0 --token t` (opciones temporales hasta 006) y
  `websocat` permiten probarlo a mano; los pasos van en `design/bridge.md`.

## Notas técnicas
- `main.rs` solo parsea argumentos (clap) y llama a la lib; la lib es lo que testean 007 y los tests de integración.
- Log a stderr con lo que se acepta y rechaza; nunca el token.

## Referencias
- cornercase-terminal (`origin/main`): `src/web/http.rs`, `src/web/auth.rs` (comprobación antes del upgrade, tiempo
  constante), `src/web/bridge.rs` (ping, silencio, tamaño máximo de mensaje), `design/browser-access.md`.

## design/ que actualiza
- `design/bridge.md`: ping/silencio, contrapresión, límite de tamaño de mensaje, cómo probarlo con `websocat`.

## Verificación
`cargo test -p cornercase-bridge` verde; prueba manual con `websocat` contra el cornercase real del Mac
(`contract --json` devuelve su JSON y `exit 0`).
