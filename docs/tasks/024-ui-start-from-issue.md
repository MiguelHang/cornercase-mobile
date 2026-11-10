# 024 — Pantalla 4: arrancar desde una issue (`issue list/read`, `start --issue`)

- Estado: hecha
- Depende de: 023
- Riesgo: bajo
- Rama: `feat/024-start-from-issue` · PR: `feat: start an agent on an issue from the phone`

## Objetivo
Elegir una issue de los trackers que cornercase tiene conectados, leerla y arrancar un agente sobre ella en su worktree.

## Criterios de aceptación
- [x] CA1 — `issue sources --json` → trackers conectados por proyecto; si ninguno, se dice cómo conectarlo desde el Mac
  (los tokens los guarda cornercase, nunca la app). Si ninguno está `connected`: "No issue tracker is connected" y, por
  tracker, qué le falta (`missing`) o por qué falla (`reason`), y dónde se conecta en el Mac (GitHub con `gh` en la
  carpeta del proyecto; el resto en Ajustes › Issues de cornercase), con `Check again`; sin campo de token.
- [x] CA2 — Lista `issue list --json` con búsqueda y filtros básicos (abiertas/mías), paginada si el CLI lo permite.
  El CLI no busca ni pagina (`design/upstream-gaps.md`): la búsqueda es local (clave, título y etiquetas, sin nueva
  llamada); `Closed too` → `--closed`, `Mine` → `--assignee me` y, con más de un tracker, `--source=`. Primero
  `--cached` y luego en vivo (P3); el exit 3 muestra lo listado y una línea por tracker que falló; si un tracker da 100,
  se pide acotar los filtros, sin cifra.
- [x] CA3 — Detalle `issue read --json` con el cuerpo en markdown (reutiliza el render de 016). Con `--source=` de la
  fila y la clave tras `--`; los comentarios debajo con el mismo `Markdown` (P4); `Reading the issue…` y, si falla, la
  razón con `Try again`.
- [x] CA4 — "Empezar" abre el formulario de 023 prellenado y manda `start --issue … --worktree --placed`. El
  formulario lleva la issue arriba, el proyecto de la lista (fijo), el agente (regla de 023), el modelo vacío y la rama
  de la issue editable, sin prompt (cornercase lo escribe y `start` rechaza prompt e issue juntos); el prompt que usará
  se ve plegado (P5). `--worktree=` solo si la rama cambia, porque no es un interruptor sino el nombre de la rama. Se
  entra por un tercer botón `Issue` en Start (P1); tras arrancar, `Back` va al tablero (P2).
- [x] CA5 — Tests del núcleo con fixtures de `issue list`/`issue read` (de upstream o generados con su versión).
  Fixtures con la forma de 0.13.19 y datos inventados (`issue-sources`, `issue-list` con un tracker en error,
  `issue-read`); tests del argv de lectura y de `start --issue` (con y sin modelo, con la rama de la issue o con otra,
  valores que empiezan por `-`), y vitest del estado de la lista, del detalle y del formulario en modo issue.

## Decisiones
Tomadas el 2026-11-10 con los valores por defecto propuestos (`design/starting-work.md` › From an issue):
- P1: se entra por un tercer botón `Issue` en la pantalla Start, que reutiliza su selector de proyecto; no un botón
  `Issues` en el tablero.
- P2: tras arrancar, la pila queda `[base, agent]` (`Router.settle`) y `Back` va al tablero, no a la issue.
- P3: cada lista se pide `--cached` y luego en vivo, como gpui; una caché vacía no borra lo que se ve.
- P4: los comentarios van bajo el cuerpo, con el mismo `Markdown`.
- P5: el prompt que usará cornercase se ve plegado y de solo lectura en el formulario.

## Referencias
- cornercase-gpui: `design/issues.md`, `src/panels/issues/list.rs`, `src/panels/issues/detail.rs`,
  `src/panels/issues/start.rs`, `src/panels/issues/load.rs`.
- cornercase-terminal (`origin/main`): `design/issues.md`, `src/cli/issue.rs`.

## design/ que actualiza
- `design/starting-work.md`: el flujo desde una issue (From an issue), P1–P5 y lo rechazado, `settle` tras arrancar.
- `DESIGN.md` (los ficheros nuevos en la línea de Starting work), `design/architecture.md` (los comandos de
  `commands/issues.rs` y `start_issue`, sus tipos, las pantallas `issues` e `issue`), `design/look/navigation.md` (las
  rutas, `settle` y `Router.latest`), `design/look.md` (`AgentPicker`, las pantallas de issues),
  `design/changes-and-files.md` (`partial_json` en `transport`) y `design/upstream-gaps.md` (sin búsqueda ni páginas en
  `issue list`, sin prompt propio con `--issue`, sin `--boot` en `issue`).

## Verificación
En el iPhone se arranca un agente en un worktree desde una issue de GitHub (criterio de éxito 4).
Comprobado el 2026-11-10 en el simulador iPhone 17 contra el puente y cornercase 0.13.19: la lista con GitHub y
Shortcut, el filtro por tracker, la búsqueda local, la issue en Markdown y un agente arrancado en el worktree de su
rama, que aparece en el tablero (`design/starting-work.md` › Checked live).
