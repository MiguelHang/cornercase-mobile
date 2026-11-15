# 059 — Quitar el bridge

- Estado: en curso
- Depende de: 058
- Rama: `chore/059-remove-bridge` · PR: `chore: remove the development bridge`

## Objetivo
Borrar `bridge/` y lo que solo existía por él, una vez que la app vive del `/cli` de cornercase.

## Criterios de aceptación
- [x] CA1 — Fuera `bridge/` del workspace, de CI y de `CLAUDE.md` › Commands; los tests de integración que lo arrancaban
  usan un cornercase con `/cli` (o un servidor falso en `crates/core/src/transport/fake`).
- [x] CA2 — Lo de `crates/core` que solo usaba el bridge (su lado del protocolo, la lista permitida, `--bridge-routes`)
  se borra; lo que comparte con `/cli` se queda.
- [x] CA3 — `docs/` y `design/` sin instrucciones de arrancar el bridge; `design/bridge.md` resumido como historia.
- [ ] CA4 — `hx check full` en verde.
