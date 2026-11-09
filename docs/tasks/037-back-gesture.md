# 037 — Volver atrás con el gesto de iOS y un chevron en todas las pantallas

- Estado: pendiente
- Depende de: 036
- Riesgo: medio — el gesto de borde compite con el scroll horizontal de la terminal, el pellizco y el swipe de la 021.
- Rama: `feat/037-back-gesture` · PR: `feat: go back with the edge swipe and a chevron on every screen`

## Objetivo
Volver atrás como en cualquier app de iOS: deslizando desde el borde izquierdo, y con un `‹` arriba a la izquierda en
vez de un botón `Back` con texto. Pedido por el usuario al probar en su iPhone el 2026-11-09: "¿no vamos a añadir los
gestos para ir atrás? O por lo menos poner el chevron en vez de un botón Back".

## Criterios de aceptación
- [ ] CA1 — Barra de navegación como en las apps de Apple (Mail, Ajustes, Notas), en todas las pantallas con pila
  (agente/chat, terminal, Start, cambios, un diff, Machines, una máquina): una cabecera fija arriba, sobre `--veil` con
  el desenfoque del vidrio, con el botón de volver a la izquierda **dentro de esa cabecera**: `‹` (`chevron-left`) seguido
  del título de la pantalla anterior en `--brand` (`‹ Agents`, `‹ Machines`; solo `‹` si no cabe), zona táctil de 44 px,
  `aria-label="Back to <pantalla>"`. El título de la pantalla actual va centrado en la barra (o grande debajo, que se
  encoge al hacer scroll, como pide la 036 para el chat). Desaparecen los botones `Back` y `Back to the board` con texto;
  el agente que ya no está usa la misma barra. Un componente compartido (p. ej. `NavBar`) que reemplace las cabeceras
  repetidas (`.top`, `.back`, `.title` de Machines, Machine, Changes, Diff, Start). Pedido por el usuario el 2026-11-09:
  "que el chevron de back vaya como en las apps de Apple, en la cabecera".
- [ ] CA2 — Gesto de borde: deslizar desde el borde izquierdo (≤ 20–24 pt) hacia la derecha hace `router.back()`, con la
  pantalla siguiendo al dedo y la de debajo asomando (o, como mínimo, un desplazamiento con umbral y la transición de
  `motion`); soltar antes del umbral la devuelve a su sitio. Respeta Reduce Motion.
- [ ] CA3 — Sin pisar otros gestos: en la terminal, el pellizco, el scroll horizontal y el swipe de cursor de la 021 siguen
  funcionando; solo un toque que empieza en el borde es el gesto de volver. En el chat, el scroll y la selección de
  texto siguen igual. Investigar si WKWebView ofrece `allowsBackForwardNavigationGestures` y por qué no sirve (no hay
  historial de navegación: el router es estado), y anotarlo.
- [ ] CA4 — Nada que hacer en la base: en el tablero y en `no-machines` el gesto no hace nada.
- [ ] CA5 — Android (032) puede llamar al mismo `back()` con su botón atrás; anotarlo en `design/look.md`.
- [ ] CA6 — Comprobado en el simulador (gesto con XCUITest o simctl) y pendiente para el usuario en su iPhone; tests del
  reconocedor del gesto sin DOM (umbral, borde, cancelación).
- [ ] CA7 — `hx check full` y mutation en verde.

## design/ que actualiza
- `design/look.md` › Navigation: la barra de navegación, el botón de volver con el título anterior, el gesto de borde y su convivencia con los demás gestos.
