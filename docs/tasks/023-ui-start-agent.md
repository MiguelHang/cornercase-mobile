# 023 — Pantalla 4: arrancar un agente (`start --worktree`, `new-workspace`)

- Estado: pendiente
- Depende de: 015
- Riesgo: bajo
- Rama: `feat/023-start-agent` · PR: `feat: start an agent in its own worktree from the phone`

## Objetivo
Lanzar trabajo nuevo: un agente con prompt, agente y modelo en un worktree propio, o un workspace nuevo.

## Criterios de aceptación
- [ ] CA1 — Formulario (sheet): proyecto (de la sesión), agente (los que cornercase conoce y están instalados en el
  Mac, leídos del CLI; no una lista fija en la app), modelo (opcional, según el agente), prompt multilínea, rama/nombre
  del worktree (opcional).
- [ ] CA2 — `start --worktree … --placed --json` por la cola (con `--placed` para no quedar esperando la pregunta de
  confianza de una carpeta); al responder, se abre el chat del panel nuevo (016).
- [ ] CA3 — Acción aparte "workspace nuevo" → `new-workspace` con nombre y, opcional, worktree.
- [ ] CA4 — Validación antes de mandar (proyecto y prompt obligatorios) y errores del servidor visibles sin perder lo
  escrito.
- [ ] CA5 — Tests: argv que construye el núcleo para cada combinación (con y sin modelo, con y sin rama).

## Referencias
- cornercase-gpui: `src/sidebar/forms.rs`, `src/sidebar/forms/agents.rs`, `design/connection.md` (`start --focus
  --placed` y por qué), `design/worktrees.md`.
- cornercase-terminal (`origin/main`): `src/cli/args.rs` (`StartArgs`, `NewWorkspaceArgs`),
  `design/commands-for-scripts-and-agents/worktrees.md`, `design/agents.md`.

## design/ que actualiza
- Nuevo `design/starting-work.md` (y su línea en `DESIGN.md`): de dónde salen agentes y modelos, `--placed`.

## Verificación
En el simulador se arranca un agente en un worktree y aparece en el tablero trabajando.
