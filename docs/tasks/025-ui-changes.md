# 025 — Pantalla 5: cambios del workspace y marcarlos como vistos

- Estado: hecha
- Depende de: 015
- Riesgo: bajo
- Rama: `feat/025-changes` · PR: `feat: review a workspace's changes and mark them viewed`

## Objetivo
Ver el diff de git de un workspace fichero a fichero y marcar ficheros como vistos.

## Criterios de aceptación
- [x] CA1 — `changes list --json` (resumen por fichero: estado, +/−, visto) y `changes file --json` (las líneas con su
  texto propio) tipados en el núcleo, con fixtures `tests/fixtures/control/changes/v1/*.json` de upstream. Un solo
  tipo para `--json` y cada línea de `--follow`; lectura indulgente con `Other` en los enums; `styles`/`syntax` sin
  leer; el digest como texto; fixtures copiados byte a byte de 425b3ae1 (iguales en 0.13.19).
- [x] CA2 — Pantalla: lista de ficheros con su estado y contadores; tocar abre el diff (unificado, con colores
  añadido/quitado, números de línea, scroll horizontal en líneas largas). Se entra por `Changes` en la pantalla del
  agente (P2); tres modos (P1); el diff entero se desplaza en horizontal con los números `sticky`; los colores son
  tintes por tipo de línea y `emphasis` (P3), sin sintaxis; `binary`/`too large` muestran un aviso.
- [x] CA3 — Marcar/desmarcar visto (`changes viewed …`) por la cola, con respuesta optimista y vuelta atrás si falla.
  Marcar manda el digest de la versión que se ve y nunca `--mode`; desmarcar, `--clear --mode`. Un digest viejo responde
  OK sin marcar: no hay fallo que revertir y lo corrige el siguiente resumen (`design/upstream-gaps.md`).
- [x] CA4 — Se refresca al volver a la pantalla y al recibir eventos del workspace (o con un `--follow` si el CLI lo
  da; anotar cuál). **`changes list --follow`**: `events` no emite nada de git. Un stream por workspace y modo mientras
  la lista o el diff se ven, reabierto con el bucle que comparte la terminal; un diff abierto se relee cuando cambia su
  digest (`design/changes-and-files.md` › Following the list).
- [x] CA5 — vitest del render del diff (clases por tipo de línea). También `emphasis` en caracteres (emoji y acentos)
  y el visto optimista (aplicar, revertir, adoptar el resumen).

## Decisiones
Tomadas el 2026-11-09 con los valores por defecto propuestos (`design/changes-and-files.md`):
- P1: los tres modos (`Uncommitted` por defecto, `Commits`, `All`) en un selector; otro modo es otro stream, porque
  `changes-mode` no tiene comando. Un agente que ya hizo commit deja `Uncommitted` vacío.
- P2: botón `Changes` junto a `Terminal` en la pantalla del agente, sobre el workspace de su fila; nada en el tablero.
- P3: las palabras cambiadas (`emphasis`) con un tinte más fuerte, contadas en caracteres Unicode.
- El seguimiento sale del bucle de `Terminal::run` a `crates/core/src/follow.rs`, compartido, en vez de copiarlo.
- Sin sintaxis, gaps, stage, discard ni base; el diff grande va entero al DOM (se anota el límite).

## Referencias
- cornercase-gpui: `design/changes-panel.md`, `src/panels/changes/wire.rs`, `src/panels/changes/diff.rs`,
  `src/panels/changes/file_list.rs`, `src/panels/changes/follow.rs`.
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents/changes.md`, `design/changes-panel.md`,
  `tests/fixtures/control/changes/v1/`.

## design/ que actualiza
- Nuevo `design/changes-and-files.md` (y su línea en `DESIGN.md`).
- `design/architecture.md` (los comandos de `commands/changes.rs`, `commands/reach.rs`, `changes-changed`, la ranura de
  `Active`, `follow::reopen`, las pantallas), `design/terminal.md` (la reapertura, ahora compartida),
  `design/look.md` (rutas `changes` y `change`, iconos y tintes), `design/chat.md` (`Changes`),
  `design/connection/store.md` (el stream y la lectura sobre `Store::transport`) y `design/upstream-gaps.md`
  (`changes file`/`viewed` sin `--boot`; `viewed` con digest viejo responde OK).

## Verificación
En el simulador se ven los cambios de un worktree y se marca uno como visto, y el Mac lo refleja (criterio 5, parte cambios).
