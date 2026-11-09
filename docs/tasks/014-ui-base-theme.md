# 014 — UI base: navegación, avisos, estado de la conexión y primera máquina pegando la URL

- Estado: hecha
- Depende de: 013, 034 · decisión **D3**
- Riesgo: bajo
- Rama: `feat/014-ui-base` · PR: `feat: add navigation, toasts and the first machine form`

## Objetivo
El armazón de la UI sobre el que se montan las pantallas, sobre el aspecto que ya trae la 034: navegación, avisos,
estado de la conexión y una forma mínima de añadir la primera máquina pegando la URL del puente en la pantalla "sin
máquinas" de la 034.

## Criterios de aceptación
- [x] CA1 — Navegación por pantallas (router mínimo propio o `svelte-spa-router`; justificar), sobre las safe areas
  que ya pone la 034. La pantalla "sin máquinas" de la 034 pasa a ser una ruta. Router propio, la ruta como estado
  (`design/look.md` › Navigation).
- [x] CA2 — Avisos (toasts) desde el evento `toast` de 013: máximo 3, caducan por tipo (info 4 s, error 8 s), en la
  tarjeta de cristal de la 034, entrando desde arriba con las transiciones de `motion.ts`. También los errores de
  los comandos; los repetidos se fusionan con `×N` y un toque los descarta (`design/look.md` › Toasts).
- [x] CA3 — Barra de estado de conexión: conectando (el punto y el cursor de la marca laten, solo mientras conecta),
  conectado (por qué ruta), reintentando en N s, faltan comandos (lista de 010), token inválido. También
  `incompatible`; el token inválido no tiene arreglo desde la app hasta la 027 (`design/connection/bar.md`).
- [x] CA4 — Primera máquina: el `Add machine` en reposo de la 034 se activa y abre un formulario con "pega la URL del
  puente" + nombre, que llama a `machine_add` y conecta; mientras trabaja, `Spinner` en el botón (`busy_button` de
  gpui). (La pantalla completa de Máquinas es 027.) El formulario va en la tarjeta de la pantalla vacía
  (`design/connection.md` › Adding a machine).
- [x] CA5 — Los componentes que esto estrena, con el aspecto de gpui: `Spinner`, `Sheet` (sube desde abajo) si el
  formulario va en una, `Field` (anillo cobalto de 3 px al enfocar, ≥ 16 px de letra para que iOS no haga zoom), y los
  tonos normal/peligro de `Button` que se usen. Sin `Sheet`, porque el formulario no va en una (`design/look.md` ›
  Forms); `Button` estrena el tono `plain`, y el de peligro llega con su primer uso, en la 027.
- [x] CA6 — vitest para la lógica sin marcado: caducidad de avisos, formato del estado de conexión, validación de la
  URL del formulario.

## Referencias
- cornercase-gpui: `src/app/toasts.rs`, `src/ui/motion.rs` (`pulse`, `spin`), `src/ui/brand.rs` (cursor que late),
  `src/ui/text_field.rs` y `design/widgets.md` (`busy_button`, "What takes a while turns a spinner", "Nothing animates
  forever while idle"), `design/connection.md` (estado de la conexión), `design/machines.md` (el formulario).
- Orca: `mobile/app/` (estructura de pantallas en móvil).

## design/ que actualiza
- `design/look.md`: el router elegido, los avisos, los componentes nuevos y qué cambia en táctil.
- `design/connection.md`: cómo se enseña el estado de la conexión.

## Verificación
`pnpm check`, `pnpm lint`, `pnpm test` verdes; en el simulador se añade la máquina pegando la URL y la barra pasa a
"conectado".
