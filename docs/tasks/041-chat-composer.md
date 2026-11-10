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
- [ ] CA2 — Las teclas especiales (017) se esconden tras un icono dentro de la cápsula (p. ej. `keyboard` o `command`):
  cerradas por defecto, un toque las abre sobre la cápsula y otro las cierra; se abren solas mientras el agente está
  `waiting` (y la pregunta de 039 está a la vista) y se vuelven a cerrar cuando deja de estarlo. Pedido por el usuario
  el 2026-11-10: "esconder las teclas especiales en un iconito y cuando se quieran usar, al darle se abren; así
  ahorramos espacio".
- [ ] CA2b — `Send` vacía el campo al pulsarlo, no cuando cornercase confirma: el texto pasa a la burbuja de tu
  mensaje (enviando → enviado, 036) y el campo queda libre para seguir escribiendo. Si el envío falla, la burbuja lo
  dice y su `Edit` devuelve el texto al campo (036). Pedido por el usuario el 2026-11-10.
- [ ] CA3 — Cada imagen adjunta sale como miniatura sobre la cápsula con una `×` para quitarla antes de enviar; quitarla
  borra la ruta pegada del agente si ya se pegó (o, si upstream no lo permite, la deja y lo dice; anotar el hueco).
- [ ] CA4 — Nada de lo que funciona se pierde: estados del envío (036), `--force` tras una imagen (018), la pregunta (039).
- [ ] CA5 — Comprobado en el simulador y en el iPhone del usuario; tests; `hx check full` verde.
