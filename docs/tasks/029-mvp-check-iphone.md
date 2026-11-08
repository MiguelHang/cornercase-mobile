# 029 — Verificación del MVP en un iPhone por Tailscale

- Estado: pendiente
- Depende de: 015–028
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

## Referencias
- `docs/IDEA.md` › Criterios de éxito del MVP.

## design/ que actualiza
- Las áreas donde algo se comportó distinto de lo previsto.

## Verificación
Revisión del documento; ningún criterio queda sin resultado.
