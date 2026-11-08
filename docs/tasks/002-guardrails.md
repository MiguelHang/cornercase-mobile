# 002 — Guardarraíles: límites de clippy, 1.000 líneas por fichero, sin comentarios

- Estado: pendiente
- Depende de: 001
- Riesgo: bajo
- Rama: `chore/002-guardrails` · PR: `chore: enforce the code rules in ci`

## Objetivo
Que las reglas de `CLAUDE.md` (sin comentarios, ≤ 1.000 líneas, funciones pequeñas, `pub(crate)`) las compruebe la
CI y no la memoria de quien revisa.

## Criterios de aceptación
- [ ] CA1 — `clippy.toml` con los límites de cornercase-terminal: `too-many-lines-threshold = 40`,
  `too-many-arguments-threshold = 5`, `excessive-nesting-threshold = 4`, `cognitive-complexity-threshold = 10`, y
  `cognitive_complexity`/`excessive_nesting`/`too_many_lines` activados en `[workspace.lints.clippy]`.
- [ ] CA2 — Un script `scripts/check-rules.mjs` (Node, sin dependencias) que falla si un fichero de `crates/`,
  `src-tauri/src/`, `bridge/` o `src/` pasa de 1.000 líneas o contiene un comentario (`//`, `/* */`, `///`, `//!`
  en Rust y TS; `<!-- -->` en `.svelte`), saltando cadenas y URLs. Excepciones en una lista dentro del script que
  solo puede encoger (vacía hoy). Corre en `pnpm lint` y por tanto en la CI.
- [ ] CA3 — Tests de vitest para el script: un fichero con comentario falla, una cadena con `//` no, un fichero de
  1.001 líneas falla. Cada test falla sin la regla que protege.
- [ ] CA4 — ESLint: `max-lines-per-function` (40), `complexity` (10), `max-depth` (3) en TS y Svelte.

## Notas técnicas
- Ficheros generados (bindings de tauri-specta, `gen/`) excluidos por ruta en el script.
- jscpd como en cornercase-terminal es opcional; si se añade, `.jscpd.json` con umbral 1 % y a la CI.

## Referencias
- cornercase-terminal: `clippy.toml`, `Cargo.toml` `[lints]`, `tests/layers.rs` (comprobación de tamaños y lista
  de excepciones que solo encoge), `.jscpd.json`.
- cornercase-gpui: `clippy.toml`, `.jscpd.json`.

## design/ que actualiza
- `design/code-layout.md`: los límites, qué comprueba `check-rules.mjs` y por qué un script y no un plugin de ESLint.

## Verificación
`pnpm lint` y `cargo clippy` verdes en la CI; un comentario añadido a propósito hace fallar la CI (comprobado en local).
