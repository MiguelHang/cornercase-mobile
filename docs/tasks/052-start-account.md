# 052 — Empezar: elegir cuenta (o perfil) al arrancar un agente o un flow

- Estado: pendiente
- Depende de: 045, 047
- Rama: `feat/052-start-account` · PR: `feat: choose the account an agent or a flow starts with`

## Objetivo
Del iPhone del usuario (2026-11-12): "al arrancar un agente o un flow que deje elegir cuenta si las tiene configuradas".

Upstream ya lo permite: `start` y `flow run` llevan `ProfileArg` (`src/cli/args/accounts.rs`, leído en `main`, 0.13.46):
`--profile NAME` (las cuentas de un perfil; vacío = el login propio de cada agente) y `--account AGENT=ACCOUNT` (una
cuenta para ese agente sobre las de por defecto; `AGENT=` = su login propio). La app ya llama a `account list --check
--json` para los agentes instalados (`design/starting-work.md`) y descarta el resto de la respuesta.

## Criterios de aceptación
- [ ] CA1 — En el formulario de agente (Start, desde un agente y desde una issue), si el agente elegido tiene cuentas
  configuradas, un selector `Account` con sus cuentas (nombre que muestra cornercase), su login propio y la de por
  defecto preseleccionada (la que usaría cornercase sin opción). Se manda `--account AGENT=ACCOUNT` solo si se cambia
  la de por defecto. Sin cuentas configuradas, no sale nada.
- [ ] CA2 — En el formulario de flow (047), igual: si hay perfiles (`profile list`) o cuentas, elegir con qué arranca el
  equipo; decidir con lo que dé upstream si es `--profile` o una `--account` por agente del flow, y anotarlo en
  `design/starting-work.md`.
- [ ] CA3 — Cambiar de agente actualiza las cuentas; una cuenta que falla su check (`account list --check`) se ve como
  tal y se puede elegir igualmente o no, según lo que diga upstream de qué pasa al arrancar con ella.
- [ ] CA4 — Servidor sin `ProfileArg` en `start`/`flow run` (más viejo): no se ofrece el selector (decidir cómo se
  detecta sin romper; si no se puede, anotarlo en `design/upstream-gaps.md`).
- [ ] CA5 — Fixtures reales de `account list --check --json` (y `profile list --json` si se usa); tests del argv en
  `crates/core`; comprobado en el simulador; `design/starting-work.md` actualizado; `hx check full` verde.
