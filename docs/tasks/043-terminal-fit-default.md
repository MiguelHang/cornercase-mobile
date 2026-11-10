# 043 — Terminal: ajustada al móvil por defecto, y su scroll

- Estado: hecha (el arrastre por Tailscale y el pellizco de vuelta a la base con el teclado abierto quedan para el iPhone)
- Depende de: 022
- Rama: `feat/043-terminal-fit-default` · PR: `feat: fit the terminal to the phone by default and fix its scroll`

## Objetivo
Del iPhone del usuario (029, 2026-11-10): "la terminal siempre debería hacerse fit; aparte, en modo fit el scroll va mal".

## Criterios de aceptación
- [x] CA1 — Al abrir la terminal se pide el ajuste (022) sin tocar nada; el interruptor sigue para volver a la vista
  pasiva si el Mac la necesita a su tamaño. Revisar y actualizar la decisión "pasiva por defecto" de `design/terminal.md`
  y `docs/IDEA.md` con el motivo.
- [x] CA2 — En modo ajustado el scroll funciona: arrastrar en vertical recorre el historial (scroll del PTY o de la
  rejilla, según lo que haya) sin saltos ni pelearse con el swipe de cursor de la 021; describir qué estaba mal.
- [x] CA3 — Comprobado en el simulador y pendiente en el iPhone; tests; `hx check full` verde.
