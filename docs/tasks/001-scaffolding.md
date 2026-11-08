# 001 — Scaffolding: Tauri 2 + Svelte 5 + workspace Cargo + CI

- Estado: hecha
- Depende de: —
- Riesgo: bajo (CI de `src-tauri` en Linux necesita las dependencias de webkit2gtk)
- Rama: `chore/001-scaffolding` · Commit/PR: `chore: scaffold the tauri app, the cargo workspace and ci`

## Objetivo
Dejar un repo que compila, arranca en el simulador de iOS mostrando una pantalla vacía y pasa la CI en Linux, con
la estructura de `CLAUDE.md` (`crates/core`, `src-tauri`, `bridge/`, `src/`).

## Criterios de aceptación
- [x] CA1 — `Cargo.toml` raíz es un workspace con `crates/core` (lib `cornercase-core`), `src-tauri` (app) y
  `bridge` (bin `cornercase-bridge`, además lib para poder testearlo desde otros crates); `resolver = "2"`,
  `[workspace.lints]` con clippy `all = deny`, `pedantic = warn`, `unwrap_used = deny`, `unreachable_pub = warn`.
- [x] CA2 — `rust-toolchain.toml` fija una versión estable concreta (y `rustfmt`, `clippy`), `rustfmt.toml` con
  `max_width = 120`, como cornercase-terminal.
- [x] CA3 — Frontend Svelte 5 + Vite + TypeScript con pnpm (`package.json` con `packageManager`), `pnpm check`
  (svelte-check), `pnpm lint` (ESLint flat config + `eslint-plugin-svelte` + typescript-eslint) y `pnpm test`
  (vitest, con un test trivial que pasa).
- [x] CA4 — `src-tauri/tauri.conf.json`: `identifier = "dev.usecornercase.mobile"`, `productName = "cornercase"`;
  `pnpm tauri ios init` hecho y su `src-tauri/gen/apple` versionado (decisión D4); `pnpm tauri ios dev` abre la app
  en el simulador (comprobado a mano, anotado en la PR).
- [x] CA5 — `.github/workflows/ci.yml` en `ubuntu-latest` por PR y en `main`: `cargo fmt --check`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`,
  `pnpm install --frozen-lockfile`, `pnpm check`, `pnpm lint`, `pnpm test`. Instala `libwebkit2gtk-4.1-dev`,
  `libsoup-3.0-dev`, `libjavascriptcoregtk-4.1-dev` y lo que pida `src-tauri` en Linux; caché con `Swatinem/rust-cache`.
- [x] CA6 — `CLAUDE.md` › Commands rellenado con los comandos reales; `.gitignore` cubre `target/`, `node_modules/`,
  `dist/`, `src-tauri/gen/**/build` y lo que genere Xcode.
- [x] CA7 — `README.md` mínimo (qué es, cómo arrancar, licencia `MIT OR Apache-2.0`) y `Cargo.toml`/`package.json`
  con `license = "MIT OR Apache-2.0"`.

## Notas técnicas
- Partir de `pnpm create tauri-app` (plantilla Svelte + TS) y mover la parte Rust a un workspace. Sin código de
  ejemplo (`greet`): dead code fuera.
- `crates/core` y `bridge` no dependen de Tauri, para que su CI no necesite webkit (aunque el workspace sí).
- Dependabot mensual (cargo, npm, github-actions) en `.github/dependabot.yml`, como cornercase-terminal.
- `cargo-machete` opcional aquí; si se añade, va a la CI.

## Referencias
- cornercase-terminal (`origin/main`): `rust-toolchain.toml`, `rustfmt.toml`, `Cargo.toml` (`[lints]`),
  `.github/workflows/ci.yml`, `.github/dependabot.yml`.
- cornercase-gpui: `Cargo.toml`, `clippy.toml`, `.github/workflows/ci.yml`.
- Orca: `mobile/package.json`, `mobile/vitest.config.ts` (organización de tests en móvil).

## design/ que actualiza
- `design/architecture.md`: versiones de Tauri, Svelte y Vite elegidas; `gen/` versionado y por qué.
- `design/code-layout.md`: qué corre la CI exactamente y las dependencias de sistema que instala.

## Verificación
CI verde en la PR; `pnpm tauri ios dev` arranca en el simulador.
