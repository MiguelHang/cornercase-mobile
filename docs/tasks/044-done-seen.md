# 044 — Abrir un agente `done` lo da por visto

- Estado: hecha
- Depende de: 016
- Rama: `feat/044-done-seen` · PR: `feat: count an agent's screen on the phone as seeing it`

## Objetivo
Del iPhone del usuario (029, 2026-11-10): "cuando entras en un agente done, no se quita el estado". En cornercase, `done`
se va cuando su pestaña se ve: en la ventana o en "every active, sized, visible view" (cornercase-terminal
`design/agent-status.md`); el chat y la terminal pasiva del móvil no abren una vista así, así que el Mac no se entera.

## Criterios de aceptación
- [x] CA1 — Investigar qué comando da por vista una pestaña desde un cliente: una vista activa y visible (`view`) sin
  quedarse el tamaño, `view-set`, o algo de `status`; con lo que haya en upstream (0.13.29), abrir el chat de un
  agente `done` (y su terminal) hace que deje de estar `done` en el Mac y en el tablero.
- [x] CA2 — Si no se puede sin reclamar el tamaño, no se hace a escondidas: se anota en `design/upstream-gaps.md`
  (junto al borrador 03 de la épica de la 033, "view that sees without claiming") y el tablero lo explica.
- [x] CA3 — Comprobado con un agente real que pasa a `done` en otra pestaña; tests; `hx check full` verde.
