# 023 — Pantalla 4: arrancar un agente (`start --worktree`, `new-workspace`)

- Estado: en curso (falta la comprobación en el simulador)
- Depende de: 015
- Riesgo: bajo
- Rama: `feat/023-start-agent` · PR: `feat: start an agent in its own worktree from the phone`

## Objetivo
Lanzar trabajo nuevo: un agente con prompt, agente y modelo en un worktree propio, o un workspace nuevo.

## Criterios de aceptación
- [x] CA1 — Formulario (sheet): proyecto (de la sesión), agente (los que cornercase conoce y están instalados en el
  Mac, leídos del CLI; no una lista fija en la app), modelo (opcional, según el agente), prompt multilínea, rama/nombre
  del worktree (opcional). Una pantalla propia, `start`, en vez de la hoja (P1); los agentes, de `account list --check
  --json`, los instalados; el modelo, texto libre, porque cornercase no dice cuáles acepta cada agente
  (usecornercase/cornercase-terminal#704, `design/upstream-gaps.md`).
- [x] CA2 — `start --worktree … --placed --json` por la cola (con `--placed` para no quedar esperando la pregunta de
  confianza de una carpeta); al responder, se abre el chat del panel nuevo (016). Los valores van con `=` para que uno
  que empiece por `-` no sea una opción; la pantalla del agente sustituye a la del formulario y dice `Starting
  <agent>…` hasta que llega su fila (P3).
- [x] CA3 — Acción aparte "workspace nuevo" → `new-workspace` con nombre y, opcional, worktree. Con `--` antes del
  nombre; al responder, toast con el nombre y vuelta al tablero (P4).
- [x] CA4 — Validación antes de mandar (proyecto y prompt obligatorios) y errores del servidor visibles sin perder lo
  escrito. También el agente y el nombre del workspace; el error del servidor, sin su `Error:`, sobre el botón; un
  `Unanswered` dice que puede haber arrancado y no se reintenta.
- [x] CA5 — Tests: argv que construye el núcleo para cada combinación (con y sin modelo, con y sin rama).

## Decisiones
Tomadas el 2026-11-09 con los valores por defecto propuestos (`design/starting-work.md`):
- P1: pantalla `start` empujada en el router, con scroll, y no una hoja desde abajo: el prompt abre siempre el
  teclado y en WKWebView tapa lo fijo abajo (`design/look.md` › Forms).
- P2: agente preseleccionado, el primero instalado en el orden de cornercase y, después, el último arrancado desde la
  app mientras sigue abierta; ni `config --json` ni obligar a elegir.
- P3: se navega en cuanto responde `start --placed`, con `starting` en la ruta `agent`, y la pantalla dice `Starting
  <agent>…` hasta que llega la fila; no se espera en el formulario, porque el setup del proyecto puede tardar minutos.
- P4: tras `new-workspace`, toast `info` con el nombre y vuelta al tablero.
- P5: sin `--focus`: el móvil no mueve la ventana del Mac.

## Referencias
- cornercase-gpui: `src/sidebar/forms.rs`, `src/sidebar/forms/agents.rs`, `design/connection.md` (`start --focus
  --placed` y por qué), `design/worktrees.md`.
- cornercase-terminal (`origin/main`): `src/cli/args.rs` (`StartArgs`, `NewWorkspaceArgs`),
  `design/commands-for-scripts-and-agents/worktrees.md`, `design/agents.md`.

## design/ que actualiza
- Nuevo `design/starting-work.md` (y su línea en `DESIGN.md`): de dónde salen agentes y modelos, `--placed`.
- `design/connection.md` (`account-list` en `REQUIRED`), `design/architecture.md` (comandos de `commands/start.rs`,
  `ProjectChoice`, la pantalla `start`), `design/look.md` (`replace`, `Start` en el tablero, `TextArea` con etiqueta y
  error, la pantalla y no la hoja, los toasts `info`), `design/chat.md` (el agente que arranca), `design/board.md`
  (`Start`) y `design/upstream-gaps.md` (modelos por agente).

## Verificación
En el simulador se arranca un agente en un worktree y aparece en el tablero trabajando.
