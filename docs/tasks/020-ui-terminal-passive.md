# 020 — Pantalla 3: terminal pasiva — `view` + `surface --patches`, zoom y scroll horizontal

- Estado: pendiente
- Depende de: 019, 014
- Riesgo: medio (rendimiento del render en la WebView)
- Rama: `feat/020-terminal-view` · PR: `feat: show a tab's live terminal without resizing the mac`

## Objetivo
Ver en vivo la terminal de la pestaña de un agente sin reclamarla: el PTY sigue con el ancho del Mac y el móvil lo
enseña con zoom y desplazamiento horizontal.

## Criterios de aceptación
- [ ] CA1 — Al abrir: `view` con el tamaño del móvil (en celdas) **sin reclamar** la pestaña, y el stream
  `surface --view ID --patches`; al cerrar la pantalla se sueltan ambos (el `view` se cierra).
- [ ] CA2 — `NeedRedraw` de 019 → `redraw` por la cola; reinicio del servidor (`boot`) → se reabre todo.
- [ ] CA3 — Render en `src/lib/terminal/`: filas como líneas de `span` con estilos (colores de la paleta Tokyo Night
  de gpui, negrita, cursiva, subrayado, inverso), cursor, fuente monoespaciada; solo se tocan las filas cambiadas.
- [ ] CA4 — Pellizcar para zoom (mín. el ancho de la pantalla, máx. 2×) y desplazamiento horizontal; se recuerda el
  zoom por pestaña.
- [ ] CA5 — La ventana del Mac no cambia de tamaño mientras se mira (comprobado a mano).
- [ ] CA6 — vitest del mapeo de estilos a clases/estilos CSS y de la aplicación de filas cambiadas.

## Notas técnicas
- Si el DOM no aguanta (medir con un agente que imprime mucho), pasar a `<canvas>` en esta misma tarea o abrir una
  tarea nueva; anotarlo.

## Referencias
- cornercase-gpui: `design/terminal.md`, `src/terminal.rs`, `src/model/grid.rs`, `design/widgets.md` (paleta).
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents/views.md`, `…/surfaces.md`,
  `design/browser-access.md` (viewport en móviles).

## design/ que actualiza
- `design/terminal.md`: tamaño que pide el móvil, zoom, DOM o canvas y por qué.

## Verificación
En el simulador se ve la terminal de Claude Code en vivo al ancho del Mac, con zoom y scroll (criterio 3, parte "ver").
