# cornercase-mobile

The phone app for [cornercase](https://github.com/usecornercase/cornercase-terminal): follow, answer and start
cornercase's coding agents from an iPhone (Android later). It is a Tauri 2 app, with a Rust core and a Svelte UI, and
it talks to the cornercase server only through its command line.

Early days: the app opens an empty screen.

## Prerequisites

- Rust through [rustup](https://rustup.rs). `rust-toolchain.toml` pins the version and the iOS targets, and rustup
  installs them on the first cargo command.
- Node 22 and pnpm. `corepack enable` gives the pnpm version that `package.json` names in `packageManager`.
- For iOS, the full Xcode (the Command Line Tools are not enough):
  - `sudo xcode-select -s /Applications/Xcode.app`
  - `sudo xcodebuild -license accept`
  - open Xcode once and download an iOS simulator runtime
  - Homebrew and CocoaPods (`brew install cocoapods`); the Tauri CLI installs the other Apple tools it needs (xcodegen,
    libimobiledevice, ios-deploy) with Homebrew on its own

## Getting started

```sh
pnpm install
pnpm tauri ios dev "<iPhone>"      # a simulator's name, from `xcrun simctl list devices`
```

The checks that CI runs, and the other commands, are in [CLAUDE.md](CLAUDE.md#commands).

## Licence

Licensed under either of [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
