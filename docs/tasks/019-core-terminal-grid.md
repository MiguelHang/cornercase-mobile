# 019 — Terminal en el núcleo: frames, patches y `projection`

- Estado: pendiente
- Depende de: 012 · decisión **D2**
- Riesgo: medio (rendimiento con patches frecuentes)
- Rama: `feat/019-terminal-grid` · PR: `feat: apply surface frames and patches in the core`

## Objetivo
Mantener en Rust la rejilla de una pestaña a partir de `view` y `surface --patches`, y emitir a la WebView solo las
filas que cambian, con estilos ya resueltos.

## Criterios de aceptación
- [ ] CA1 — `crates/core/src/cli/surface.rs` y `view.rs`: tipos serde (`#[serde(default)]`) de frames, patches,
  estilos, enlaces, cursor, `modes`, `scroll`, `colours`, `projection`, `boot`, `base`, `surface`.
- [ ] CA2 — `crates/core/src/terminal/grid.rs`: `Grid::apply_frame`, `Grid::apply_patch -> Applied | NeedRedraw`
  con las reglas de gpui: el patch solo se aplica sobre su `base`/`surface`/`boot`; orden cursor → `active`/`modes`/
  `scroll`/`colours` → `copy` desde una instantánea → filas cambiadas; panel desconocido = `NeedRedraw`.
- [ ] CA3 — Runs resueltos (`Run { text, style, link }`) al llegar; `changed_rows()` para emitir solo lo nuevo.
- [ ] CA4 — `projection`: la rejilla guarda la de su frame; un frame con `projection` menor que la última conocida de
  la vista se descarta.
- [ ] CA5 — Tests con los fixtures de upstream `tests/fixtures/control/surface/v1/frame.json` y `…/patch.json`
  (copiados, con su versión), más casos propios: patch sobre otra base → `NeedRedraw`, `copy` solapado, fila fuera
  de la rejilla, `projection` vieja.
- [ ] CA6 — Microbenchmark simple (test `#[ignore]`) que aplica 1.000 patches y anota el tiempo en `design/terminal.md`.

## Referencias
- cornercase-gpui: `design/session-model.md` (grid, patches, projection, popup), `src/model/surface.rs`,
  `src/model/grid.rs`, `src/app/surface.rs`, `design/terminal.md`.
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents/surfaces.md`, `…/views.md`,
  `tests/fixtures/control/surface/v1/`, `tests/fixtures/control/view/v1/snapshot.json`.
- Orca: `mobile/terminal-output-streaming-findings.md`.

## design/ que actualiza
- `design/terminal.md`: D2 (por qué en Rust), reglas de aplicación, qué se emite a la WebView, medida del benchmark.

## Verificación
`cargo test -p cornercase-core` verde.
