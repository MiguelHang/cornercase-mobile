# 042 — Chat: la pregunta del agente se lee sin scroll horizontal

- Estado: pendiente
- Depende de: 039
- Rama: `feat/042-question-layout` · PR: `feat: lay the agent's question out to the phone's width`

## Objetivo
Del iPhone del usuario (029, 2026-11-10): "el estilo de pregunta AskUserQuestion se ve un poco mal, hay que hacer scroll
horizontal, podemos mejorar esa UI".

## Criterios de aceptación
- [ ] CA1 — La pregunta no se pinta como un trozo de terminal a su ancho: de las filas recortadas (039) se sacan la
  pregunta, las pestañas de varias preguntas y las opciones con su descripción, y se maquetan al ancho del chat con
  saltos de línea (tarjeta con la pregunta en texto normal, opciones como botones a todo el ancho con su descripción
  debajo en `--muted`, la elegida marcada). Sin scroll horizontal.
- [ ] CA2 — Si una pantalla no se deja leer así, se cae al modo actual (filas monoespaciadas) sin romperse; anotarlo.
- [ ] CA3 — Las pestañas (`☐ Colour ☐ Size ✔ Submit`) son una fila de chips que se puede tocar (manda las flechas que
  toquen) o, como mínimo, se ven enteras.
- [ ] CA4 — Comprobado con `AskUserQuestion` real (una y varias preguntas, opciones largas) en el simulador; tests del
  parser con los fixtures de 039; `hx check full` verde.
