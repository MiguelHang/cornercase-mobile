# 010 — Núcleo: `contract` y `boot`

- Estado: pendiente
- Depende de: 007 · decisión **D5**
- Riesgo: bajo
- Rama: `feat/010-core-contract` · PR: `feat: check the server's contract and notice restarts by boot`

## Objetivo
Al conectar, leer `cornercase contract --json`, decir qué comandos del MVP faltan en el Mac en vez de fallar a ciegas
y detectar reinicios del servidor por `boot`.

## Criterios de aceptación
- [ ] CA1 — `crates/core/src/cli/contract.rs`: tipos serde de `Descriptor` (`version`, `contract`, `boot`,
  `operations[{name, available, reason}]`, `features`, `streams`), todos `#[serde(default)]`.
- [ ] CA2 — `REQUIRED`: lista de operaciones y features que usa el MVP (las de las pantallas 015–027, con `requests`,
  `input`, `surface-patches`, `views`, `last-message`, `changes`, `issues`…). `missing(descriptor) -> Vec<Missing>`
  con nombre y motivo (`reason` del servidor si `available = false`).
- [ ] CA3 — Un servidor sin `contract` (el CLI dice `has no \`contract\` command`) o con `contract` mayor que el que
  entiende la app da un error explicativo ("actualiza cornercase en el Mac").
- [ ] CA4 — `Boot` como tipo; `BootWatch::observe(boot) -> Same | Restarted`.
- [ ] CA5 — Fixtures en `crates/core/tests/fixtures/` sacados del cornercase real (`contract --json` de upstream main)
  y uno recortado sin operaciones; tests de `missing` y `observe`.

## Notas técnicas
- `contract` no lleva `--boot`; el resto de comandos que lleven ids podrán llevarlo más adelante (`stale-boot` sale
  con código 4: mapearlo a `Error::StaleBoot` ya aquí).
- Indicar en el fixture de qué versión de cornercase sale.

## Referencias
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents/contract.md`, `src/control/contract.rs`,
  `src/cli/contract.rs`.
- cornercase-gpui: `design/connection.md` (versión mínima y por qué), `src/app/connection.rs`.

## design/ que actualiza
- `design/connection.md`: lista `REQUIRED` y por qué cada una; qué pasa con un servidor viejo.
- `design/upstream-gaps.md` si algo del MVP no tiene operación o feature con nombre.

## Verificación
`cargo test -p cornercase-core` verde.
