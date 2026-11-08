# 003 — Puente: protocolo y lista permitida

- Estado: pendiente
- Depende de: 001 · decisión **D1** (bytes y EOF) tomada antes de empezar
- Riesgo: medio (la lista permitida es la frontera de seguridad del puente)
- Rama: `feat/003-bridge-protocol` · PR: `feat: define the bridge messages and the commands it allows`

## Objetivo
Lógica pura del puente, sin red ni procesos: los mensajes JSON y la decisión de si un argv se puede ejecutar.

## Criterios de aceptación
- [x] CA1 — `bridge/src/protocol.rs`: tipos serde de los mensajes. Cliente → puente: `{"argv": [...]}` (primero y
  obligatorio), `{"stdin": "<línea>"}` (+ lo que fije D1: `{"stdin_base64": "…"}`, `{"eof": true}`). Puente →
  cliente: `{"stdout": "<línea>"}`, `{"stderr": "<línea>"}`, `{"exit": n}`. Un objeto por mensaje, claves
  desconocidas rechazadas (`deny_unknown_fields`), un mensaje con dos claves rechazado.
- [x] CA2 — `bridge/src/allowlist.rs`: `allowed(argv) -> Result<(), Refusal>`. Permite los comandos `Kind::There` de
  `src/cli/reach.rs` (los subcomandos de control, `stats` y las acciones de `plugin` cuyo `asks_the_server()` es
  verdadero). Rechaza: argv vacío (= TUI), `start-server`, `kill-server`, `update`, `restart`, `logs`, `remote`,
  `skill`, `server`, `proxy`, las acciones de `plugin` que actúan en esta máquina y `plugin windows`, y **cualquier
  `--remote`/`--remote-command`** (también `--remote=x`) en cualquier posición.
- [x] CA3 — `Refusal` da el texto del `stderr` y el código de `exit` (2, como un error de uso del CLI).
- [x] CA4 — Tests con `rstest` y casos con nombre: cada comando del MVP permitido (`contract`, `status --follow
  --json`, `events --json`, `read --last-message`, `send --enter`, `keys`, `paste-file`, `view`, `view-set --claim`,
  `surface --patches`, `input --follow --json -`, `requests --follow -`, `scroll`, `redraw`, `start --worktree`,
  `new-workspace`, `issue list`, `issue read`, `changes list`, `changes viewed`, `files list|read|search`); cada
  excluido rechazado; `--remote` en medio rechazado; protocolo: ida y vuelta de cada mensaje, `argv` que no llega
  primero, JSON inválido.

## Notas técnicas
- La lista es explícita (nombres de subcomando), no "todo menos lo excluido": un comando nuevo de cornercase queda
  fuera hasta que se añada a mano. Copiar los nombres de `Command` en `src/cli/args.rs` de upstream (y los de
  `src/cli/args/plugin.rs` para `plugin`).
- `requests --follow -` se permite: cornercase ya rechaza en sus líneas `update`, `restart`, `kill-server`, `logs`,
  `start-server`, `--remote` y los `plugin` locales (`design/commands-for-scripts-and-agents/requests.md`). Anotarlo.
- El subcomando es el primer argumento que no es una opción global; `--help`/`--version` solos: permitidos (no tocan
  el servidor) o rechazados, decidir y anotar.

## Referencias
- cornercase-terminal (`origin/main`): `src/cli/reach.rs` (`Kind`, `kind`), `src/cli/args.rs` (`Command`),
  `src/cli/args/plugin.rs` (`asks_the_server`), `design/remote-windows.md` (qué va al otro lado y qué no),
  `design/commands-for-scripts-and-agents/requests.md` (qué rechaza una línea de `requests`).

## design/ que actualiza
- `design/bridge.md`: la lista exacta, por qué explícita, por qué `requests` entra, cómo se trata `--remote`, y los
  mensajes de D1.

## Verificación
`cargo test -p cornercase-bridge` verde; clippy limpio.
