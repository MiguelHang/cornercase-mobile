# 052 — Empezar: elegir cuenta (o perfil) al arrancar un agente o un flow

- Estado: hecha (comprobada en el iPhone del usuario el 2026-11-14)
- Depende de: 045, 047
- Rama: `feat/052-start-account` · PR: `feat: choose the account an agent or a flow starts with`

## Objetivo
Del iPhone del usuario (2026-11-12): "al arrancar un agente o un flow que deje elegir cuenta si las tiene configuradas".

Upstream ya lo permite: `start` y `flow run` llevan `ProfileArg` (`src/cli/args/accounts.rs`, leído en `main`, 0.13.46):
`--profile NAME` (las cuentas de un perfil; vacío = el login propio de cada agente) y `--account AGENT=ACCOUNT` (una
cuenta para ese agente sobre las de por defecto; `AGENT=` = su login propio). La app ya llama a `account list --check
--json` para los agentes instalados (`design/starting-work.md`) y descarta el resto de la respuesta.

## Criterios de aceptación
- [x] CA1 — En el formulario de agente (Start, desde un agente y desde una issue), si el agente elegido tiene cuentas
  configuradas, un selector `Account` con sus cuentas (nombre que muestra cornercase), su login propio y la de por
  defecto preseleccionada (la que usaría cornercase sin opción). Se manda `--account AGENT=ACCOUNT` solo si se cambia
  la de por defecto. Sin cuentas configuradas, no sale nada.
- [x] CA2 — En el formulario de flow (047), igual: si hay perfiles (`profile list`) o cuentas, elegir con qué arranca el
  equipo; decidir con lo que dé upstream si es `--profile` o una `--account` por agente del flow, y anotarlo en
  `design/starting-work.md` (perfiles fuera, P1).
- [x] CA3 — Cambiar de agente actualiza las cuentas; una cuenta que falla su check (`account list --check`) se ve como
  tal y se puede elegir igualmente o no, según lo que diga upstream de qué pasa al arrancar con ella.
- [x] CA4 — Servidor sin `ProfileArg` en `start`/`flow run` (más viejo): no se ofrece el selector (decidir cómo se
  detecta sin romper; si no se puede, anotarlo en `design/upstream-gaps.md`).
- [x] CA5 — Fixtures reales de `account list --check --json` (y `profile list --json` si se usa); tests del argv en
  `crates/core`; comprobado en el simulador; `design/starting-work.md` actualizado; `hx check full` verde.

## Decisiones
Tomadas el 2026-11-12 con los valores por defecto propuestos, sin respuesta del usuario (`design/starting-work.md` ›
Where the choices come from, Starting an agent y With a flow):
- Las cuentas salen de `providers` del mismo `account list --check --json` que ya da los agentes: ninguna llamada nueva,
  se cargan, fallan y se releen con los agentes. `profile list` no se lee.
- Por defecto, `active` (vacío o desconocido = login propio), la selección efectiva de cornercase: el puente quita
  `CORNERCASE_PANE`, así que no hay cuentas de un panel que llama. `--account=<agent>=<cuenta>` solo si la elección
  difiere; sin tocar nada, el argv es el de antes. Login propio sobre una cuenta por defecto: `--account=<agent>=`.
- La elección se guarda por agente mientras vive el formulario, compartida entre `Agent` y `Flow`; un reinicio del
  servidor la vacía. No se recuerda la última cuenta entre formularios.
- P1: en el flow, una cuenta por agente del equipo (`<agent> account`), sin `--profile`, como el diálogo de flow del Mac.
- P2: toda cuenta se puede elegir; `signed-out` y `failed` se marcan en la opción, y una `signed-out` elegida lleva una
  nota bajo el selector.
- CA4: sin detección en tiempo de ejecución. `accounts` (0.12.154) y `account-switch` (0.12.159) entran en
  `contract::REQUIRED`; como `files-search` (0.12.215) ya se exige, no deja fuera a ningún servidor aceptado, y no hace
  falta línea en `design/upstream-gaps.md`.
- Comprobado en el simulador iPhone Air contra cornercase 0.13.31 en un servidor aislado con cuentas.

## design/ que actualiza
- `design/starting-work.md`: Where the choices come from, Starting an agent, With a flow, Checked live y Left out.
- `DESIGN.md` (la línea de Starting work), `design/connection.md` (`REQUIRED`), `design/architecture.md`
  (`installed_agents` y los tipos de specta) y `design/look.md` (`AccountPicker`).
