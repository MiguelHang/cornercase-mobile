# 034 — UI look: tema de gpui, cristal, logo y pantalla "sin máquinas"

- Estado: hecha
- Depende de: 002 (no necesita red, ni el puente, ni 013)
- Riesgo: bajo
- Rama: `feat/034-ui-look` · PR: `feat: give the app cornercase's look and an empty first screen`
- Orden: se hace justo después de la 002 y antes de la 003, para ver algo en el simulador cuanto antes.

## Objetivo
Que la app se vea ya como cornercase en el simulador de iOS: el aspecto de cornercase-gpui (colores de
usecornercase.dev, cristal en capas sobre un fondo, formas redondeadas con aire, cobalto para lo elegido, el logo,
movimiento al entrar) portado a CSS y adaptado al táctil, y una primera pantalla estática "sin máquinas". Sin red, sin
comandos de Tauri: solo marcado, estilos y la lógica de movimiento. Lo que necesita datos (navegación, avisos, barra de
conexión, formulario de la primera máquina) es la 014.

## Criterios de aceptación
- [x] CA1 — Tokens: `src/lib/theme/tokens.css` con propiedades CSS portadas de `src/theme.rs` de gpui: colores
  (`glass`/ground `#0b0c0e`, `raised`, `surface`, `panel`, `field`, `veil`, `border`, `edge`, `hover`, `selected`;
  textos `text` `soft` `muted` `faint`; `brand` `#005fff`, `brand-hover`, `brand-light`, `on-brand`, `accent`
  `#74a3ff`, `accent-soft`, `selection`; estados `working` `waiting` `done` `failed` `merged`), radios
  (`chip` 5, `control` 8, `card` 12, `sheet` 16), espacios (`gap` 8 y los márgenes de panel 12–16), tipografías
  (sistema y monoespaciada del sistema) y duraciones (`enter` 180 ms, `leave` 120 ms, `pop` 110 ms). Mismos nombres
  que en `theme.rs` (en kebab-case) para que se puedan buscar en los dos repos. Nada de la paleta Tokyo Night de la
  terminal: llega con 020.
- [x] CA2 — Base: `src/lib/theme/base.css` pinta el fondo (ground opaco + el lavado cobalto de `glass::wash` arriba),
  `color-scheme: dark`, la tipografía, `-webkit-tap-highlight-color: transparent`, sin selección de texto en el cromo,
  y respeta las safe areas (`env(safe-area-inset-*)`); `index.html` lleva `viewport-fit=cover`.
- [x] CA3 — Componentes en `src/lib/ui/`, solo los que usa la pantalla de CA5 (sin código muerto):
  - `Glass.svelte`: la tarjeta flotante de `glass::card` (brillo arriba, línea superior que se desvanece desde la
    izquierda, borde de 1 px, sombra `lift`), con `backdrop-filter` y `-webkit-backdrop-filter` (iOS 15–17 solo
    entiende el prefijo; el `deploymentTarget` es 15.0).
  - `Button.svelte`: el tono primario (degradado `cobalt` + `glow`) y el estado "en reposo" de `resting_button`
    (atenuado e inerte). Tamaño de toque ≥ 44 pt; el pulsado es `:active`, no hover. Los tonos normal y peligro
    llegan con su primer uso (014 / 027).
  - `Emblem.svelte`: el emblema de `ui::emblem` (disco cobalto al 16 %, borde `brand-light` al 35 %, icono
    `accent`) con su título y sus líneas (`emblem_page`, `emblem_line`).
  - `Mark.svelte` y la marca de palabra: la esquina y el cursor sobre la rejilla de 12 de `ui/brand.rs` (esquina
    cobalto con brillo, cursor en `text`), escalable por `unit`. Sin parpadeo: el cursor solo late mientras se conecta,
    y eso es de la 014.
  - `Icon.svelte` con el SVG de Lucide `server` (el de la página vacía de Máquinas en gpui), trazo 1,75.
- [x] CA4 — Movimiento: `src/lib/ui/motion.ts` con las transiciones de Svelte de gpui (`slide`/`fade` de
  `ui/motion.rs`: subir 14 px y aparecer en 180 ms con ease-out cúbico al entrar, 120 ms ease-in al salir), que pasan
  a duración 0 con `prefers-reduced-motion: reduce`. Nada se anima para siempre en reposo.
- [x] CA5 — Pantalla "sin máquinas" (`src/routes/NoMachines.svelte` o equivalente; el router es de la 014),
  montada por `App.svelte`: la marca y `cornercase` arriba, y en una tarjeta de cristal la página de emblema de gpui
  (`Icon::Server`, `No machines yet`, una frase de qué es una máquina, otra de qué necesita: el puente en el Mac y su
  URL) con `Add machine` primario en reposo, que la 014 activa. La tarjeta entra con la transición de CA4. Textos en
  inglés.
- [x] CA6 — Sin destello blanco ni barra de estado ilegible: fondo oscuro en el arranque (`backgroundColor` de la
  ventana en `tauri.conf.json` y/o `LaunchScreen.storyboard`) y `UIUserInterfaceStyle: Dark` en
  `src-tauri/gen/apple/project.yml` (y el `.xcodeproj` que genera, los dos en git).
