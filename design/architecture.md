# Architecture

- **Tauri 2 for iOS and Android**: one Rust core shared by both platforms, a WebView UI. Bundle id `dev.usecornercase.mobile` from the start, so the transfer to `usecornercase` never changes the app's identity.
- **All networking in Rust**: the WebView never opens a socket. It calls Tauri commands and listens to Tauri events whose TypeScript types come from tauri-specta. Rejected: networking in the WebView (iOS suspends it unpredictably, keys would leave the Keychain, and SSH or iroh routes are Rust-only anyway).
- **Workspace**: `crates/core` holds pure logic (no Tauri, no platform) so it is tested on Linux CI; `src-tauri` is the shell (commands, events, key storage, lifecycle); `bridge/` is a separate binary for the Mac.
- **UI**: Svelte 5 + Vite + TypeScript. Chosen for its small bundle, runes and built-in transitions for gpui's motion. Rejected: React (more runtime and re-render care for the live terminal), SolidJS (smaller ecosystem, motion by hand), no framework (routing, state and motion by hand).
- **Platform order**: iOS first (simulator and iPhone); Android gets its own task (Keystore, keyboard).
- **Native projects in git**: `src-tauri/gen/apple` and `gen/android` are versioned, since they carry the Info.plist, the camera permission and the entitlements.
