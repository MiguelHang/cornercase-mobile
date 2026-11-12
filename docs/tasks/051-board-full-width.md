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
- [ ] CA2 — Decisión del usuario (2026-11-12): el indicador de que la fila tiene hijos y se despliega va **debajo del
  punto de estado**, un triangulito del mismo tamaño que el punto (apunta a la derecha plegado y hacia abajo desplegado,
  girando como el chevron de 040). Sin columna propia: el punto y el triángulo comparten la columna del punto, así que
  una fila con hijos y otra sin ellos alinean sus puntos sin reservar ancho. Tocar el punto o el triángulo pliega o
  despliega (área de toque de 44 pt sobre esa columna, que no abre el chat); el resto de la fila abre el chat. La
  atención y el `+n` salen de la columna de 60 px (por ejemplo, junto a la hora a la derecha en gris), sin tocar el punto.
- [ ] CA3 — Ninguna fila reserva ancho para el disclosure; los puntos de todas las filas de un nivel siguen en una
  columna. La sangría por nivel se mantiene.
- [ ] CA4 — El buscador y las cabeceras de sección se alinean con las filas; los márgenes laterales del tablero, como los
  del resto de pantallas (Look).
- [ ] CA5 — Comprobado en el simulador (raíces sin hijos, con hijos plegados y desplegados, `+12` con atención, título
  largo) con capturas; `design/board.md` actualizado (por qué se dejó la columna de 60); `hx check full` verde.
