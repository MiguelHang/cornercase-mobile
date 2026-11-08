# 007 — Núcleo: `Transport` y cliente del puente

- Estado: pendiente
- Depende de: 005
- Riesgo: bajo
- Rama: `feat/007-core-transport` · PR: `feat: run cornercase commands through a transport`

## Objetivo
Una interfaz única para ejecutar comandos de cornercase desde la app, con el puente WebSocket como primera
implementación; SSH, iroh y Cloudflare serán otras implementaciones sin tocar a quien la usa.

## Criterios de aceptación
- [x] CA1 — `crates/core/src/transport.rs`: trait `Transport` (async) con `run(argv, stdin: Stream<Line>, stdout:
  Sink<Line>) -> Result<Exit>`, donde las líneas de stderr también llegan (por un `Sink` aparte o un enum
  `Out::Stdout|Stderr`) y `Exit` lleva el código y el stderr reunido. Cancelar el futuro = cerrar el comando.
- [x] CA2 — Ayudantes encima: `output(argv) -> (stdout, stderr, code)` para comandos de una vez y `json::<T>(argv)`
  que añade nada por su cuenta (el argv ya trae `--json`) y deserializa.
- [x] CA3 — `crates/core/src/transport/bridge.rs`: implementación con tokio-tungstenite: abre `ws://…/?token=…`,
  manda `argv`, reenvía stdin, entrega stdout/stderr, termina con `exit`; un cierre sin `exit` es `Error::Dropped`;
  un 401 es `Error::Unauthorized` (distinto, la app dirá "token inválido").
- [x] CA4 — Plazo de conexión configurable (por defecto 3 s) para que 008 pueda probar rutas rápido.
- [x] CA5 — Tests: `FakeTransport` para el resto del núcleo (respuestas enlatadas por argv, streams programables) y
  tests de integración contra el puente real en proceso (`bridge` como dev-dependency, `cornercase` falso): eco,
  stderr, código de salida, 401, cierre a mitad.

## Notas técnicas
- Los errores del núcleo con `thiserror` (`crates/core/src/error.rs`).
- Sin reintentos aquí: los decide quien llama (008, 012).

## Referencias
- cornercase-gpui: `src/cli.rs` (`output`, `spawn`, `Stream`), `design/connection.md`.
- `design/connection.md` de este repo (Transport behind a trait).

## design/ que actualiza
- `design/connection.md`: forma final del trait, errores distinguidos (401 vs caída), plazo de conexión.

## Verificación
`cargo test -p cornercase-core` verde.
