# 025 — Pantalla 5: cambios del workspace y marcarlos como vistos

- Estado: pendiente
- Depende de: 015
- Riesgo: bajo
- Rama: `feat/025-changes` · PR: `feat: review a workspace's changes and mark them viewed`

## Objetivo
Ver el diff de git de un workspace fichero a fichero y marcar ficheros como vistos.

## Criterios de aceptación
- [ ] CA1 — `changes list --json` (resumen por fichero: estado, +/−, visto) y `changes file --json` (las líneas con su
  texto propio) tipados en el núcleo, con fixtures `tests/fixtures/control/changes/v1/*.json` de upstream.
- [ ] CA2 — Pantalla: lista de ficheros con su estado y contadores; tocar abre el diff (unificado, con colores
  añadido/quitado, números de línea, scroll horizontal en líneas largas).
- [ ] CA3 — Marcar/desmarcar visto (`changes viewed …`) por la cola, con respuesta optimista y vuelta atrás si falla.
- [ ] CA4 — Se refresca al volver a la pantalla y al recibir eventos del workspace (o con un `--follow` si el CLI lo
  da; anotar cuál).
- [ ] CA5 — vitest del render del diff (clases por tipo de línea).

## Referencias
- cornercase-gpui: `design/changes-panel.md`, `src/panels/changes/wire.rs`, `src/panels/changes/diff.rs`,
  `src/panels/changes/file_list.rs`, `src/panels/changes/follow.rs`.
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents/changes.md`, `design/changes-panel.md`,
  `tests/fixtures/control/changes/v1/`.

## design/ que actualiza
- Nuevo `design/changes-and-files.md` (y su línea en `DESIGN.md`).

## Verificación
En el simulador se ven los cambios de un worktree y se marca uno como visto, y el Mac lo refleja (criterio 5, parte cambios).
