# 012 — Núcleo: `Store` y streams que se reabren solos

- Estado: hecha
- Depende de: 008, 010, 011
- Riesgo: medio (iOS suspende los sockets en segundo plano)
- Rama: `feat/012-core-store` · PR: `feat: keep one store per machine fed by streams that reopen`

## Objetivo
Un `Store` por máquina conectada que mantiene el estado vivo a partir de `status --follow --json` (y `--app`) y
`events --json`, que se reabren solos y se reconstruye cuando cambia `boot`.

## Criterios de aceptación
- [x] CA1 — `crates/core/src/cli/status.rs`: tipos serde (`#[serde(default)]`) de lo que el MVP lee de
  `status --json`: grupos, proyectos, workspaces, pestañas, paneles (`agent`, `status`, `name`, `path` opcional,
  `parent`), `boot`, `machines` ignorado. `crates/core/src/cli/events.rs`: eventos v1 (`status`, `opened`, `closed`,
  `exited`, `notice`… con variante `Other` para los desconocidos).
- [x] CA2 — `crates/core/src/store.rs`: estado (sesión + `app`), conexión (`Connecting | Connected{route} |
  Retrying{in} | Missing{ops} | Unauthorized`), y una suscripción de cambios (canal) que 013 convierte en eventos de Tauri.
- [x] CA3 — Ciclo: `pick_route` (008) → `contract` (010) → si faltan operaciones, `Missing` y para → abre streams.
  Si un stream acaba o falla: backoff 1, 2, 4, 8 y luego cada 15 s, y vuelve a elegir ruta; `resume_now()` lo salta
  (para primer plano y cambio de red).
- [x] CA4 — `boot` distinto al de antes: se vacía el estado, se llama a `queue.reconnect()` (011) y se reabren los
  streams; nada viejo (ids) sobrevive al reinicio.
- [x] CA5 — Fixtures de `tests/fixtures/control/view/v1/snapshot.json`, `events/v1/*.json` y un `status --json` real
  (anotar versión); tests con `FakeTransport`: arranque, caída y backoff, `resume_now`, cambio de `boot`, `Missing`.

## Notas técnicas
- Un solo `Store` de verdad por máquina; la UI pide lecturas puntuales (`read`, `changes`, `files`) por comandos, no
  las guarda aquí salvo que una pantalla las necesite vivas.
- `status --follow --json` frente a `watch`: usar lo que use gpui hoy (anotar cuál y por qué).

## Referencias
- cornercase-gpui: `src/store.rs`, `src/app/connection.rs` (reconexión cada 1,5 s, `follow_app`), `src/model.rs`,
  `design/session-model.md`, `design/connection.md` (Store).
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents/status.md`, `…/events.md`,
  `…/watching.md`, `design/browser-access.md` (backoff 1-2-4-8-15 s y reconexión inmediata al volver).
- Orca: `mobile/issue-5049-unresponsive-session-findings.md` (sesiones que dejan de responder en móvil).

## design/ que actualiza
- `design/connection.md`: estados de conexión, backoff, qué reinicia `boot`, qué streams se abren.

## Verificación
`cargo test -p cornercase-core` verde.
