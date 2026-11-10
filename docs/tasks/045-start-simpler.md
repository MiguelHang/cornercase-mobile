# 045 — Arrancar: sin workspaces nuevos y las issues sin un filtro de proyecto que no filtra

- Estado: en curso
- Depende de: 023, 024
- Rama: `feat/045-start-simpler` · PR: `feat: start agents in existing workspaces and list every tracker's issues`

## Objetivo
Del iPhone del usuario (029, 2026-11-10): "igual crear worktrees no tiene sentido, ya que luego no se pueden usar" y "en
las issues lo de filtrar por proyecto igual no tiene sentido porque no se está filtrando; o mostramos todas o ninguna".
El usuario aclaró después (2026-11-10) que se equivocó al escribir "worktrees": lo que no sirve desde el móvil es
**crear workspaces** (un workspace sin agente no tiene fila, así que el móvil no lo puede abrir). Las worktrees se
quedan: un agente en su propia worktree se usa entero desde el móvil, y una issue sigue arrancando en la worktree de su
rama.

## Criterios de aceptación
- [x] CA1 — Start deja de ofrecer el formulario de nuevo workspace (`new-workspace`): un agente se arranca en un
  workspace que ya existe (el del agente desde el que se abre, con su botón `New agent`, o el que se elija), o en su
  propia worktree con `Own worktree` y `Branch`. Ningún código ni binding usa `new-workspace`, y `contract::REQUIRED` ya
  no lo exige; el allowlist del puente lo conserva porque refleja upstream.
- [x] CA2 — Issues: el selector de proyecto no aparenta filtrar. `issue list --project` solo filtra GitHub y GitLab (la
  issue es del repositorio de la carpeta del proyecto); Shortcut, Linear, Jira, Plane y Asana son de la cuenta y dan la
  misma lista en cualquier proyecto. Decisión: se listan las issues de todos los trackers conectados (los de la cuenta
  una vez, GitHub y GitLab por proyecto), con el tracker y, si es de un repositorio, el proyecto en cada fila; dónde
  arrancar se elige al arrancar (`Where`, con los sitios de `issue sources`). Rechazado: un filtro real solo para
  GitHub y GitLab, y dejar el selector con una nota. Decisión en `design/starting-work.md`; el hueco de upstream (no hay
  `issue list` de todos los proyectos) en `design/upstream-gaps.md`.
- [ ] CA3 — Comprobado en el simulador; tests; `hx check full` verde.
