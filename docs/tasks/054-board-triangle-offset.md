# 054 — Tablero: el triangulito, entre el punto y el texto

- Estado: pendiente
- Depende de: 051
- Rama: `feat/054-board-triangle-offset` · PR: `feat: move the board's fold triangle between the dot and the title`

## Objetivo
Del iPhone del usuario (2026-11-12), sobre la 051: "la flechita de los subagentes me gustaría que estuviera más en el
centro entre el status y el texto, en la línea que está ahora pero desplazada un poco a la derecha".

## Criterios de aceptación
- [ ] CA1 — El triángulo sigue en la segunda línea de la fila (la del lugar, donde está hoy), pero su centro horizontal
  queda a medio camino entre el centro del punto de estado y el inicio del título; el punto no se mueve y el título
  tampoco.
- [ ] CA2 — El área de toque del pliegue (44 pt) sigue cubriendo punto y triángulo y no llega al título; nada cambia en
  filas sin hijos ni durante la búsqueda.
- [ ] CA3 — Comprobado en el simulador con capturas (plegado, desplegado, niveles 1–3); `design/board.md` dice dónde va
  y por qué; `hx check full` verde.
