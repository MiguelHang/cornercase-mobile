# 004 — Puente: ejecutar `cornercase` línea a línea y matar su grupo

- Estado: hecha
- Depende de: 003
- Riesgo: medio (procesos huérfanos si el grupo no se mata bien)
- Rama: `feat/004-bridge-process` · PR: `feat: run an allowed cornercase command and stream its lines`

## Objetivo
Dado un argv permitido, lanzar `cornercase <argv>` y exponer su stdin/stdout/stderr como streams de líneas y su
código de salida, matando el proceso y su grupo cuando el que lo pidió se va.

## Criterios de aceptación
- [x] CA1 — `bridge/src/process.rs`: `spawn(argv) -> Running` con tokio: stdout y stderr leídos línea a línea por
  separado (buffer grande: un frame de `surface` puede ser una línea de cientos de KiB), stdin escrito línea a línea,
  EOF según D1, y `exit` con el código (señal → código distinto de 0 documentado).
- [x] CA2 — El hijo arranca en su propio grupo de procesos (`process_group(0)`); al soltar `Running` (o al llamar a
  `kill`) se manda `SIGTERM` al grupo y `SIGKILL` tras un plazo corto; nunca queda un zombi (`wait`).
- [x] CA3 — Entorno del hijo sin `CORNERCASE_PANE`, `CORNERCASE_SERVER` ni `CORNERCASE_REMOTE` (si el puente corre
  dentro de una pestaña de cornercase, los comandos no deben apuntar a esa pestaña ni a otra máquina).
- [x] CA4 — El binario se elige así: `--cornercase <ruta>` del puente, si no `CORNERCASE_BIN`, si no `cornercase` en
  el `PATH`; si no se encuentra, el puente no arranca y lo dice.
- [x] CA5 — Tests con un `cornercase` falso (script `/bin/sh` en un `TempDir`) que: hace eco de stdin, escribe en
  stderr, sale con 3, se queda colgado lanzando un `sleep` hijo (al soltar, el `sleep` también muere: se comprueba su
  pid), y muestra que las variables `CORNERCASE_*` no le llegan. Esperas con sondeo y plazo, nunca `sleep` fijo.

## Notas técnicas
- Usar `tokio::process::Command` + `nix`/`libc` solo para `killpg`; justificar la dependencia en `design/bridge.md`.
- No interpretar la salida: el puente es un tubo.

## Referencias
- cornercase-gpui: `src/cli.rs` (`Stream` que mata y recoge al soltar, `kill_all`, buffer de 1 MiB, `CALLER`),
  `design/connection.md` (Commands run without the caller's variables; Streams stop when dropped).
- cornercase-terminal: `src/test_util.rs` (`write_executable`, `wait_until`, `Family` para árboles de procesos).

## design/ que actualiza
- `design/bridge.md`: grupo de procesos, plazo de `SIGKILL`, variables eliminadas, cómo se encuentra el binario.

## Verificación
`cargo test -p cornercase-bridge` verde, también repitiéndolo 20 veces sin fallos intermitentes.
