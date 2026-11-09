# 036 — Chat: ver lo que has enviado y qué está haciendo el agente

- Estado: pendiente
- Depende de: 016, 017, 018
- Riesgo: medio — cambia una decisión de la 016 ("No optimistic state") y toca la pantalla del chat, que la 018 también toca.
- Rama: `feat/036-chat-feedback` · PR: `feat: show what you sent and what the agent is doing in the chat`

## Objetivo
Al escribir en el chat, el usuario tiene que saber en todo momento **si su mensaje ha salido** y **qué está haciendo el
agente** (pensando, esperándole, atascado). Hoy, al enviar, el campo se vacía y en pantalla sigue solo el último mensaje
del agente; el estado `working` solo cambia en la cabecera, en pequeño. El usuario lo probó en su iPhone el 2026-11-09:
"cuando escribes en el chat no aparece lo que has escrito, entonces es raro, parece que no hay feedback. No sabes si
está pensando Claude o se ha quedado pillado ni tampoco sabes si el mensaje se ha enviado".

## Criterios de aceptación
- [ ] CA1 — Tu mensaje se ve: al pulsar `Send`, el texto aparece en el hilo como mensaje tuyo (burbuja o bloque
  propio, alineado distinto del del agente), con su estado: **enviando** (mientras la cola lo manda), **enviado** (cuando
  cornercase lo confirma) o **no enviado** (con el motivo y una forma de reintentar o recuperar el texto al campo).
  Sustituye la decisión "No optimistic state" de `design/chat.md` (anotar por qué cambia).
- [ ] CA2 — Qué hace el agente, bien visible: mientras está `working`, una línea viva bajo tu mensaje (p. ej. el
  `StatusDot` en órbita y "Claude is working…", con el tiempo que lleva); `waiting` y `failed` con su texto; `done`/`idle`
  sin indicador. Sale del estado de la fila (`agents-changed`), sin polling nuevo.
- [ ] CA3 — Respuesta en curso: si el último mensaje del agente crece mientras trabaja, se ve crecer (ya se relee con
  `pane-status-changed`; comprobar que la pantalla lo enseña sin saltos y con scroll al final si el usuario estaba al final).
- [ ] CA4 — Orden del hilo: tu último mensaje queda **después** del mensaje del agente al que contestaba y antes de la
  respuesta nueva; cuando llega la respuesta nueva (`last_message` cambia), tu mensaje sigue encima. Como el CLI solo da
  el último mensaje, el hilo es corto (tu último envío y la respuesta); no hay historial completo (anotarlo en
  `design/upstream-gaps.md` si no está).
- [ ] CA5 — ¿Atascado?: si el agente lleva mucho en `working` sin que cambie el mensaje (umbral a decidir, p. ej.
  2 min), la línea de estado lo dice con suavidad ("no news for 2 min") y recuerda que `Ctrl-C` interrumpe; sin alarmas.
- [ ] CA6 — Al volver a la pantalla (Back y abrir otra vez, o volver de la terminal) se conserva el último mensaje
  enviado y su estado mientras el `Chat` viva; al reabrir el agente desde el tablero puede empezar limpio (decidir y
  anotar).
- [ ] CA7 — VoiceOver: el estado del envío y del agente se anuncian (región viva), sin repetir cada segundo.
- [ ] CA8 — Comprobado en el simulador con un agente real (enviar, ver "enviando → enviado", "working" y la respuesta);
  pendiente para el usuario en su iPhone.
- [ ] CA9 — `hx check full` y mutation en verde; tests de los estados del envío y de la línea del agente.

## Referencias
- `design/chat.md` (016–018): What comes back, No optimistic state, Reading the message, Header.
- `design/board.md` (`statusName`, `StatusDot`).
- cornercase-gpui: cómo enseña el envío y el estado del agente en su chat, si lo tiene.

## design/ que actualiza
- `design/chat.md`: el hilo corto, los estados del envío, la línea del agente, "atascado".
- `design/look.md`: el mensaje propio y la línea de estado.

## Verificación
En el iPhone, al enviar se ve el mensaje propio con su estado y se nota en todo momento si el agente trabaja, espera o
se ha quedado parado.
