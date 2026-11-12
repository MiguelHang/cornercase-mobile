# 053 — Partir por temas `design/starting-work.md`

- Estado: pendiente
- Depende de: 052
- Rama: `docs/053-split-starting-work` · PR: `docs: split design/starting-work.md by topic`

## Objetivo
De la review de la 052: `design/starting-work.md` pesa ~48 KB y `DESIGN.md` pide partir por temas un fichero de más de
unos 40 KB, como hizo la 050 con `terminal.md` y `chat.md`.

## Criterios de aceptación
- [ ] CA1 — `design/starting-work.md` queda como índice del área y sus temas pasan a `design/starting-work/` (por
  ejemplo: el agente, desde una issue, con un flow, la cuenta, comprobado en vivo), cada uno < 40 KB.
- [ ] CA2 — No se pierde ni se duplica texto (comprobación por multiconjunto de líneas, como en la 050); solo cambian
  cabeceras y citas.
- [ ] CA3 — Toda cita `Starting work › …` del resto de `design/`, `DESIGN.md` y `CLAUDE.md` apunta a su tema nuevo.
- [ ] CA4 — `hx check full` verde.
