# 050 — Partir por temas los ficheros de `design/` de más de 40 KB

- Estado: pendiente
- Depende de: 043
- Rama: `docs/050-split-design-by-topic` · PR: `docs: split the largest design files by topic`

## Objetivo
`DESIGN.md` › What goes in an area file pide partir por temas un fichero de más de unos 40 KB. Tras la 043,
`design/terminal.md` pasa de 55 KB y `design/chat.md` de 64 KB (la review de la 043, R5, lo aplazó aquí para no
mezclar el movimiento de texto con su diff).

## Criterios de aceptación
- [ ] CA1 — `design/terminal.md` se parte en `design/terminal/` (p. ej. `grid.md`, `fit.md`, `typing.md`), y
  `design/chat.md` en `design/chat/`, como `design/look/`: el fichero del área se queda con lo que vale para toda el
  área y la lista de temas, y `DESIGN.md` sigue con una línea por área.
- [ ] CA2 — Las citas "Terminal › …" y "Chat › …" del resto de `design/`, `docs/` y `CLAUDE.md` siguen llevando a su
  sitio; ningún texto se pierde ni se duplica (diff sin cambios de redacción).
