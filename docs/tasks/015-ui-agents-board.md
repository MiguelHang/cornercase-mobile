# 015 — Pantalla 1: tablero de agentes

- Estado: hecha, salvo la comprobación en el simulador
- Depende de: 014
- Riesgo: bajo
- Rama: `feat/015-agents-board` · PR: `feat: show every project's agents with the waiting ones first`

## Objetivo
La pantalla principal: los agentes de todos los proyectos con su estado, los que esperan arriba, en vivo.

## Criterios de aceptación
- [x] CA1 — En el núcleo, `agents(session) -> Vec<AgentRow>` (proyecto, workspace, pestaña, panel, agente, título,
  estado, desde cuándo) ordenado: `waiting` > `failed` > `done` > `pending` > `working`, y dentro por antigüedad del
  estado. Tests con el fixture de `status`. `shell` va con `working` e `idle` al final; el fixture de `status` no
  tiene agentes, así que el orden se prueba con el de `watch` y snapshots construidos (`design/board.md` › Order).
- [x] CA2 — Comando/evento de Tauri que da la lista y sus cambios (sin mandar la sesión entera en cada cambio si no
  hace falta). `agents` y `agents-changed`, que solo sale si cambian las filas; `session` y `session-changed`
  desaparecen (`design/architecture.md` › Commands and events).
- [x] CA3 — Pantalla en `src/routes/board/`: filas con `StatusDot`, nombre del agente (título sin el spinner del
  programa), proyecto/workspace, tiempo en el estado; secciones "Esperan", "Terminados", "Trabajando"; vacío con texto.
  Las secciones van en inglés, como el resto de la UI (Waiting, Done, Working), y una cuarta, Idle, al final
  (`design/board.md` › Sections).
- [x] CA4 — Se actualiza en vivo desde `status --follow` / `events` sin parpadeo (claves estables por panel, animación
  de reordenación). Una sola lista con las cabeceras, para que `flip` mueva una fila de una sección a otra
  (`design/board.md` › Live, without flicker).
- [x] CA5 — Tocar una fila abre la vista de chat del agente (ruta de 016, aunque sea un esqueleto aquí). La ruta
  `agent` y `settle`, que no la pisa con cada cambio (`design/look.md` › Navigation).
- [x] CA6 — vitest de formato de tiempo y agrupado por sección.

## Referencias
- cornercase-gpui: `design/session-model.md` (Loudest status, `status_rank`, `agent_title`, agents tree),
  `src/sidebar/agents.rs`, `src/sidebar/machine_agents.rs`, `src/model.rs`.
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents/status.md`, `…/events.md`,
  `design/agent-status.md`.

## design/ que actualiza
- Nuevo `design/board.md` (y su línea en `DESIGN.md`): orden, secciones, qué se ve por fila y por qué.
- `design/look.md` (Components, Navigation), `design/architecture.md` (Commands and events),
  `design/connection/store.md` (Since when, the forwarder) y `design/upstream-gaps.md` (desde cuándo).

## Verificación
Tests verdes; en el simulador, con un agente de cornercase que pasa a esperar en el Mac, su fila sube arriba sola.
