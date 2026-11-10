# 041 — Chat: barra de escribir estilo Claude/WhatsApp y quitar una imagen adjunta

- Estado: pendiente
- Depende de: 036, 039
- Rama: `feat/041-chat-composer` · PR: `feat: a rounded composer with attach inside, and remove an attached image`

## Objetivo
Del iPhone del usuario (029, 2026-11-10): "que la barra donde escribes sea más estilo Claude o WhatsApp" (adjuntó la app
de Claude: una cápsula redondeada con `+` a la izquierda, el texto en medio y el botón de enviar circular a la derecha,
que crece hacia arriba con el texto) y "poder eliminar una imagen cuando la subes".

## Criterios de aceptación
- [ ] CA1 — El campo es una cápsula (`--radius-sheet` o pill) con el `+` de adjuntar dentro a la izquierda y un botón
  circular de enviar dentro a la derecha (flecha `arrow-up`, en `--brand` solo con algo que enviar); crece con el
  texto hacia arriba hasta su máximo; el teclado la empuja sin huecos.
- [ ] CA2 — La fila de teclas (017) no ocupa siempre dos líneas encima: se pliega tras un botón (p. ej. un icono de
  teclado o `⌘` dentro de la cápsula) y se abre sola mientras el agente está `waiting`; decidir y anotarlo.
- [ ] CA3 — Cada imagen adjunta sale como miniatura sobre la cápsula con una `×` para quitarla antes de enviar; quitarla
  borra la ruta pegada del agente si ya se pegó (o, si upstream no lo permite, la deja y lo dice; anotar el hueco).
- [ ] CA4 — Tu mensaje enviado se queda a la vista: al enviar, el hilo enseña tu mensaje (enviando → enviado) y, debajo,
  la línea del agente ("Claude is working…"). Hoy (iPhone del usuario, 2026-11-10) al pasar el agente a `working` solo
  se ve la línea y tu mensaje desaparece: averiguar por qué (¿la relectura de `last_message` lo sustituye?) y que no
  pase; tu último mensaje sigue encima hasta que llegue la respuesta, y encima de ella después (036 › CA4).
- [ ] CA5 — Nada de lo que funciona se pierde: estados del envío (036), `--force` tras una imagen (018), la pregunta (039).
- [ ] CA6 — Comprobado en el simulador y en el iPhone del usuario; tests; `hx check full` verde.
