# Code layout

- **No comments**, in Rust or TypeScript: the why lives here, in `design/`.
- **At most 1,000 lines per file**; small functions with one purpose.
- **`pub(crate)` by default** in Rust; `pub` only across crates of the workspace.
- **CI**: GitHub Actions on Linux for every pull request: `cargo fmt --check`, clippy with warnings as errors and tests for `crates/core` and `bridge/`; svelte-check, lint and vitest. No iOS or Android builds in CI (macOS runners are expensive); those are checked locally.
- **Licence**: `MIT OR Apache-2.0`, as cornercase-terminal.
