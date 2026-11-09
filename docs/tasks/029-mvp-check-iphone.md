# 029 — Verificación del MVP en un iPhone por Tailscale

- Estado: pendiente
- Depende de: 015–028 y 030–032 (se hace al final, antes de la 033; decidido el 2026-11-09)
- Riesgo: bajo (perfil de Apple gratuito: la app caduca a los 7 días)
- Rama: `docs/029-mvp-check` · PR: `docs: record the mvp check on an iphone`

## Objetivo
Comprobar los criterios de éxito 1–6 de `docs/IDEA.md` en un iPhone real por Tailscale, con el puente en el Mac, y
dejar constancia; lo que falle se convierte en tareas nuevas.

## Criterios de aceptación
- [ ] CA1 — `docs/mvp-check.md`: por cada criterio, pasos seguidos, resultado (sí/no/parcial), versión de cornercase,
  de iOS y de la app, y fecha.
- [ ] CA2 — Cada fallo tiene su tarea nueva en `docs/tasks/` (numeración siguiente) enlazada desde el ROADMAP.
- [ ] CA3 — `CLAUDE.md` › Commands con cómo instalar en un iPhone con perfil gratuito (y la caducidad de 7 días).
- [ ] CA4 — Las notas "probado en vivo" de `design/*.md` actualizadas con lo visto.
- [ ] CA5 — Si nadie lo hizo antes, el CA4 de la 028: wifi → datos (con Tailscale) y datos → wifi, con el tablero
  y la terminal abiertos; tiempo desde la línea `network` hasta `connected`, anotado en
  `design/connection/network.md` (lo que falta, en su apartado Checked live).

## Referencias
- `docs/IDEA.md` › Criterios de éxito del MVP.

## design/ que actualiza
- Las áreas donde algo se comportó distinto de lo previsto.

## Verificación
Revisión del documento; ningún criterio queda sin resultado.
