# 046 — Elegir la LAN cuando la LAN y Tailscale contestan

- Estado: pendiente
- Depende de: 035
- Rama: `feat/046-prefer-lan` · PR: `feat: prefer the lan route when both answer`

## Objetivo
En el iPhone del usuario (029, 2026-11-10), en la wifi de casa, la barra decía `via [fd7a:115c:a1e0::…]`: la ruta de
Tailscale ganó a la de la LAN. La carrera de `pick_route` se queda con la que contesta antes y pone primero la
ganadora anterior, así que una vez gana Tailscale sigue ganando.

## Criterios de aceptación
- [ ] CA1 — Con varias rutas que contestan, gana la LAN (luego Tailscale, luego el resto) si contesta dentro de una
  ventana corta (p. ej. 150 ms) tras la primera; si no, la primera. La "ganadora anterior primero" no impide volver a
  la LAN al cambiar de red (028). Anotarlo en `design/connection.md` › The race.
- [ ] CA2 — Tests de la carrera con relojes falsos (solo LAN, solo Tailscale, las dos, LAN lenta); `hx check full` verde.
