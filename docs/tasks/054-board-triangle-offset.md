# 054 — Tablero: el triangulito, entre el punto y el texto

- Estado: hecha (en el iPhone del usuario, pendiente)
- Depende de: 051
- Rama: `feat/054-board-triangle-offset` · PR: `feat: move the board's fold triangle between the dot and the title`

## Objetivo
Del iPhone del usuario (2026-11-12), sobre la 051: "la flechita de los subagentes me gustaría que estuviera más en el
centro entre el status y el texto, en la línea que está ahora pero desplazada un poco a la derecha".

## Criterios de aceptación
- [x] CA1 — El triángulo sigue en la segunda línea de la fila (la del lugar, donde está hoy), pero su centro horizontal
  queda a medio camino entre el centro del punto de estado y el inicio del título; el punto no se mueve y el título
  tampoco.
- [x] CA2 — El área de toque del pliegue (44 pt) sigue cubriendo punto y triángulo y no llega al título; nada cambia en
  filas sin hijos ni durante la búsqueda.
- [x] CA3 — Comprobado en el simulador con capturas (plegado, desplegado, niveles 1–3); `design/board.md` dice dónde va
  y por qué; `hx check full` verde.
  - En el iPhone 17 del simulador (iOS 27.2, cornercase 0.13.31): el centro del triángulo queda a 34.5 pt del borde en
    una raíz (plegada y desplegada) y a 50.5 y 66.5 en los niveles 2 y 3; los puntos siguen en 23 + 16 por nivel, los
    títulos en 46 + 16 y el área de toque en 1–45 + 16 (en x=44 pliega, en x=47 abre el chat; tocar el triángulo movido
    pliega). En la búsqueda no hay triángulo y los puntos no se mueven. Girando el SVG, WebKit daba al botón del pliegue
    desplegado un marco de accesibilidad de 55 pt (1–56), encima del título; se gira el `<path>` y el marco se queda en
    44 pt en todos los niveles, plegado y desplegado (VoiceOver no se miró). `design/board.md` › Tree and folding (dónde
    va y por qué), › A row y › Checked live. Capturas en `.harness/tasks/t054/shots/` y, de la ronda de review,
    `shots/r1/` (fuera del repo).
