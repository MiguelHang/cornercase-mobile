# 017 — Pantalla 2: chat — fila de teclas

- Estado: hecha
- Depende de: 016
- Riesgo: bajo
- Rama: `feat/017-chat-keys` · PR: `feat: send esc, tab, arrows, ctrl-c and numbers to an agent`

## Objetivo
Contestar a las preguntas de los agentes que se responden con una tecla (menús, confirmaciones, interrumpir).

## Criterios de aceptación
- [x] CA1 — Fila de botones: `Esc`, `Tab`, `↑`, `↓`, `←`, `→`, `Enter`, `Ctrl-C` y `1`–`9` (estos últimos plegados
  tras un botón para no llenar la fila), que mandan `keys --pane P <KEY>` por la cola.
  Los dígitos llevan también `0`, que cierra la encuesta de Claude Code (`design/chat.md` › Keys).
- [x] CA2 — `src/lib/keys.ts`: mapa botón → nombre de tecla de cornercase (los de `KEY_NAMES`), con vitest.
- [x] CA3 — `Ctrl-C` pide confirmación con un toque largo o un segundo toque (evitar interrumpir por accidente).
  Con un segundo toque en 3 s; el toque largo se descartó (`design/chat.md` › Keys).
- [x] CA4 — Respuesta háptica ligera al pulsar (plugin `haptics` de Tauri si existe para iOS; si no, sin háptica y
  anotado). Con `tauri-plugin-haptics` 2.4.2; en el simulador solo se comprobó que no falla
  (`design/chat.md` › Checked live).

## Referencias
- cornercase-terminal (`origin/main`): `src/cli/args.rs` (`KeysArgs`, `KEY_NAMES`),
  `design/commands-for-scripts-and-agents/driving-terminals.md`.
- cornercase-gpui: `src/keys.rs`.

## design/ que actualiza
- `design/chat.md`: teclas elegidas y por qué esas.

## Verificación
En el simulador se elige una opción de un menú de Claude Code con `↓` + `Enter` y se interrumpe con `Ctrl-C`.
Hecho en el iPhone 18 Pro el 2026-11-09 (`design/chat.md` › Checked live).
