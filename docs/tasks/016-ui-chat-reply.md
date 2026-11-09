# 016 — Pantalla 2: chat con el agente — último mensaje y responder

- Estado: hecha
- Depende de: 015
- Riesgo: bajo
- Rama: `feat/016-chat` · PR: `feat: read an agent's last message and answer it`

## Objetivo
Ver lo último que dijo un agente y contestarle con texto.

## Criterios de aceptación
- [x] CA1 — Núcleo: `read --last-message --pane P --json` tipado; se vuelve a leer cuando llega un evento `status` de
  ese panel (p. ej. pasa a `waiting` o `done`). `LastMessage` y el comando `last_message`; el evento
  `pane-status-changed` sale de cada `status` con panel y de cada `dropped` (`design/chat.md` › Reading the message).
- [x] CA2 — `send --pane P --enter` por la cola (011) con el texto por stdin o argumento (según D1 y lo que acepte
  `send`); si el servidor rechaza (`busy`, agente no esperando…), aviso con su mensaje. Por stdin, con
  `--timeout 30`; el rechazo va en un aviso sin su `Error:` (`design/chat.md` › Answering).
- [x] CA3 — Pantalla `src/routes/agent/[pane]/`: cabecera con agente, estado y workspace; el mensaje como markdown
  (librería ligera, justificada) con bloques de código desplazables; campo de texto multilínea que crece y botón enviar.
  Va en `src/routes/agent/`, porque el router propio no tiene carpetas por parámetro; el markdown, con
  `mdast-util-from-markdown` y GFM, se pinta como árbol sin `{@html}` (`design/chat.md` › Markdown).
- [x] CA4 — Tras enviar se ve el estado pasar a `working` y el campo se vacía; si falla, el texto se conserva.
  Comprobado en el simulador contra el cornercase del Mac (`design/chat.md` › Checked live).
- [x] CA5 — Si el panel desaparece (`closed`), la pantalla lo dice y ofrece volver al tablero. Para siempre, aunque
  vuelva el mismo panel (`design/chat.md` › Gone); comprobado en el simulador cerrando el panel en el Mac.
- [x] CA6 — vitest del render de markdown seguro (sin HTML crudo) y del estado del campo.

## Referencias
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents/reading.md`, `…/agent-sends.md`,
  `…/confirming-sends.md`, `…/held-sends.md`, `…/typing-and-waiting.md`.
- cornercase-gpui: `src/terminal/steps.rs` (cómo envía y espera), `src/notify/feed.rs`.
- Orca: `mobile/rpc-foundation/goldens/matrix-nativechat.*` (forma del chat nativo).

## design/ que actualiza
- Nuevo `design/chat.md` (y su línea en `DESIGN.md`): qué se lee, cuándo se vuelve a leer, cómo se envía.
- `design/upstream-gaps.md`: confirmar el límite del historial.

## Verificación
En el simulador: un agente que espera recibe la respuesta y vuelve a trabajar (criterio de éxito 2, parte texto).