- [x] CA7 — `scripts/check-rules.mjs` también mira `.css` (comentarios `/* */`, 1.000 líneas), con su test en vitest
  que falla sin la regla; si el lexer de `.svelte` no mira los bloques `<style>`, también.
- [x] CA8 — vitest: (a) cada `var(--x)` usado en `src/` está definido en `tokens.css` (falla con un nombre mal
  escrito); (b) `motion.ts`: duraciones y desplazamientos de entrada/salida, y duración 0 con movimiento reducido;
  (c) `App.test.ts` pasa a comprobar que se pinta la pantalla sin máquinas (`svelte/server`).

## Notas técnicas
- **Sin dependencias nuevas**: transiciones y easing de `svelte/transition` y `svelte/easing` (`cubicOut` es el
  `ease_out` de gpui, `quadIn` su `ease_in`); el SVG de Lucide se copia como hace gpui (`assets/icons/`), con su
  `LICENSE` (ISC) al lado; fuentes del sistema (SF Pro / SF Mono en iOS), sin descargar ni empaquetar fuentes (la
  CSP es `'self'` y la app no tiene red en esta tarea). Archivo y JetBrains Mono, las del sitio, quedan descartadas por
  ahora (peso y licencia); anotarlo en `design/look.md`.
- **Cristal en el móvil**: en gpui el cristal deja ver el escritorio; aquí no hay nada detrás del WebView, así que el
  "fondo" es el ground opaco con el lavado cobalto (como el Linux opaco de gpui, `theme::ground`). A diferencia de
  gpui, WebKit sí difumina el contenido de la propia página bajo un `backdrop-filter`; empezar con los valores de gpui
  (tarjetas al 88–91 %) y ajustarlos a ojo en el simulador, apuntando el porqué. `backdrop-filter` cuesta en
  superficies grandes que se desplazan: solo en tarjetas flotantes, nunca en listas.
- **Lo que se queda fuera a propósito** (llega con su primer uso, para no dejar código muerto): `StatusDot` y la
  órbita de "trabajando" (015), `Spinner`, `Sheet`, la tarjeta de aviso y el punto que late al conectar (014), tonos
  normal y peligro de `Button` y `Chip` (014/015/027). Una galería de componentes solo para verlos se descarta por la
  misma razón.
- `check-rules.mjs` hoy solo mira `.rs .ts .js .mjs .svelte`: los primeros `.css` entran con esta tarea, de ahí CA7.
- `src-tauri/gen/apple` se cambia en `project.yml` y se regenera el `.xcodeproj` (`pnpm tauri ios dev` lo hace).

## Referencias
- cornercase-gpui (`gh api repos/usecornercase/cornercase-gpui/contents/<ruta> -H "Accept: application/vnd.github.raw"`):
  - `src/theme.rs`: `mod radius`, `mod colours`, `ground`.
  - `src/ui/glass.rs`: `card`, `sheen`, `highlight`, `lift`, `glow`, `ring`, `cobalt`, `wash`, `lit`, `tint`.
  - `src/ui/brand.rs`: `mark`, `mark_size`, `wordmark` (rejilla de 12: esquina (3,0) 9×3 y (0,3) 3×9, cursor (6,6) 2×5).
  - `src/ui/motion.rs`: `ENTER`, `LEAVE`, `POP`, `Way`, `ease_out`, `ease_in`, `slide`, `fade`.
  - `src/ui/mod.rs`: `emblem`, `emblem_page`, `emblem_line`; `src/panels/machines.rs` `empty_page`.
  - `assets/icons/server.svg`, `assets/icons/LICENSE`.
  - `design/widgets.md`: "The chrome takes usecornercase.dev's colours", "The whole window is glass in layers",
    "Floating cards stay nearly opaque", "Rounded corners and more air", "Cobalt marks what is chosen, without bars",
    "The mark", "What moves", "Nothing animates forever while idle".
- cornercase-terminal `design/website.md`, "The brand" (la marca, el cobalto, por qué un solo acento).

## design/ que actualiza
- `design/look.md`: qué tokens se portaron y con qué nombres, el fondo y el cristal en el móvil (qué cambia respecto a
  gpui y por qué), los valores ajustados a ojo y con qué versión de iOS del simulador, toque (≥ 44 pt, `:active` en vez
  de hover), movimiento reducido, fuentes del sistema (y por qué no las del sitio), Lucide copiado y no como paquete,
  por qué cada componente llega con su primer uso.
- `design/code-layout.md`: `check-rules.mjs` también mira `.css`.
- `DESIGN.md`: la línea de Look nombra `src/lib/theme/` y `src/lib/ui/`.

## Verificación
`pnpm build`, `pnpm check`, `pnpm lint`, `pnpm test`, `npx -y jscpd@4.3.0` y los checks de cargo verdes. En el
simulador (`pnpm tauri ios dev "iPhone 17"`, en background), `xcrun simctl io booted screenshot <f>.png` muestra la
pantalla sin máquinas: fondo oscuro con el lavado cobalto, la marca, la tarjeta de cristal con el emblema y el botón en
reposo, sin tapar la isla dinámica ni el indicador de inicio, barra de estado clara. La captura se guarda en el
`handoff/` de la tarea y se describe en la PR.
