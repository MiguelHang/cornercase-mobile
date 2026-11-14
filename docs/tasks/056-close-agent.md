# 056 — Cerrar un agente desde el teléfono

- Estado: hecha (comprobada en el iPhone del usuario el 2026-11-14)
- Depende de: 051
- Rama: `feat/056-close-agent` · PR: `feat: close an agent from the phone`

## Objetivo
Del iPhone del usuario (2026-11-12): "añadir la opción de eliminar un agente, de cerrarlo".

Upstream ya lo tiene: `cornercase close --pane P --agent` cierra el agente como el "close agent" de la ventana (su
pestaña y las de los agentes que arrancó en el mismo workspace, a cualquier profundidad; no borra nada del disco), con
`--check --json` para saber antes qué se cerraría y `--boot` para no actuar sobre otra ejecución del servidor.

## Criterios de aceptación
- [x] CA1 — En la pantalla del agente, una acción `Close agent` (en la cabecera, en un menú o donde encaje con
  `design/look.md`) y en el tablero el gesto de iOS de deslizar la fila a la izquierda con `Close` en rojo (en Android,
  pulsación larga o el equivalente). Decidir con capturas y anotarlo en `design/`.
- [x] CA2 — Antes de cerrar se pide confirmación con lo que dice `close --pane P --agent --check --json` (cuántas
  pestañas o subagentes se cierran con él); la acción va en el tono de peligro (danger) de la app.
- [x] CA3 — Se manda `close --pane P --agent --boot <boot>` por la cola; mientras, la fila o la pantalla dicen que se
  está cerrando; al acabar, si se estaba en su pantalla se vuelve al tablero; un fallo muestra la razón de cornercase.
- [x] CA4 — Nada de borrar worktrees: solo cerrar el agente (sin `--remove-worktree`); anotado por qué.
- [x] CA5 — Fixtures reales de `close --agent --check --json` y `--json`; tests del argv y de la lógica; comprobado en
  el simulador; `design/` actualizado; `hx check full` verde.
  - En el iPhone 17 del simulador (iOS 27.2, cornercase 0.13.31, un servidor aislado con agentes falsos): deslizar una
    fila descubre `Close` en rojo y un arrastre vertical no la abre; `Cancel` no manda nada más; una raíz con dos
    subagentes manda check, check otra vez y cierre, los tres con el `--boot` de la fila, dice `Closing…` y salen las
    tres filas; un miembro de flow avisa de que el equipo entero se cierra; `Close agent` de la pantalla vuelve al
    tablero, también con la terminal encima y desde un subagente sobre su padre, sin que asome `This agent is no longer
    running.`; un `--boot` de otro arranque (reescrito por un envoltorio, porque los agentes falsos no sobreviven a un
    reinicio de verdad) da exit 4 y un toast con la razón. En el emulador de Android (API 37), la pulsación larga abre
    la misma hoja; el cierre no se repitió allí y la vibración no se nota en un emulador. Sin probar: tocar fuera para
    cerrar la fila abierta y `Unanswered`. `design/board/closing.md` › Checked live. Capturas en
    `.harness/tasks/t056/shots/` (fuera del repo).

## Decisiones
Tomadas el 2026-11-13 con los valores por defecto propuestos (`design/board/closing.md`):
- P1: `Close agent` es un botón de peligro al pie de la cabecera grande de la pantalla del agente, decidido con las
  capturas del simulador. Rechazados un sexto icono en la barra (no cabe en 390 pt) y un `⋯` con menú.
- P2: la confirmación es una hoja que sube desde abajo (`Sheet`, su primer uso, y `CloseConfirm`), la misma para el
  tablero y la pantalla. Su texto sale del `--check --json` de cornercase: subagentes o agentes que para, si termina un
  flow, lo que sigue vivo, que no se borra nada del disco y el `warning`.
- P3: un topic nuevo, `design/board/closing.md`, enlazado desde Board, Chat y Look.
- P4: `Close` repite el check y, si el plan creció, enseña el nuevo en vez de cerrar, como `Plan::grew` en el Mac.
- P5: al cerrar el agente abierto (o su terminal, o un subagente sobre su padre) se vuelve al tablero.
- En iOS se desliza la fila a la izquierda (`RowSwipe`, con los umbrales de `backSwipe.ts`; una fila abierta a la vez);
  en Android, pulsación larga de 500 ms con un toque háptico (`LongPress`).
- Check y cierre van por la cola (`reach::asked`) con el `--boot` de la fila; una fila sin `boot` no ofrece cerrar.
- Nunca `--remove-worktree`, `--delete-branch`, `--force` ni `--unlock` (un test del argv lo fija). `close-agent` y
  `close-agent-tabs` entran en `contract::REQUIRED`; no dejan fuera a ningún servidor que hoy se acepte.

## design/ que actualiza
- `design/board/closing.md` (nuevo) y su entrada en `design/board.md`.
- `design/chat.md` (la acción de la pantalla), `design/look.md` (`Sheet` y el tono danger),
  `design/connection.md` (`REQUIRED`: 29 operaciones, 11 features), `design/upstream-gaps.md` (nada que reabrir tras
  `close --agent`, ni un cierre de solo lo confirmado) y `DESIGN.md`.
