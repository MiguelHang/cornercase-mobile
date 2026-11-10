# 045 — Arrancar: sin worktrees nuevos y las issues sin un filtro de proyecto que no filtra

- Estado: pendiente
- Depende de: 023, 024
- Rama: `feat/045-start-simpler` · PR: `feat: drop new worktrees and the issues' project filter from the phone`

## Objetivo
Del iPhone del usuario (029, 2026-11-10): "igual crear worktrees no tiene sentido, ya que luego no se pueden usar" y "en
las issues lo de filtrar por proyecto igual no tiene sentido porque no se está filtrando; o mostramos todas o ninguna".

## Criterios de aceptación
- [ ] CA1 — Start deja de ofrecer `Own worktree` y el formulario de nuevo workspace: un agente se arranca en un
  workspace que ya existe (el del agente desde el que se abre, o el que se elija). Si el análisis encuentra un caso en
  que sí sirve desde el móvil, se pregunta al usuario antes de quitarlo.
- [ ] CA2 — Issues: el selector de proyecto no aparenta filtrar. Se averigua qué hace `issue list` con el proyecto
  (trackers por proyecto o por cuenta) y se elige: o se listan las de todos los trackers conectados, con el proyecto de
  cada issue a la vista y el proyecto donde arrancar elegido al arrancar, o el filtro filtra de verdad. Decisión en
  `design/starting-work.md`.
- [ ] CA3 — Comprobado en el simulador; tests; `hx check full` verde.
