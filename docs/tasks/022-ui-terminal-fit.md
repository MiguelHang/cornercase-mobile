# 022 — Pantalla 3: "ajustar a este móvil" (`view-set --claim`)

- Estado: hecha, salvo la prueba en vivo (simulador: `controller` en `cornercase status`; iPhone: el segundo plano real)
- Depende de: 021
- Riesgo: bajo
- Rama: `feat/022-terminal-fit` · PR: `feat: let the phone claim a tab's size while it is open`

## Objetivo
Un botón que reclama la pestaña para que el PTY tome el ancho del móvil mientras esa pantalla está abierta, y la
suelta al salir.

## Criterios de aceptación
- [x] CA1 — Botón "ajustar a este móvil" (conmutador) → abre una `view --tab T --cols C --rows R` (activa: reclama
  la pestaña al abrirse) y cambia el stream de `surface --tab` (020, pasivo, sin `view`) a `surface --view ID
  --patches`; `view-set --view ID --claim` por la cola solo para volver a reclamar. Un rechazo (no elegible) se
  muestra con su motivo. **Desviación**: sin `view-set`; una `view` nueva ya reclama al abrirse, así que
  `--claim` no tenía disparador (`design/terminal.md` › Fit to this phone). El botón usa el icono `smartphone` de
  Lucide (`aria-label="Fit to this phone"`, `aria-pressed`) y el rechazo es un toast de error con el texto de
  cornercase.
- [x] CA2 — Al salir de la pantalla, al ir a segundo plano o al apagar el conmutador → se cierra la `view` (o
  `view-set --view ID --inactive`) y, si la pantalla sigue abierta, se vuelve al `surface --tab` pasivo de 020. Al
  volver a primer plano con la pantalla abierta y el conmutador encendido, se vuelve a reclamar. Se cierra
  dejando caer el proceso, nunca con `view-set`; el segundo plano se detecta con `visibilitychange`, no con
  `WindowEvent::Suspended`. Un tamaño nuevo con el ajustado encendido reabre la `view` (rebote breve en el Mac).
- [x] CA3 — Los frames se filtran por `projection` (019): no se pinta un frame viejo de antes del cambio de tamaño.
  Además, nada se pinta hasta que la primera línea de la `view` muestra la pestaña, y un frame de otra `view` u
  otra pestaña no se aplica.
- [x] CA4 — Si el Mac retoma la pestaña (se usa su ventana), el conmutador se apaga solo y se dice: `controls:
  false` → toast `info` "The Mac took this tab back" y vuelta al pasivo. Si la `view` se asienta en otra pestaña
  (la pestaña cerró), "This tab was closed on the Mac", sin el toast.
- [x] CA5 — Tests del núcleo: la secuencia de comandos al abrir/cerrar/segundo plano con el conmutador encendido.
  Más tests de Tauri (`Batch` con `unfitted`, reabrir con otro modo) y vitest del conmutador y de `Opening`.

## Referencias
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents/views.md` (claims, eligible,
  `--inactive`, la ventana reclama al usarse).
- `design/terminal.md` de este repo (Passive by default, Fit to this phone): 020 no abre ninguna `view`, porque una
  activa con tamaño reclama al abrirse; la `view` nace aquí.

## design/ que actualiza
- `design/terminal.md`: cuándo se suelta, qué pasa al ir a segundo plano, cómo se detecta que el Mac la retomó.

## Verificación
En iPhone: con el conmutador se lee al ancho del móvil; al salir, la ventana del Mac vuelve a su tamaño
(criterio de éxito 3, parte "ajustar").
