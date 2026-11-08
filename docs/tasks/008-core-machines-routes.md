# 008 — Núcleo: máquinas, rutas y elección de ruta

- Estado: pendiente
- Depende de: 007
- Riesgo: bajo
- Rama: `feat/008-core-machines` · PR: `feat: save machines with several routes and pick the one that answers`

## Objetivo
Guardar máquinas con varias rutas cada una y conectarse por la primera que responde, volviendo a probar cuando se
le pida (cambio de red, vuelta a primer plano, fallo de un stream).

## Criterios de aceptación
- [ ] CA1 — `crates/core/src/machines.rs`: `Machine { id, name, routes: Vec<Route> }`, `Route::Bridge { url }` (enum
  abierto a `Ssh`, `Iroh`, `Cloudflare` más tarde, sin variantes muertas hoy).
- [ ] CA2 — `Route::parse(url)` acepta `ws://<host>:<puerto>/?token=…` (lo que imprime y codifica en QR el puente) y
  rechaza con mensaje claro una URL sin token, otro esquema o un host vacío.
- [ ] CA3 — Persistencia detrás de un trait `MachineStore` con implementación de fichero JSON (escritura atómica:
  temporal + rename); el token de la URL es un secreto: va al almacén de secretos (trait `Secrets`, implementado en
  `src-tauri` con el Keychain en 009/013), no al JSON.
- [ ] CA4 — `pick_route(machine, transport_factory)`: prueba las rutas a la vez con `contract --json` y plazo, gana la
  primera que responde con éxito; guarda cuál ganó para probarla primero la próxima vez; devuelve el error de cada
  ruta si ninguna responde.
- [ ] CA5 — Tests con `FakeTransport`: gana la más rápida, una que falla no bloquea, todas fallan, la última ganadora
  se prueba primero, persistencia ida y vuelta sin token en el JSON.

## Notas técnicas
- La identidad de la máquina para el `Store` no es la ruta: dos rutas de la misma máquina dan el mismo `boot`.
- "Volver a probar" es una función que llama 012; los disparadores (red, primer plano) están en 013 y 028.

## Referencias
- cornercase-gpui: `design/machines.md`, `src/model/machines.rs`, `src/panels/machines/data.rs` (cómo modela
  máquinas y su estado), `design/remote.md`.
- t3code: `docs/user/remote-access.md` (varias rutas por máquina).

## design/ que actualiza
- `design/connection.md`: formato de ruta, dónde vive el token, cómo se elige la ruta y en qué orden.

## Verificación
`cargo test -p cornercase-core` verde.
