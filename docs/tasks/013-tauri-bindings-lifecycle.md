# 013 — App: comandos y eventos de Tauri con tipos (tauri-specta) y ciclo de vida

- Estado: pendiente
- Depende de: 009, 012
- Riesgo: medio (tauri-specta para Tauri 2 en versión RC)
- Rama: `feat/013-tauri-bindings` · PR: `feat: expose the store to the webview with typed commands and events`

## Objetivo
Unir el núcleo a la WebView: comandos y eventos de Tauri con tipos TypeScript generados, el `Store` vivo en la app y
la reconexión al volver a primer plano.

## Criterios de aceptación
- [ ] CA1 — `src-tauri/src/commands/`: comandos tipados mínimos para lo que hay: `machines_list`, `machine_add(url,
  name)`, `machine_remove`, `connect(machine)`, `connection_state`, `identity_fingerprint`. Errores serializados con
  un tipo propio (código + mensaje), nunca `String` suelto.
- [ ] CA2 — Eventos tipados: `connection-changed`, `session-changed` (el estado del `Store` o un diff), `toast`.
- [ ] CA3 — Bindings generados con tauri-specta a `src/lib/bindings.ts` (fichero generado, excluido de
  `check-rules`); un test de Rust en CI regenera y falla si el fichero cambia (bindings al día).
- [ ] CA4 — Ciclo de vida: al volver a primer plano (`RunEvent::Resumed` en móvil) se llama a `resume_now()`; al ir a
  segundo plano no se hace nada activo (iOS suspende), y al volver no queda ningún stream zombi.
- [ ] CA5 — `Secrets` de iOS (009) y `MachineStore` de fichero en el directorio de datos de la app conectados al núcleo.
- [ ] CA6 — Una pantalla Svelte de prueba (se reemplaza en 014) que lista el estado de conexión y el número de
  paneles, comprobada en el simulador contra el puente real.

## Plan B
Si tauri-specta RC da problemas: tipos TS escritos a mano en `src/lib/bindings.ts` y un test de Rust que serializa un
valor de cada tipo y compara con un JSON de ejemplo que también lee un test de vitest.

## Referencias
- tauri-specta (docs de v2 para Tauri 2; consultar la versión actual con context7 antes de fijarla).
- Tauri 2: eventos de ciclo de vida en móvil (`RunEvent`), `app.path().app_data_dir()`.
- cornercase-gpui: `src/main.rs` (cómo arranca el `Store` y la conexión).

## design/ que actualiza
- `design/architecture.md`: comandos y eventos expuestos, generación de bindings y su comprobación en CI.
- `design/connection.md`: qué dispara `resume_now()`.

## Verificación
CI verde (incluida la comprobación de bindings); en el simulador, matar el puente y arrancarlo de nuevo hace que la
pantalla pase a "reintentando" y vuelva sola; mandar la app a segundo plano y volver reconecta.
