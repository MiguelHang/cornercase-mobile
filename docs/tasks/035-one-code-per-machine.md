# 035 — Una máquina, un código: el puente da todas sus rutas y la app elige

- Estado: hecha (escanear el QR con la cámara y quitar la wifi en el iPhone queda para el usuario)
- Depende de: 027, 028
- Riesgo: medio — cambia lo que imprime el puente y lo que acepta la app; hay máquinas ya guardadas con una sola ruta.
- Rama: `feat/035-one-code-per-machine` · PR: `feat: add a machine with one code that carries every route of its bridge`

## Objetivo
Para el usuario, añadir un Mac es añadir **una máquina**: un solo QR (o una sola URL) y listo. Por qué red se llega
(LAN, Tailscale…) y cuál es la más rápida es cosa de la app, no suya.

Hoy el puente imprime un bloque y un QR por dirección (`LAN (en0)`, `Tailscale (utun…)`), y la app guarda solo la ruta
del código que se escanea. Con el de LAN, al quitar la wifi no hay por dónde llegar aunque Tailscale esté encendido;
hay que añadir a mano la segunda ruta (Machines › Edit › Add route). El usuario lo probó el 2026-11-09 y lo pidió así:
"para el usuario añadir una máquina tiene que darle igual, es una máquina, y usar la red más rápida es cosa nuestra".

## Criterios de aceptación
- [x] CA1 — El puente imprime **un solo código de máquina** (QR y su texto para pegar) con todas las direcciones en las
  que escucha y el token, y el nombre del Mac como sugerencia de nombre. El formato (p. ej. un enlace con varias
  direcciones) se decide en el análisis y se documenta en `design/bridge.md`; las URLs por dirección pueden seguir
  saliendo como texto para depurar, pero el QR que se enseña es uno.
- [x] CA2 — La app acepta ese código al añadir una máquina (escáner y pegar): guarda la máquina con **todas** sus rutas
  de una vez, en el orden que convenga, sin preguntar por ninguna.
- [x] CA3 — Si el código es de un Mac ya guardado (mismo token o mismas direcciones), la app no crea otra máquina:
  le añade las rutas que falten y actualiza el token si cambió, y lo dice en una línea.
- [x] CA4 — Compatibilidad: las URLs de una sola ruta que ya imprime el puente (y las máquinas ya guardadas) siguen
  funcionando.
- [x] CA5 — Rutas al día sin volver a escanear: estando conectada, la app conoce las direcciones actuales del puente
  (p. ej. el puente las anuncia o responde a una petición propia del puente, no un comando de cornercase) y añade a la
  máquina las que falten (p. ej. Tailscale encendido después). Las que ya no existan no se borran solas; como mucho
  se marcan. Si esto se complica, queda como plan B (abajo) y se documenta.
- [x] CA6 — Elegir la ruta es de la app: con varias rutas que contestan se usa la más rápida (hoy la carrera de
  `pick_route`, con la ganadora anterior primero); al cambiar de red (028) se vuelve a elegir sola. Anotar en
  `design/connection.md` qué pasa cuando LAN y Tailscale contestan a la vez y por qué gana la que gana.
- [x] CA7 — La UI habla de máquinas: añadir no menciona rutas; en la pantalla de una máquina las rutas se ven como
  información (por dónde se llega ahora) y las acciones de rutas a mano quedan como opción avanzada, no como paso
  necesario.
- [x] CA8 — Comprobado en el simulador (pegar el código; el escáner no tiene cámara) y, pendiente para el usuario en
  su iPhone: escanear el QR único, quitar la wifi y ver que sigue por Tailscale sin tocar nada.
- [x] CA9 — `hx check full` y mutation en verde; tests del formato (puente y app) con casos de una, dos y ninguna
  dirección, IPv6 y token con caracteres raros.

## Plan B
Si CA5 (rutas al día) no cabe: dejarlo fuera, con el código único (CA1–CA4) ya resuelto, y anotar en
`design/connection.md` que tras cambiar las direcciones del Mac hay que volver a escanear (y que la app lo detecta
como el mismo Mac, CA3).

## Referencias
- `design/bridge.md` › Listening on the network, The URL and its QR code.
- `design/connection.md` › Machines with several routes, The race (`pick_route`).
- `design/connection/network.md` (028).
- Tarea 027 (Machines, rutas, "the same machine").

## design/ que actualiza
- `design/bridge.md`: el código de máquina y su formato.
- `design/connection.md`: añadir con varias rutas, el mismo Mac, rutas al día, cuál gana.
- `design/look.md`: Machines sin rutas en primer plano.

## Verificación
Escanear un único QR de un Mac con LAN y Tailscale; con la wifi apagada la app sigue conectada por Tailscale sin que el
usuario haya tocado nada más.
