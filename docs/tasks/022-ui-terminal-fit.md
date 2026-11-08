# 022 — Pantalla 3: "ajustar a este móvil" (`view-set --claim`)

- Estado: pendiente
- Depende de: 021
- Riesgo: bajo
- Rama: `feat/022-terminal-fit` · PR: `feat: let the phone claim a tab's size while it is open`

## Objetivo
Un botón que reclama la pestaña para que el PTY tome el ancho del móvil mientras esa pantalla está abierta, y la
suelta al salir.

## Criterios de aceptación
- [ ] CA1 — Botón "ajustar a este móvil" (conmutador) → `view-set --view ID --claim` por la cola; un rechazo
  (no elegible) se muestra con su motivo.
- [ ] CA2 — Al salir de la pantalla, al ir a segundo plano o al apagar el conmutador → `view-set --view ID --inactive`.
  Al volver a primer plano con la pantalla abierta y el conmutador encendido, se vuelve a reclamar.
- [ ] CA3 — Los frames se filtran por `projection` (019): no se pinta un frame viejo de antes del cambio de tamaño.
- [ ] CA4 — Si el Mac retoma la pestaña (se usa su ventana), el conmutador se apaga solo y se dice.
- [ ] CA5 — Tests del núcleo: la secuencia de comandos al abrir/cerrar/segundo plano con el conmutador encendido.

## Referencias
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents/views.md` (claims, eligible,
  `--inactive`, la ventana reclama al usarse).
- `design/terminal.md` de este repo (Passive by default, Fit to this phone).

## design/ que actualiza
- `design/terminal.md`: cuándo se suelta, qué pasa al ir a segundo plano, cómo se detecta que el Mac la retomó.

## Verificación
En iPhone: con el conmutador se lee al ancho del móvil; al salir, la ventana del Mac vuelve a su tamaño
(criterio de éxito 3, parte "ajustar").
