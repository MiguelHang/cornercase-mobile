# 055 — Pantalla de carga al arrancar, sin parpadeo

- Estado: hecha
- Depende de: —
- Rama: `feat/055-launch-screen` · PR: `feat: show a launch screen until the app is ready`

## Objetivo
Del iPhone del usuario (2026-11-12): "estaría guay una pequeña pantalla de carga al arrancar la app, así no mete ese
parpadeo".

## Criterios de aceptación
- [x] CA1 — Grabar el arranque en frío en el simulador (`simctl io recordVideo`) y en el iPhone si está, y describir el
  parpadeo: qué se ve y cuándo (launch storyboard, WebView en blanco/negro, primer render, pantalla vacía hasta leer las
  máquinas o conectar…). La causa queda escrita en `design/look.md`.
- [x] CA2 — Desde que se toca el icono hasta la primera pantalla útil se ve una sola pantalla de carga continua: el
  fondo de la app, con el icono (o el logotipo) centrado desde la primera pintura de la WebView hasta que se va; nada
  de destellos de otro color ni saltos de posición. Ajustado el 2026-11-13: en iOS el launch screen nativo es solo el
  fondo, sin la marca, porque la WebView tarda 0,3–0,9 s en pintar tras él y la marca se apagaba y volvía a salir; una
  marca que aparece una vez es mejor que una que parpadea (`design/look.md` › Launch).
- [x] CA3 — La pantalla de carga se va con un fundido corto cuando la app sabe qué enseñar (lista de máquinas leída y,
  si hay máquina activa, el primer estado de conexión), no por tiempo fijo; con Reduce Motion, sin fundido. Si tarda
  más de unos segundos, enseña la pantalla normal con su estado (no se queda colgada).
- [x] CA4 — Android igual (splash del sistema de Android 12+ con el mismo fondo e icono) o anotado como pendiente.
- [x] CA5 — Grabación antes/después en el simulador; tests de la lógica de "listo"; `hx check full` verde.
