# cornercase-mobile

> iOS and Android app (Tauri 2) to follow, answer and start cornercase agents from a phone. It talks to
> the cornercase server only through its command line, the public contract, like cornercase-gpui.

## Context
- Idea and requirements: `docs/IDEA.md`
- Roadmap and tasks: `docs/ROADMAP.md`, `docs/tasks/`
- Design decisions: `DESIGN.md` (index) and `design/`, one file per area. Read the area's file before
  changing it; record what was decided and why (and what was rejected) in the same PR.
- References to follow, not to copy blindly:
  - `usecornercase/cornercase-gpui`: `design/widgets.md` + `src/theme.rs` (look), `design/connection.md`
    (connection), `design/session-model.md` (terminal JSON, frames and patches).
  - `usecornercase/cornercase-terminal` on upstream `main` (the local `~/me/cornercase-terminal` may lag):
    `src/cli/reach.rs`, `design/commands-for-scripts-and-agents.md` and its topics, `design/remote-windows.md`,
    `design/browser-access.md`.
  - `pingdotgg/t3code` (`docs/user/remote-access.md`), `stablyai/orca` (`mobile/`).

## Stack
- Tauri 2 (iOS first, then Android), Rust stable, Cargo workspace.
- Svelte 5 + Vite + TypeScript in the WebView, pnpm.
- tokio, tokio-tungstenite, ed25519-dalek, tauri-specta (TypeScript types from Rust).
- No network code in the WebView: everything goes through Tauri commands and events.

## Commands
```sh
pnpm install                                      # frontend packages and the Tauri CLI (pnpm from packageManager)
pnpm build                                        # dist/: release builds need it; CI runs it before cargo
pnpm tauri ios dev "<iPhone>"                     # the app in the iOS simulator; names from `xcrun simctl list devices`
pnpm tauri android dev [<AVD or device>]          # the app on Android; names from `emulator -list-avds`, `adb devices`
cargo run -p cornercase-bridge                    # the development bridge, on the Mac's LAN and Tailscale addresses
cargo run -p cornercase-bridge -- --loopback --port 0   # the same, only on 127.0.0.1, to try it on the Mac
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
pnpm check                                        # svelte-check, then tsc on vite.config.ts
pnpm lint                                         # eslint, then scripts/check-rules.mjs (1,000 lines, comments)
npx -y jscpd@4.3.0                                # copy-paste detector, reads .jscpd.json
pnpm test                                         # vitest
```

- **The toolchain is pinned** in `rust-toolchain.toml`, with the two iOS targets and `aarch64-linux-android`; rustup
  installs it on the first cargo command. Node 22.
- **CI runs `pnpm install --frozen-lockfile`, `pnpm build` and the checks from `cargo fmt --check` down, in that
  order** (`design/code-layout.md`). Before calling a task done, run them and fix what fails.
- **iOS needs the full Xcode**, not the Command Line Tools: `sudo xcode-select -s /Applications/Xcode.app`,
  `sudo xcodebuild -license accept` (until then `git` and `clang` refuse to run), open Xcode once and download an iOS
  simulator runtime. With Homebrew, the Tauri CLI installs the Apple tools it is missing on its own (xcodegen and
  libimobiledevice on `ios init`, ios-deploy on `ios dev`); CocoaPods is `brew install cocoapods`.
- **`src-tauri/gen/apple` is in git** (`design/architecture.md`): change the Xcode project in its `project.yml`,
  which the Tauri CLI turns into the `.xcodeproj`, and commit both; `.gitignore` leaves out what the builds write.
