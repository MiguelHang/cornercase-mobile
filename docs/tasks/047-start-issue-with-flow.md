# 047 — Arrancar una issue con un flow

- Estado: pendiente
- Depende de: 024, 045
- Rama: `feat/047-issue-flow` · PR: `feat: start an issue with one of the project's flows`

## Objetivo
Del iPhone del usuario (029, 2026-11-10): "cuando arrancas una issue, que te permita elegir un flow para lanzarla".
cornercase tiene flows (equipos de agentes con roles: orquestador, coder, reviewer…) y `flow run --issue`.

## Criterios de aceptación
- [ ] CA1 — Al arrancar desde una issue (024), el formulario ofrece "Agent" (lo de ahora) o "Flow": la lista de flows
  del proyecto que da la CLI (comprobar en upstream 0.13.29 cómo se listan: `flow list --json` o lo que haya) y, al
  elegir uno, `flow run <flow> --issue <key> [--source=…]` por la cola, con los parámetros que el flow pida y la CLI
  permita pasar.
- [ ] CA2 — Al arrancar, la app lleva al tablero con el orquestador del flow como agente "arrancando" (023) y su equipo
  plegado debajo (038); los fallos se dicen como en Start (motivo de cornercase, `Unanswered`).
- [ ] CA3 — Sin flows en el proyecto, la opción no aparece; `contract` exige lo que haga falta (`REQUIRED` o un aviso).
- [ ] CA4 — Comprobado en el simulador con un flow real; tests (argv, lectura de la lista, formulario); `hx check full` verde.

## Referencias
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents.md` (flow run, caps), los flows y sus
  roles (`design/layout/agents-section.md` › roles).
