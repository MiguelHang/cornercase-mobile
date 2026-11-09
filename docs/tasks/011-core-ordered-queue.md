# 011 — Núcleo: cola ordenada (`requests --follow`, `input --follow`)

- Estado: hecha
- Depende de: 007
- Riesgo: medio (orden entre teclas y acciones)
- Rama: `feat/011-core-queue` · PR: `feat: send input and actions through one ordered queue`

## Objetivo
Que las acciones del usuario (teclas, `send`, `focus`, `scroll`, `view-set`…) lleguen al servidor en el orden en que
se hicieron, con una conexión larga en vez de una por acción.

## Criterios de aceptación
- [x] CA1 — `crates/core/src/queue.rs`: un worker que ejecuta trabajos uno tras otro. Comandos por un proceso
  `requests --follow -` (una conexión del puente), una línea `{"args": [...], "stdin": "…"}` por comando, esperando su
  respuesta (`status`, `json`, `text`, `stderr`, `error`) antes del siguiente.
- [x] CA2 — Teclas por `input --follow --json -` (el "typist"), una línea por evento con su `pane`; antes de cualquier
  otro comando se espera a que todas las líneas escritas tengan respuesta (`settle`, plazo 10 s).
- [x] CA3 — Si `requests` o `input --follow` no arrancan o caen, el trabajo va de una vez (`run` normal) y el proceso
  se recrea en el siguiente trabajo; una línea escrita sin respuesta es un error que no se reenvía (puede haber corrido).
- [x] CA4 — `coalesce`: de una racha de trabajos con la misma clave solo va el último; `scroll` del mismo panel se
  suma en un único `scroll --pane P --up/--down N`.
- [x] CA5 — Los comandos con bytes (`paste-file -`) van de una vez por la cola, tras las teclas anteriores (con lo
  que fije D1).
- [x] CA6 — `reconnect()` suelta ambos procesos (servidor reiniciado o ruta nueva); errores hacia un callback
  `on_error` (avisos en la UI).
- [x] CA7 — Tests con `FakeTransport`: orden tecla → acción → tecla, `settle` espera, coalesce, suma de scroll,
  caída del proceso a mitad, respuesta de error que no para la cola.

## Referencias
- cornercase-gpui: `src/cli/requests.rs`, `src/cli/typist.rs`, `design/connection.md` (Input and the actions that
  must follow it go through one ordered queue, con sus "Rejected").
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents/requests.md`,
  `design/commands-for-scripts-and-agents/driving-terminals.md`.

## design/ que actualiza
- `design/connection.md`: la cola, sus dos procesos, qué se coalesce y qué hace al caer.

## Verificación
`cargo test -p cornercase-core` verde, repetido 20 veces sin fallos intermitentes.
