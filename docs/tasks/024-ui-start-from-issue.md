# 024 — Pantalla 4: arrancar desde una issue (`issue list/read`, `start --issue`)

- Estado: pendiente
- Depende de: 023
- Riesgo: bajo
- Rama: `feat/024-start-from-issue` · PR: `feat: start an agent on an issue from the phone`

## Objetivo
Elegir una issue de los trackers que cornercase tiene conectados, leerla y arrancar un agente sobre ella en su worktree.

## Criterios de aceptación
- [ ] CA1 — `issue sources --json` → trackers conectados por proyecto; si ninguno, se dice cómo conectarlo desde el Mac
  (los tokens los guarda cornercase, nunca la app).
- [ ] CA2 — Lista `issue list --json` con búsqueda y filtros básicos (abiertas/mías), paginada si el CLI lo permite.
- [ ] CA3 — Detalle `issue read --json` con el cuerpo en markdown (reutiliza el render de 016).
- [ ] CA4 — "Empezar" abre el formulario de 023 prellenado y manda `start --issue … --worktree --placed`.
- [ ] CA5 — Tests del núcleo con fixtures de `issue list`/`issue read` (de upstream o generados con su versión).

## Referencias
- cornercase-gpui: `design/issues.md`, `src/panels/issues/list.rs`, `src/panels/issues/detail.rs`,
  `src/panels/issues/start.rs`, `src/panels/issues/load.rs`.
- cornercase-terminal (`origin/main`): `design/issues.md`, `src/cli/issue.rs`.

## design/ que actualiza
- `design/starting-work.md`: el flujo desde una issue.

## Verificación
En el iPhone se arranca un agente en un worktree desde una issue de GitHub (criterio de éxito 4).
