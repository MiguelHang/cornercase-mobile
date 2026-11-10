# 040 — Tablero: plegado que no parezca un "ir" y buscador de agentes

- Estado: hecha
- Depende de: 038
- Rama: `feat/040-board-fold-search` · PR: `feat: show folded subagents as a thread and search the board`

## Objetivo
Del iPhone del usuario (029, 2026-11-10): "el chevron de los subagentes plegados parece el botón de ir; igual usar otro
recurso, una flecha en el lado izquierdo o como en el inbox del iPhone cuando ves un hilo" y "igual metía un buscador
para filtrar agentes".

## Criterios de aceptación
- [x] CA1 — El plegado se lee como un hilo, no como navegación: a la izquierda de la fila padre (como Mail agrupa un
  hilo), un disclosure que gira (`chevron-right` → `chevron-down`) junto al número de subagentes, o el contador de Mail
  (`3 ›` a la derecha en gris pequeño, sin parecer la flecha de "abrir"). Nada en el borde derecho que se confunda con
  ir al chat. Decidir con capturas en el simulador y anotarlo en `design/board.md`.
- [x] CA2 — Buscador arriba del tablero (como el de Mail/Ajustes: aparece al tirar hacia abajo o fijo bajo la barra),
  que filtra por título, agente, proyecto, workspace y rol, sin distinguir mayúsculas ni acentos; con texto, los hijos
  que coinciden se muestran aunque su padre esté plegado; `Cancel` lo limpia.
- [x] CA3 — VoiceOver: el disclosure dice "Show 3 subagents"/"Hide"; el buscador tiene su etiqueta.
- [x] CA4 — Comprobado en el simulador; tests de filtrado sin DOM; `hx check full` verde.
