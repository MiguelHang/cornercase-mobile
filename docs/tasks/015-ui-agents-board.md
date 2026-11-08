# 015 — Pantalla 1: tablero de agentes

- Estado: pendiente
- Depende de: 014
- Riesgo: bajo
- Rama: `feat/015-agents-board` · PR: `feat: show every project's agents with the waiting ones first`

## Objetivo
La pantalla principal: los agentes de todos los proyectos con su estado, los que esperan arriba, en vivo.

## Criterios de aceptación
- [ ] CA1 — En el núcleo, `agents(session) -> Vec<AgentRow>` (proyecto, workspace, pestaña, panel, agente, título,
  estado, desde cuándo) ordenado: `waiting` > `failed` > `done` > `pending` > `working`, y dentro por antigüedad del
  estado. Tests con el fixture de `status`.
- [ ] CA2 — Comando/evento de Tauri que da la lista y sus cambios (sin mandar la sesión entera en cada cambio si no
  hace falta).
- [ ] CA3 — Pantalla en `src/routes/board/`: filas con `StatusDot`, nombre del agente (título sin el spinner del
  programa), proyecto/workspace, tiempo en el estado; secciones "Esperan", "Terminados", "Trabajando"; vacío con texto.
- [ ] CA4 — Se actualiza en vivo desde `status --follow` / `events` sin parpadeo (claves estables por panel, animación
  de reordenación).
- [ ] CA5 — Tocar una fila abre la vista de chat del agente (ruta de 016, aunque sea un esqueleto aquí).
- [ ] CA6 — vitest de formato de tiempo y agrupado por sección.

## Referencias
- cornercase-gpui: `design/session-model.md` (Loudest status, `status_rank`, `agent_title`, agents tree),
  `src/sidebar/agents.rs`, `src/sidebar/machine_agents.rs`, `src/model.rs`.
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents/status.md`, `…/events.md`,
  `design/agent-status.md`.

## design/ que actualiza
- Nuevo `design/board.md` (y su línea en `DESIGN.md`): orden, secciones, qué se ve por fila y por qué.

## Verificación
Tests verdes; en el simulador, con un agente de cornercase que pasa a esperar en el Mac, su fila sube arriba sola.
