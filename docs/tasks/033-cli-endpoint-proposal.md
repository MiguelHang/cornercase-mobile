# 033 — Propuesta de `/cli` para cornercase-terminal

- Estado: pendiente
- Depende de: 003–006 (puede empezar en cuanto el puente esté hecho); se cierra con lo aprendido en 029
- Riesgo: bajo
- Rama: `docs/033-cli-proposal` · PR: `docs: propose the bridge's format as /cli for cornercase`

## Objetivo
Documentar el formato del puente como propuesta para que cornercase-terminal sirva `/cli` en `src/web/`, y la app deje
de necesitar el puente.

## Criterios de aceptación
- [ ] CA1 — `docs/proposals/cli-endpoint.md` (en inglés, listo para pegar en una issue de upstream): motivo (clientes
  sin ssh ni binario de cornercase: móviles), el protocolo exacto (mensajes, orden, EOF y bytes de D1, cierre, códigos
  de salida, ping/silencio), la lista permitida (y que debe coincidir con `Kind::There` de `src/cli/reach.rs`),
  autenticación (encaje con el emparejamiento y la cookie/`browsers.json` de `design/browser-access.md` y con
  `config pair` de upstream en vez del token fijo), escucha (Tailscale serve en 8443 como el navegador),
  contrapresión y límites, y qué se probó con el puente (versiones).
- [ ] CA2 — Alternativas rechazadas con motivo: un WebSocket con multiplexado de comandos; exponer el socket del
  servidor; reutilizar `/ws` del navegador.
- [ ] CA3 — Encaje con el contrato: `contract` anuncia la feature (p. ej. `cli-endpoint`) para que un cliente sepa si
  puede usarlo; cómo convive con `requests --follow` (que ya da muchas órdenes en una conexión).
- [ ] CA4 — `design/bridge.md` y `design/upstream-gaps.md` enlazan la propuesta; el borrador de la issue queda en el
  fichero (no se abre la issue sin el visto bueno del usuario).

## Referencias
- cornercase-terminal (`origin/main`): `src/web/mod.rs`, `src/web/http.rs`, `src/web/auth.rs`, `src/web/bridge.rs`,
  `src/cli/reach.rs`, `design/browser-access.md`, `design/remote-windows.md`,
  `design/commands-for-scripts-and-agents/contract.md`, `…/requests.md`, `src/control/pair.rs`.
- `design/bridge.md` de este repo.

## design/ que actualiza
- `design/bridge.md`, `design/upstream-gaps.md`.

## Verificación
Revisión del documento por el usuario (criterio de éxito 7).
