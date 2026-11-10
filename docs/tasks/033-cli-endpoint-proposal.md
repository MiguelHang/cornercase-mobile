# 033 — Propuesta de `/cli` para cornercase-terminal

- Estado: pendiente
- Depende de: 029 (se hace al final del roadmap, con todo lo aprendido; decidido el 2026-11-09)
- Riesgo: bajo
- Rama: `docs/033-cli-proposal` · PR: `docs: propose the bridge's format as /cli for cornercase`

## Decisión (2026-11-09)
No se guarda en el repo ni lleva PR. El resultado son borradores de issues para cornercase-terminal, agrupadas en una
épica (issue padre con sub-issues nativas de GitHub), cada una autocontenida para que un agente la coja y la lance en su
worktree. Entran la propuesta de `/cli` y los huecos de `design/upstream-gaps.md`, entre ellos "The pending question is
not data" (039: `read --pending --pane P --json` o un campo `question` en `status`, con la pregunta, sus opciones con
número, etiqueta y descripción, la selección, multi-select, las pestañas de varias preguntas, la herramienta de origen y
el agente). No se crea nada sin el OK del usuario. Los requisitos del primer análisis están aparcados en
`.harness/parked/t033/` (sin versionar). Los criterios de abajo describen el contenido; donde dicen fichero o enlaces en
`design/`, vale el borrador de la issue.

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

## Notas
- 035 (2026-11-09): el servidor real necesitaría algo como `--bridge-routes` del puente
  (`design/bridge/machine-code.md`): responder, sin token, las direcciones en las que escucha, para que la app añada
  las rutas que le falten a una máquina sin volver a escanear; y su QR, un código con todas sus direcciones, el token y
  el nombre del Mac.

## Referencias
- cornercase-terminal (`origin/main`): `src/web/mod.rs`, `src/web/http.rs`, `src/web/auth.rs`, `src/web/bridge.rs`,
  `src/cli/reach.rs`, `design/browser-access.md`, `design/remote-windows.md`,
  `design/commands-for-scripts-and-agents/contract.md`, `…/requests.md`, `src/control/pair.rs`.
- `design/bridge.md` de este repo.

## design/ que actualiza
- `design/bridge.md`, `design/upstream-gaps.md`.

## Verificación
Revisión del documento por el usuario (criterio de éxito 7).