- **Android needs JDK 17, the SDK and NDK 29.0.14206865**, without Android Studio and without sudo
  (`design/architecture/android.md`):
  - `brew install openjdk@17`, `brew install --cask android-commandlinetools`, then in the shell's profile
    `JAVA_HOME=/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home`,
    `ANDROID_HOME=/opt/homebrew/share/android-commandlinetools`, `NDK_HOME=$ANDROID_HOME/ndk/29.0.14206865`, and
    `$JAVA_HOME/bin`, `$ANDROID_HOME/platform-tools` and `$ANDROID_HOME/emulator` on the `PATH`.
  - `yes | sdkmanager --licenses`, then `sdkmanager platform-tools "platforms;android-37.0" "build-tools;37.0.0"
    "ndk;29.0.14206865" emulator "system-images;android-37.0;google_apis;arm64-v8a"`. Gradle downloads the rest
    (platform 36) on the first build, which takes a few minutes.
  - An emulator: `avdmanager create avd -n cornercase_api37 -d pixel_8 -k
    "system-images;android-37.0;google_apis;arm64-v8a"`, started with `emulator -avd cornercase_api37`. A phone needs
    USB debugging on.
  - rustup brings `aarch64-linux-android` with the toolchain; a release for every ABI also needs
    `rustup target add armv7-linux-androideabi i686-linux-android x86_64-linux-android`.
  - Clippy for the phone, `cargo clippy -p cornercase-mobile --target aarch64-linux-android --all-targets --locked --
    -D warnings`, needs the NDK's compiler: with `T=$NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin`, set
    `CC_aarch64_linux_android=$T/aarch64-linux-android24-clang`, `AR_aarch64_linux_android=$T/llvm-ar` and
    `CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$T/aarch64-linux-android24-clang`.
  - **The emulator reaches the bridge** at the Mac's LAN or Tailscale address the bridge prints, not at `10.0.2.2`
    (`--loopback` prints `127.0.0.1`, which is the emulator itself). Paste the code into the form, or type it with
    `adb shell "input text '<piece>'"`, 16 characters at a time: longer text gets cut.
- **`src-tauri/gen/android` is in git** too, as `android init` wrote it; only the manifest, the two files of
  `res/xml/` and `MainActivity.kt` (the keyboard's insets) are changed by hand. `minSdk` and the debug suffix live in `tauri.conf.json` › `bundle.android`.

## Structure
- `crates/core/`: pure, testable logic: transport trait, bridge protocol, CLI types over `--json`,
  ordered queue, `Store`, machines and routes. No Tauri, no platform code.
- `src-tauri/`: the app shell: Tauri commands and events, key storage (Keychain / Keystore), lifecycle
  and network-change hooks, mobile projects.
- `bridge/`: the development bridge binary (WebSocket ↔ `cornercase <args>`), runs on the Mac.
- `src/`: the Svelte UI.
- `design/`, `DESIGN.md`: decisions by area. `docs/`: idea, roadmap and tasks.

## Code rules
- No comments, in Rust, TypeScript or Svelte, directives such as `eslint-disable` included. Names and small functions
  say what the code does; the why goes to `design/`. `scripts/check-rules.mjs` checks it.
- At most 1,000 lines per file (`scripts/check-rules.mjs`). Small functions with one purpose: at most 40 lines,
  complexity 10 and three levels of nesting, checked by clippy (`clippy.toml`, also 5 arguments) and ESLint; the two
  count differently (`design/code-layout.md`). Split the function; never silence these rules, and never add a file to
  the script's lists of exceptions, which only shrink.
- No copied code: jscpd fails above 1 % duplication. Extract the shared code; do not raise the threshold.
- Rust: `pub(crate)` by default; `pub` only for what another crate of the workspace uses (`unreachable_pub`).
- The CLI is the only contract with cornercase: never read its files, sockets or internal state.
- No workarounds for what cornercase lacks: note the gap in `design/upstream-gaps.md`.
- No new dependency without a line in the area's `design/` file saying why.
- No dead code.

## Tests
- `crates/core` and `bridge/` carry the logic and its unit tests (protocol, allowlist, queue, Store,
  applying frames and patches, route selection), with JSON fixtures from cornercase's `--json` output.
- UI logic that is not markup (stores, formatting, key mapping) has vitest tests.
- Every task closes with tests green and fmt, clippy, svelte-check, lint and jscpd clean.

## Workflow
- One task in `docs/tasks/` = one branch = one PR.
- Commits: Conventional Commits.
- This repo uses the harness (`/hx-start`, `/hx-build`, `/hx-review`, `/hx-ship`).
