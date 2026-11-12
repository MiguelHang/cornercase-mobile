# 056 — Cerrar un agente desde el teléfono

- Estado: pendiente
- Depende de: 051
- Rama: `feat/056-close-agent` · PR: `feat: close an agent from the phone`

## Objetivo
Del iPhone del usuario (2026-11-12): "añadir la opción de eliminar un agente, de cerrarlo".

Upstream ya lo tiene: `cornercase close --pane P --agent` cierra el agente como el "close agent" de la ventana (su
pestaña y las de los agentes que arrancó en el mismo workspace, a cualquier profundidad; no borra nada del disco), con
`--check --json` para saber antes qué se cerraría y `--boot` para no actuar sobre otra ejecución del servidor.

## Criterios de aceptación
- [ ] CA1 — En la pantalla del agente, una acción `Close agent` (en la cabecera, en un menú o donde encaje con
  `design/look.md`) y en el tablero el gesto de iOS de deslizar la fila a la izquierda con `Close` en rojo (en Android,
  pulsación larga o el equivalente). Decidir con capturas y anotarlo en `design/`.
- [ ] CA2 — Antes de cerrar se pide confirmación con lo que dice `close --pane P --agent --check --json` (cuántas
  pestañas o subagentes se cierran con él); la acción va en el tono de peligro (danger) de la app.
- [ ] CA3 — Se manda `close --pane P --agent --boot <boot>` por la cola; mientras, la fila o la pantalla dicen que se
  está cerrando; al acabar, si se estaba en su pantalla se vuelve al tablero; un fallo muestra la razón de cornercase.
- [ ] CA4 — Nada de borrar worktrees: solo cerrar el agente (sin `--remove-worktree`); anotado por qué.
- [ ] CA5 — Fixtures reales de `close --agent --check --json` y `--json`; tests del argv y de la lógica; comprobado en
  el simulador; `design/` actualizado; `hx check full` verde.
