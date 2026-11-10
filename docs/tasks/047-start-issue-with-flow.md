# 047 — Arrancar una issue con un flow

- Estado: hecha
- Depende de: 024, 045
- Rama: `feat/047-issue-flow` · PR: `feat: start an issue with one of the project's flows`

## Objetivo
Del iPhone del usuario (029, 2026-11-10): "cuando arrancas una issue, que te permita elegir un flow para lanzarla".
cornercase tiene flows (equipos de agentes con roles: orquestador, coder, reviewer…) y `flow run --issue`.

## Criterios de aceptación
- [x] CA1 — Al arrancar desde una issue (024), el formulario ofrece "Agent" (lo de ahora) o "Flow": la lista de flows
  del proyecto que da la CLI (comprobar en upstream 0.13.29 cómo se listan: `flow list --json` o lo que haya) y, al
  elegir uno, `flow run <flow> --issue <key> [--source=…]` por la cola, con los parámetros que el flow pida y la CLI
  permita pasar.
- [x] CA2 — Al arrancar, la app lleva al tablero con el orquestador del flow como agente "arrancando" (023) y su equipo
  plegado debajo (038); los fallos se dicen como en Start (motivo de cornercase, `Unanswered`).
- [x] CA3 — Sin flows en el proyecto, la opción no aparece; `contract` exige lo que haga falta (`REQUIRED` o un aviso).
- [x] CA4 — Comprobado en el simulador con un flow real; tests (argv, lectura de la lista, formulario); `hx check full` verde.

## Decisiones
Tomadas el 2026-11-10 con los valores por defecto propuestos, sin respuesta del usuario (`design/starting-work.md` ›
From an issue › With a flow):
- Los "parámetros que el flow pida": un flow de upstream no declara entradas; la CLI solo deja pasar la tarea
  (`--prompt`), la rama, el proyecto, el nombre del workspace y perfiles o cuentas. Se mandan el proyecto (por `Where`)
  y la rama; el resto queda fuera.
- P1: sin tarea propia (`--prompt`, que sustituye al prompt de la issue): el prompt de cornercase se ve plegado y de solo
  lectura, como con un agente.
- P2: tras arrancar, la pantalla del orquestador con `Starting <agent>…` y `Back` al tablero, donde lleva a su equipo
  plegado debajo; no se aterriza en el tablero.
- P3: solo el selector `Agent` / `Flow` en el formulario de la issue, sin un segundo botón en la issue.
- `Flow` aparece solo con un sitio de nueva worktree y al menos un flow en la lista de su proyecto (o una lectura fallida,
  con `Try again`); mientras se lee no se muestra. La lista va por el transporte, de una vez, y el run por la cola.
- `flow-list` y `flow-run` en `contract::REQUIRED` (ya están en 0.13.19).

## design/ que actualiza
- `design/starting-work.md`: From an issue › With a flow, las comprobaciones de The screen, Checked live y Left out.
- `DESIGN.md` (los ficheros nuevos en la línea de Starting work), `design/connection.md` (`REQUIRED`),
  `design/architecture.md` (`issue_flows`, `start_flow`) y `design/look.md` (`Segmented`, `FlowPicker`).

## Verificación
2026-11-10, simulador iPhone 17 (iOS 27.2) con la app y el puente de la rama, cornercase 0.13.31 en un servidor aislado
(`HOME` falso, `gh` y agentes de mentira) y los toques por XCUITest (`design/starting-work.md` › Checked live):
- `Flow` en el formulario de la issue, `Where` solo con la nueva worktree, la lista en el menú de iOS (opciones largas
  en varias líneas, la que tiene problemas deshabilitada), el equipo bajo el selector y el último flow recordado.
- Arrancar `Quick` (built-in): worktree, workspace y una pestaña por rol; `Starting claude…`, la pantalla del
  orquestador y `Back` al tablero con el `coder` plegado debajo.
- Con un proyecto que no es raíz de un repo, `Flow` no aparece.
- El orquestador arranca el último (`launch_run` de upstream): el coder salió como agente ~1,1 s antes, así que el
  tablero lo pondría como raíz ese rato. No es un hueco de upstream (`status` ya da el `role` del panel del
  orquestador), así que `design/upstream-gaps.md` no cambia.
- `hx check full` verde.

## Referencias
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents.md` (flow run, caps), los flows y sus
  roles (`design/layout/agents-section.md` › roles).
