# 041 — Chat: barra de escribir estilo Claude/WhatsApp y quitar una imagen adjunta

- Estado: pendiente
- Depende de: 036, 039
- Rama: `feat/041-chat-composer` · PR: `feat: a rounded composer with attach inside, and remove an attached image`

## Objetivo
Del iPhone del usuario (029, 2026-11-10): "que la barra donde escribes sea más estilo Claude o WhatsApp" (adjuntó la app
de Claude: una cápsula redondeada con `+` a la izquierda, el texto en medio y el botón de enviar circular a la derecha,
que crece hacia arriba con el texto) y "poder eliminar una imagen cuando la subes".

## Criterios de aceptación
- [x] CA1 — El campo es una cápsula (`--radius-sheet` o pill) con el `+` de adjuntar dentro a la izquierda y un botón
  circular de enviar dentro a la derecha (flecha `arrow-up`, en `--brand` solo con algo que enviar); crece con el
  texto hacia arriba hasta su máximo; el teclado la empuja sin huecos.
  Cápsula, `+`, `Send` redondo y el crecimiento, vistos en el simulador. Con el teclado abierto, la pantalla del chat
  mide lo que el teclado deja a la vista (revisión 1, opción a): la barra de conexión, la cabecera compacta, el hilo y tu
  burbuja quedan encima de la cápsula, y el campo queda a 26 pt de la barra ‹ › ✓ de WKWebView (16 son el relleno
  inferior del compositor) en lugar de 44 (`design/chat.md` › The screen › With the keyboard open, Checked live).
- [x] CA2 — Las teclas especiales (017) se esconden tras un icono dentro de la cápsula (p. ej. `keyboard` o `command`):
  cerradas por defecto, un toque las abre sobre la cápsula y otro las cierra; se abren solas mientras el agente está
  `waiting` (y la pregunta de 039 está a la vista) y se vuelven a cerrar cuando deja de estarlo. Pedido por el usuario
  el 2026-11-10: "esconder las teclas especiales en un iconito y cuando se quieran usar, al darle se abren; así
  ahorramos espacio".
- [x] CA2b — `Send` vacía el campo al pulsarlo, no cuando cornercase confirma: el texto pasa a la burbuja de tu
  mensaje (enviando → enviado, 036) y el campo queda libre para seguir escribiendo. Si el envío falla, la burbuja lo
  dice y su `Edit` devuelve el texto al campo (036). Pedido por el usuario el 2026-11-10.
- [x] CA3 — Cada imagen adjunta sale como miniatura sobre la cápsula con una `×` para quitarla antes de enviar; quitarla
  borra la ruta pegada del agente si ya se pegó (o, si upstream no lo permite, la deja y lo dice; anotar el hueco).
  Las imágenes esperan en el teléfono hasta `Send`, así que una quitada antes nunca llega al Mac; la ya pegada tras un
  envío fallido se queda en el input y un toast lo dice (`design/upstream-gaps.md` › A pasted file cannot be taken back).
- [x] CA4 — Nada de lo que funciona se pierde: estados del envío (036), `--force` tras una imagen (018), la pregunta (039).
- [ ] CA5 — Comprobado en el simulador y en el iPhone del usuario; tests; `hx check full` verde.
  Simulador hecho (Verificación) y `hx check full` verde (tester); falta el iPhone del usuario.

## Verificación
En el iPhone, la barra es una cápsula como la de Claude o WhatsApp, las teclas salen solo cuando hacen falta y una
imagen adjunta se puede quitar antes de enviarla.
Hecha el 2026-11-10 en el simulador iPhone 17e (390 pt, iOS 27.2), con la app de la rama, contra un puente propio en
`127.0.0.1` y un cornercase 0.13.31 propio con Claude Code 2.1.322 (Haiku 5.5), los toques hechos por XCUITest
(`design/chat.md` › Checked live). Dos imágenes adjuntas (roja y azul) salieron como miniaturas sin pegar nada en el
Mac; la `×` de la azul la quitó; un texto de dos líneas con la roja vació el campo al pulsar `Send`, con el teclado
abierto, y el puente corrió un solo `paste-file`: Claude recibió `[Image #1]` y contestó `One image, a solid red.`
Pedirle una pregunta con `AskUserQuestion` lo puso en `waiting` y abrió las teclas sobre la cápsula con la pregunta y
sus botones; `Option 1, Apple` contestó, el agente pasó a `done` y las teclas se cerraron solas. El icono `Keys` las
abre y las cierra a mano. El hueco de 44 pt sobre la barra del teclado (CA1) se resolvió en la revisión 1: la pantalla mide el viewport visible. Sin ver aquí: quitar una
imagen ya pegada tras un envío fallido, y su toast (cubierto por tests). El iPhone del usuario queda pendiente.
