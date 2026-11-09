# 039 — Chat: ver las preguntas y menús del agente sin ir a la terminal

- Estado: pendiente
- Depende de: 036, 020
- Riesgo: medio — cornercase no da la pregunta pendiente por la CLI; la primera versión la saca de la pantalla del panel.
- Rama: `feat/039-chat-questions` · PR: `feat: show the agent's question and its options in the chat`

## Objetivo
Cuando el agente pregunta algo con un menú (Claude Code `AskUserQuestion`, un permiso, elegir entre opciones), verlo y
contestarlo desde el chat, sin abrir la terminal. Hoy el chat solo lee el último mensaje (`read --last-message`), que no
incluye el menú: en `waiting` enseña la nota "<agent> is asking you something on its screen; answer with the keys" y la
fila de teclas, pero no la pregunta ni las opciones. El usuario lo vio en su iPhone el 2026-11-09: "cuando Claude te saca
un menú con varias preguntas y opciones, no me lo ha mostrado en el chat, solo lo he visto en terminal".

## Criterios de aceptación
- [ ] CA1 — Mientras el agente está `waiting`, el chat enseña la pregunta tal cual se ve en su pantalla: las últimas
  filas del panel (lo que ocupa el menú), con los colores y el resaltado de la opción elegida, en monoespaciado y a lo
  ancho del chat (con scroll horizontal si hace falta). Sale de lo que ya hay para la terminal (019/020: `surface` del
  panel, la rejilla en Rust), sin reclamar la pestaña ni cambiar su tamaño, y se actualiza al mover la selección con las
  teclas. Al dejar `waiting`, desaparece.
- [ ] CA2 — Qué filas: desde el principio del menú hasta el final de la pantalla, sin el resto del historial;
  documentar cómo se acota (p. ej. las filas no vacías desde el final hasta la primera línea en blanco o el borde del
  cuadro) y su límite.
- [ ] CA3 — Contestar sigue siendo con las teclas (017): `↑`/`↓`, números, `Enter`, `Esc`, con la fila justo debajo de
  la pregunta. Si las opciones son numeradas, valorar botones grandes por opción que manden ese número.
- [ ] CA4 — Pregunta estructurada (upstream): proponer en `design/upstream-gaps.md` y en la épica de la 033 un comando
  que dé la pregunta pendiente como datos (`read --pending --json` o similar: pregunta, opciones, selección, multi-select,
  de qué herramienta viene), para pintar las opciones como botones nativos. Si upstream ya lo tiene cuando se haga esta
  tarea, usarlo en lugar de CA1.
- [ ] CA5 — Comprobado en el simulador con Claude Code real haciendo `AskUserQuestion` (una pregunta y varias), un
  permiso de herramienta y un menú de Codex si se puede; pendiente para el usuario en su iPhone.
- [ ] CA6 — `hx check full` y mutation en verde; tests del recorte de filas sin DOM.

## Referencias
- `design/chat.md` (016–018): waiting takes no text, Keys.
- `design/terminal.md` (019–022): surface, la rejilla, `projection`.
- cornercase-terminal (`origin/main`): `design/agent-status.md` (waiting), bots de `away` (cómo enseñan una pregunta
  pendiente, si lo hacen).

## design/ que actualiza
- `design/chat.md`: la pregunta en el chat y de dónde sale.
- `design/upstream-gaps.md`: la pregunta pendiente como datos.
