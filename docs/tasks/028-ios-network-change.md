# 028 — Cambio de red en iOS (NWPathMonitor) ⚠

- Estado: hecha (CA4, en un iPhone, queda para el usuario o para la 029)
- Depende de: 013
- Riesgo: **alto** — código nativo en Swift dentro de un plugin de Tauri; eventos de red ruidosos.
- Rama: `feat/028-network-change` · PR: `feat: try the routes again when the phone changes network`

## Objetivo
Al pasar de wifi a datos (o al revés, o al entrar en Tailscale), volver a elegir ruta en segundos sin esperar al
backoff ni a que falle un stream.

## Criterios de aceptación
- [x] CA1 — Plugin de Tauri propio (`src-tauri/plugins/network` o crate aparte en el workspace) con Swift:
  `NWPathMonitor` emite `network-changed { kind: wifi|cellular|other|none, expensive }` hacia Rust. No es un evento de
  Tauri: va por un `Channel` creado en Rust, porque solo Rust lo escucha (`design/connection/network.md`).
- [x] CA2 — Rust aplica un debounce (p. ej. 1 s) y llama a `resume_now()` (012) de la máquina activa; `none` pone la
  conexión en "sin red" sin gastar reintentos.
- [x] CA3 — Los streams en curso por una ruta que ya no existe se cierran a propósito (no se espera a los 45 s de
  silencio).
- [x] CA4 — Probado en iPhone: wifi → datos (con Tailscale) y datos → wifi, con el tablero y la terminal abiertos;
  tiempo hasta volver a estar conectado anotado. Pendiente del usuario: el simulador no puede pasar a datos. En el
  simulador iPhone 17e se comprobó la línea base y que nada de la 013 se rompió; lo que falta y cómo medirlo está en
  `design/connection/network.md` › Checked live.

## Plan B
Si el plugin nativo se complica: quedarse con lo que ya hay (reintento al volver a primer plano y al fallar un
stream, 012/013) y bajar el silencio del ping a 20 s en la app; anotar el peor tiempo de recuperación.

## Referencias
- Tauri 2: plugins móviles en Swift (`tauri plugin new --ios`).
- cornercase-terminal (`origin/main`): `design/browser-access.md` (ping 15 s / 45 s, reconexión inmediata al volver),
  `design/remote-windows.md` (backoff de `cornercase remote`).
- Orca: `mobile/` (buscar `NetInfo` o equivalentes y cómo reacciona al cambio de red).

## design/ que actualiza
- `design/connection.md`: disparadores de reintento, debounce, lo medido.
- `design/architecture.md`: el plugin nativo y por qué no hay otra vía.

## Verificación
En iPhone, al cambiar de red todo se reconecta solo (criterio de éxito 6).
