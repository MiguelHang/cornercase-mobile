# 048 — El chat como conversación: historial en el móvil, y el de cornercase cuando exista

- Estado: pendiente
- Depende de: 036, 041
- Rama: `feat/048-chat-history` · PR: `feat: keep the chat's conversation on the phone`

## Objetivo
Del usuario (2026-11-10): "quiero que se vea como un chat, no solo el último mensaje". cornercase solo da el último
mensaje por la CLI (`read --last-message`); el historial es el borrador 05 de la épica de la 033 (`read --messages`,
paginado). Decidido con el usuario (opción 3): historial local ya, y el de cornercase cuando llegue, sin cambiar la
pantalla.

## Criterios de aceptación
- [ ] CA1 — La app guarda, por máquina, `boot` y panel, cada mensaje que envía (con su estado, 036) y cada último
  mensaje del agente que lee y sea distinto del anterior, con su hora; persistido en el dispositivo (fichero de la app,
  no `localStorage`), con un tope (p. ej. 200 mensajes por agente) y borrado al quitar la máquina o cuando el `boot`
  cambia.
- [ ] CA2 — El chat se ve como una conversación: burbujas tuyas y del agente en orden, scroll al final, la línea del
  agente debajo (036), la pregunta (039/042) al final.
- [ ] CA3 — Los huecos se dicen: cuando hubo mensajes que el móvil no vio (el agente siguió mientras no mirabas, o
  escribiste desde el Mac), una línea "Earlier messages are on the Mac" entre burbujas, sin inventar nada.
- [ ] CA4 — Fuente intercambiable: la conversación sale de una interfaz (p. ej. `History`) con la implementación local;
  si `contract` anuncia el historial de cornercase (borrador 05), se usa ese y el local solo rellena lo nuevo. Anotar
  en `design/chat.md` y en `design/upstream-gaps.md`.
- [ ] CA5 — Comprobado en el simulador con un agente real (varios turnos, salir y volver, reiniciar la app); tests;
  `hx check full` verde.
