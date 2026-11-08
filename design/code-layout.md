# Code layout

- **No comments**, in Rust or TypeScript: the why lives here, in `design/`.
- **At most 1,000 lines per file**; small functions with one purpose.
- **`pub(crate)` by default** in Rust; `pub` only across crates of the workspace.
- **Licence**: `MIT OR Apache-2.0`, as cornercase-terminal.

## CI

- **One job** in `.github/workflows/ci.yml`, on `ubuntu-24.04`, for every pull request, every push to `main` and by hand. A new push to the same branch cancels the run in progress. No iOS or Android builds in CI (macOS runners are expensive); those are checked locally.
- **Steps, in order**:
  1. `rustup toolchain install`, which reads `rust-toolchain.toml`.
  2. `apt-get install` of `libwebkit2gtk-4.1-dev`, `libsoup-3.0-dev`, `libjavascriptcoregtk-4.1-dev`, `libssl-dev`, `libxdo-dev`, `libayatana-appindicator3-dev` and `librsvg2-dev`: Tauri 2's prerequisites for Debian and Ubuntu.
  3. `Swatinem/rust-cache` for `target/` and cargo's registry.
  4. pnpm, at the version of `packageManager`, and Node 22 with pnpm's store cached.
  5. `pnpm install --frozen-lockfile`, then `pnpm build`.
  6. `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`.
  7. `pnpm check` (svelte-check on `src/`, then `tsc -p tsconfig.node.json` on `vite.config.ts`, which svelte-check leaves out), `pnpm lint`, `pnpm test`.
- **Clippy and the tests cover the whole workspace**, `src-tauri` included, built for Linux desktop. That is why the job installs WebKitGTK and its friends. Rejected: excluding `src-tauri` to skip them, which would leave the shell unchecked.
- **`pnpm build` runs before cargo**: `tauri::generate_context!` embeds `dist/` (`frontendDist`) when Tauri builds with `custom-protocol`, as `tauri build` and `tauri ios build` do, and fails without it. A dev build, which is what clippy and the tests compile, does not need it (checked on 2026-11-08 with Tauri 2.12.5). The step still goes first, so that CI proves the frontend bundles and cargo stays green if a check ever enables `custom-protocol`.
- **The Rust cache is saved from `main` only**, as in cornercase-terminal: pull requests restore `main`'s cache and do not fill the repository's quota with caches no other branch can read. The first pull request builds everything from scratch.
- **pnpm 12 refuses a package published less than a day ago** (`ERR_PNPM_MINIMUM_RELEASE_AGE_VIOLATION`), even with `--frozen-lockfile`. A Dependabot pull request that bumps something that new fails until the day has passed; rerun the job then.
- **Checked live**: the steps from `pnpm install` on, from a clean tree on macOS, on 2026-11-08. The apt packages are only checked by the job itself.
- **Dependabot** (`.github/dependabot.yml`): cargo, npm and GitHub Actions, monthly, one grouped pull request per ecosystem, titled `chore(deps): …`. It ignores TypeScript majors (see Architecture).
