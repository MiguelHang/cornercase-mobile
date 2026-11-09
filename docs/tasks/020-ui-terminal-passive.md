# 020 — Pantalla 3: terminal pasiva — `view` + `surface --patches`, zoom y scroll horizontal

- Estado: hecha, salvo la prueba en un iPhone (pellizco real, Claude Code trabajando, rendimiento)
- Depende de: 019, 014
- Riesgo: medio (rendimiento del render en la WebView)
- Rama: `feat/020-terminal-view` · PR: `feat: show a tab's live terminal without resizing the mac`

## Objetivo
Ver en vivo la terminal de la pestaña de un agente sin reclamarla: el PTY sigue con el ancho del Mac y el móvil lo
enseña con zoom y desplazamiento horizontal.

## Criterios de aceptación
- [x] CA1 — Al abrir: `view` con el tamaño del móvil (en celdas) **sin reclamar** la pestaña, y el stream
  `surface --view ID --patches`; al cerrar la pantalla se sueltan ambos (el `view` se cierra).
  **Desviación**: en upstream una `view` activa con tamaño reclama la pestaña al abrirse (redimensiona el Mac, contra
  CA5) y una inactiva o sin tamaño no pinta ni deja `surface --view`. Se hizo con `surface --tab T --cols C --rows R
  --patches` solo, sin `view` ni `view-set`, la única vía pasiva del contrato; al cerrar la pantalla el `surface`
  termina. Coste: el móvil no cuenta como "visto" (el ✓ y los avisos del agente siguen en el Mac); propuesta
  `view --no-claim` en `design/upstream-gaps.md`. La `view` llega con 022, al encender "ajustar".
- [x] CA2 — `NeedRedraw` de 019 → `redraw` por la cola; reinicio del servidor (`boot`) → se reabre todo.
- [x] CA3 — Render en `src/lib/terminal/`: filas como líneas de `span` con estilos (colores de la paleta Tokyo Night
  de gpui, negrita, cursiva, subrayado, inverso), cursor, fuente monoespaciada; solo se tocan las filas cambiadas.
- [x] CA4 — Pellizcar para zoom (mín. el ancho de la pantalla, máx. 2×) y desplazamiento horizontal; se recuerda el
  zoom por pestaña. Máx. 2× la fuente base (13 px), recordado en memoria por máquina, `boot` y pestaña.
- [x] CA5 — La ventana del Mac no cambia de tamaño mientras se mira (comprobado a mano). En el simulador iPhone Air,
  iOS 27.2, contra cornercase 0.13.19 por el puente, el 2026-11-09: primero entrando por Home y, tras el merge de 015,
  otra vez por tablero → agente → `Terminal` (el PTY siguió en 127×48 con la terminal abierta).
- [x] CA6 — vitest del mapeo de estilos a clases/estilos CSS y de la aplicación de filas cambiadas.

## Notas técnicas
- Si el DOM no aguanta (medir con un agente que imprime mucho), pasar a `<canvas>` en esta misma tarea o abrir una
  tarea nueva; anotarlo. Aguanta en el simulador (`seq` en bucle: la WebView ~1/5 de núcleo, el servidor más de uno):
  se queda el DOM (`design/terminal.md` › The DOM, not a canvas). Falta medirlo en un iPhone.

## Referencias
- cornercase-gpui: `design/terminal.md`, `src/terminal.rs`, `src/model/grid.rs`, `design/widgets.md` (paleta).
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents/views.md`, `…/surfaces.md`,
  `design/browser-access.md` (viewport en móviles).

## design/ que actualiza
- `design/terminal.md`: tamaño que pide el móvil, zoom, DOM o canvas y por qué.

## Verificación
En el simulador se ve la terminal de Claude Code en vivo al ancho del Mac, con zoom y scroll (criterio 3, parte "ver").

Se entra por el tablero: tocar la fila del agente abre su pantalla, y su botón `Terminal` abre la terminal encima
(Home, por donde se entraba antes, desapareció con 015). Comprobado el 2026-11-09 en el simulador iPhone Air, iOS 27.2,
contra cornercase 0.13.19 por el puente, con un `claude` de mentira que pasaba de working a done cada pocos segundos:
- la terminal siguió abierta y en vivo mientras cambiaban las filas, con un solo proceso `surface` todo el rato;
- `Back` volvió a la pantalla del agente y cortó el `surface`; un segundo `Back` volvió al tablero;
- el PTY no cambió de tamaño (127×48);
- con la terminal abierta, `kill-server` y `start-server`: reabrió `surface --tab` en el nuevo `boot`, sobre la misma
  pestaña restaurada con su id (ya como shell), y la pantalla del agente pasó a "This agent is no longer running."
  sin botón `Terminal`.

Falta probarlo con Claude Code de verdad trabajando y en un iPhone.
