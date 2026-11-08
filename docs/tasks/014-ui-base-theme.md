# 014 — UI base: tema de gpui, cristal, navegación, avisos y primera máquina pegando la URL

- Estado: pendiente
- Depende de: 013 · decisión **D3**
- Riesgo: bajo
- Rama: `feat/014-ui-base` · PR: `feat: add the app's look, navigation and first machine form`

## Objetivo
El armazón de la UI sobre el que se montan las pantallas: aspecto de cornercase-gpui adaptado al móvil, navegación,
avisos, estado de la conexión y una forma mínima de añadir la primera máquina pegando la URL del puente.

## Criterios de aceptación
- [ ] CA1 — `src/lib/theme/tokens.css`: colores, radios, espacios, tipografías y duraciones portados de `src/theme.rs`
  de gpui (tema oscuro de usecornercase.dev, cobalto para lo elegido, verde/ámbar/rojo de estados) como propiedades CSS.
- [ ] CA2 — Componentes base en `src/lib/ui/`: `Glass` (tarjeta de cristal: `backdrop-filter` sobre el fondo),
  `Button` (normal, primario, peligro), `Chip`, `StatusDot` (trabajando con órbita animada, esperando, terminado,
  fallido), `Spinner`, `Sheet` (sube desde abajo), el logo (`Mark`). Movimiento con transiciones de Svelte; respeta
  `prefers-reduced-motion`.
- [ ] CA3 — Navegación por pantallas (router mínimo propio o `svelte-spa-router`; justificar), con safe areas de iOS
  (`env(safe-area-inset-*)`) y `viewport-fit=cover`.
- [ ] CA4 — Avisos (toasts) desde el evento `toast` de 013: máximo 3, caducan por tipo (info 4 s, error 8 s).
- [ ] CA5 — Barra de estado de conexión: conectando, conectado (por qué ruta), reintentando en N s, faltan comandos
  (lista de 010), token inválido.
- [ ] CA6 — Sin máquinas: pantalla de bienvenida con un campo "pega la URL del puente" + nombre, que llama a
  `machine_add` y conecta. (La pantalla completa de Máquinas es 027.)
- [ ] CA7 — vitest para la lógica sin marcado: caducidad de avisos, formato del estado de conexión.

## Referencias
- cornercase-gpui: `design/widgets.md` (cristal en capas, radios, cobalto sin barras, el logo, movimiento, "nada se
  anima para siempre en reposo"), `src/theme.rs`, `src/ui/glass.rs`, `src/ui/signs.rs`, `src/ui/motion.rs`,
  `src/ui/brand.rs`, `src/app/toasts.rs`.
- Orca: `mobile/app/` (estructura de pantallas en móvil).

## design/ que actualiza
- `design/look.md`: qué tokens se portaron, qué cambia en táctil (tamaños de toque ≥ 44 pt, sin hover), el router elegido.

## Verificación
`pnpm check`, `pnpm lint`, `pnpm test` verdes; en el simulador se añade la máquina pegando la URL y la barra pasa a
"conectado".
