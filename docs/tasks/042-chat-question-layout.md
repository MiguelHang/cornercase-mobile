# 042 — Chat: la pregunta del agente se lee sin scroll horizontal

- Estado: hecha
- Depende de: 039
- Rama: `feat/042-question-layout` · PR: `feat: lay the agent's question out to the phone's width`

## Objetivo
Del iPhone del usuario (029, 2026-11-10): "el estilo de pregunta AskUserQuestion se ve un poco mal, hay que hacer scroll
horizontal, podemos mejorar esa UI".

## Criterios de aceptación
- [x] CA1 — La pregunta no se pinta como un trozo de terminal a su ancho: de las filas recortadas (039) se sacan la
  pregunta, las pestañas de varias preguntas y las opciones con su descripción, y se maquetan al ancho del chat con
  saltos de línea (tarjeta con la pregunta en texto normal, opciones como botones a todo el ancho con su descripción
  debajo en `--muted`, la elegida marcada). Sin scroll horizontal.
  El core lee la tarjeta de las filas recortadas (`crates/core/src/terminal/question/card.rs`) y viaja como `card` en
  `question-changed`; `QuestionCard.svelte` la pinta con salto de línea, también el comando de un permiso (`pre-wrap`).
  Hueco aceptado: el detalle no se distingue de la continuación de una etiqueta (`design/upstream-gaps.md`).
- [x] CA2 — Si una pantalla no se deja leer así, se cae al modo actual (filas monoespaciadas) sin romperse; anotarlo.
  Menos de dos opciones, una fila sin clase bajo una opción, más de dos filas de pie o un `╌` sin cerrar: sin tarjeta y
  las filas de antes (`design/chat.md` › The question › The card › When it falls back).
- [x] CA3 — Las pestañas (`☐ Colour ☐ Size ✔ Submit`) son una fila de chips que se puede tocar (manda las flechas que
  toquen) o, como mínimo, se ven enteras.
  Tocables: un toque manda las flechas en una sola llamada a `keys`, que toma una lista. La abierta es el chip con un
  fondo propio; sin una así, los chips se ven pero no se tocan.
- [x] CA4 — Comprobado con `AskUserQuestion` real (una y varias preguntas, opciones largas) en el simulador; tests del
  parser con los fixtures de 039; `hx check full` verde.
  Simulador hecho (Verificación) y `hx check full` verde el 2026-11-10; falta probarlo en el iPhone del usuario.

## Verificación
En el iPhone, la pregunta del agente se lee entera a lo ancho del teléfono, sin scroll lateral: el texto y las
descripciones hacen salto de línea, las opciones son botones a todo el ancho y las pestañas, chips que se tocan.
Hecha el 2026-11-10 en el simulador iPhone 17e (390 pt, iOS 27.2), con la app de la rama, contra un puente propio en
`127.0.0.1` y un cornercase 0.13.31 propio con Claude Code 2.1.322 (Haiku 5.5), los toques hechos por XCUITest
(`design/chat.md` › Checked live). Una pregunta de cuatro colores salió como tarjeta y `↓` movió la marca; dos preguntas
en pestañas con descripciones de unas 50 palabras se leyeron enteras, los chips cambiaron de pestaña y la de `Submit`
también salió como tarjeta: `You chose Banana for the fruit and Medium for the size.` Un permiso de Bash salió con su
comando en un bloque que hace salto de línea, y `1. Yes` lo ejecutó. Dos arreglos salieron de aquí: la tarjeta sigue a la
opción elegida también cuando cambia de alto (con el teclado y las teclas abiertos se encogía a unos 40 pt), y el comando
pierde la barra `│` que Claude Code 2.1.322 le pinta delante (fixture `permission_gutter.json`). Sin resolver: con una
pregunta y las teclas abiertas, enfocar el campo a veces sube la página unos 56 pt; sin pregunta no pasa, y no se
comparó con `main`. Tras la review (2026-11-11), con tres preguntas de cuatro opciones de unas 40 palabras (30 filas desde
la regla), la tarjeta salió entera, sin la barra `│` en la pregunta, con los chips fijos arriba al hacer scroll. Tras la
segunda review (2026-11-11), con la tarjeta desplazada, cada pestaña abierta con un chip o al contestar empezó por su
pregunta con la opción elegida a la vista, el resumen de `Review your answers` salió sin `│` y los chips volvieron a
mandar sus flechas pasados 1,5 s sin cambio de pantalla. Tras la tercera review (2026-11-11), con tres preguntas de unas
cinco filas y opciones de unas 45 palabras, la pregunta empezó justo bajo los chips en la primera vista, al tocar un
chip y al contestar, con la `1.` debajo; en `Review your answers` ganó el resumen y `1. Submit answers` quedó a un
swipe, que la alcanzó y envió las respuestas. Tras la cuarta review (2026-11-11), el `❯` de la elegida quedó en la línea
de su `N. etiqueta`, a la vista sobre el borde de la tarjeta, y la rama se mezcló con `main` (048): `openTab` y el
historial del chat conviven. Tras la quinta review (2026-11-12), la tarjeta guarda un mínimo de la banda de chips y tres
líneas: con el teclado abierto tras `Send`, la pregunta llegó con sus chips, la pregunta y el principio de `1. Cat` a la
vista y la barra sin tapar; el campo quedó bajo el borde del compositor hasta escribir. El iPhone del usuario queda
pendiente.
