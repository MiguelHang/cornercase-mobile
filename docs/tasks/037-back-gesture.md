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
- [ ] CA1 — Un solo botón de volver en todas las pantallas con pila (agente/chat, terminal, Start, cambios, un diff,
  Machines, una máquina): `chevron-left` de 44 px arriba a la izquierda, con `aria-label="Back"`, como ya hacen Machines y
  una máquina (027). Desaparecen los `Back` y `Back to the board` con texto (el agente que ya no está ofrece el mismo
  chevron). Un componente compartido (p. ej. `ScreenHeader` o `BackButton`) si evita repetir CSS.
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
- `design/look.md` › Navigation: el chevron, el gesto de borde y su convivencia con los demás gestos.
