# 051 — Tablero: las filas aprovechan todo el ancho

- Estado: pendiente
- Depende de: 040
- Rama: `feat/051-board-full-width` · PR: `feat: let the board's rows use the phone's full width`

## Objetivo
Del iPhone del usuario (2026-11-12): "la lista de agentes deja mucho espacio a la izquierda, ocupemos todo".

Hoy una fila raíz sin hijos tiene ~88 px antes del punto de estado: el margen del tablero (`--inset-wide`, 16), la
columna fija de 60 px que reserva toda fila para el disclosure y el `+n` (040, `design/board.md` › What a row shows: «The
cost is 60 px of title on a row without children») y el padding de la fila (`--inset`, 12). Se eligió alinear los puntos
de todas las filas por encima del ancho; en el teléfono real pesa más el ancho.

## Criterios de aceptación
- [ ] CA1 — Una fila sin hijos empieza su punto a la distancia del borde que usan las listas de iOS (unos 16–20 pt), no
  a ~88. Nada de columna vacía de 60 px.
- [ ] CA2 — El disclosure sigue a la izquierda y no parece un "ir" (lo decidido en 040), pero estrecho: solo el chevron
  (área de toque de 44 pt que puede solaparse con el padding, no ancho reservado). La atención y el `+n` salen de esa
  columna (por ejemplo, junto a la hora a la derecha en gris o bajo el título), sin tocar el punto.
- [ ] CA3 — Si ninguna fila visible tiene hijos, ninguna reserva sitio para el disclosure; si alguna lo tiene, los puntos
  siguen en una columna (el hueco que quede es el del chevron, no 60 px). La sangría por nivel se mantiene.
- [ ] CA4 — El buscador y las cabeceras de sección se alinean con las filas; los márgenes laterales del tablero, como los
  del resto de pantallas (Look).
- [ ] CA5 — Comprobado en el simulador (raíces sin hijos, con hijos plegados y desplegados, `+12` con atención, título
  largo) con capturas; `design/board.md` actualizado (por qué se dejó la columna de 60); `hx check full` verde.
