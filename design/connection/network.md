# Network

A topic of [Connection](../connection.md). The app notices when the phone changes network and tells the active machine's `Store`, which then picks a route again within a second or two. Without this it would wait for the backoff (up to 15 s), or for the transport's 45 s of silence on a half-open socket.

- **The path of a change**: the Swift plugin hears a new `NWPath` and sends `{kind, expensive}` through a channel to the plugin's Rust, which calls the app's callback. A task in the app reads those paths. The core's `Debounce` turns them into a `Decision`, which goes to `AppState` and to the active `Store`.

## The iOS plugin

- **A Tauri plugin of our own in Swift** (`src-tauri/plugins/network/`, a crate of the workspace). The decision to go native, and the alternative rejected, are in `design/architecture.md`.
  - **What it sends**: `NetworkPlugin.swift` starts an `NWPathMonitor` on a queue of its own. Each path that differs from the last one sent goes out as `{kind, expensive}`, the first path the monitor gives included.
  - **`kind`**: `none` when the path is not `satisfied`; otherwise the first interface type the path uses, checked as `wifi`, then `cellular`, then `other` (wired, loopback or a VPN alone).
  - **`expensive`** is the path's `isExpensive`.
  - **Only Rust talks to it**: the crate is laid out like `tauri plugin new --ios`, but with no JavaScript API, no commands and no permissions.
  - **Generated files stay out of git**: `build.rs` declares no commands, so the permissions `tauri-plugin` writes are ignored (`.gitignore`). So are `.tauri/`, where the build copies Tauri's Swift package, and `ios/.build`.
- **A channel made in Rust**: `ios.rs` registers the plugin in its `setup`. It builds a `tauri::ipc::Channel` with `Channel::new`, whose handler reads the path and calls the app's callback, and hands it to Swift's `watch` with `run_mobile_plugin`.
  - Rejected: Swift's `trigger`. Its events reach only listeners that the WebView registers with `addPluginListener`, so every network change would go through the WebView and back.
- **The callback holds no logic**: `init(on_path)` takes an `Fn(Network)`. The app's callback (`network::listen`) puts each path on an unbounded channel, read by a task on Tauri's runtime that `setup` starts once `AppState` is managed. Swift calls the callback on the monitor's queue, so nothing slow runs there.
- **Other platforms build a stub** (`other.rs`), which registers the plugin and sends nothing.
  - Linux CI therefore compiles and tests the crate and the app.
  - With no paths, the app never thinks the network is down.
  - Android's `ConnectivityManager` (032) becomes an `android.rs` behind the same callback, and reuses the debounce and the decisions.
- **Swift is checked only on a Mac**: CI runs on Linux and never compiles it. Two local checks cover `ios.rs` and the package:
  - `cargo clippy -p tauri-plugin-network --target aarch64-apple-ios-sim --all-targets`, and the same command for `cornercase-mobile`;
  - a build for the simulator.
  - Neither `scripts/check-rules.mjs` nor jscpd reads Swift, so its rule against comments is kept by hand (`design/code-layout.md`).
  - `import Network` links the framework on its own, so `project.yml` is unchanged.
  - The package asks for swift-tools 5.9 and iOS 15, the app's minimum.

## The debounce and the decisions

- **1 s of calm, the last value wins** (`crates/core/src/network.rs`, `Debounce`). A transition sends a burst of paths, often through `none`, with interfaces appearing one after another; the burst gives one decision.
  - **Why 1 s**: a false change costs a reconnection of about a second, and a longer wait adds straight to the time to reconnect.
  - The wait is a parameter of `Debounce::new`, and 1 s its default.
  - **Pure logic in the core**: like the queue, it has no runtime of its own and the caller polls it. It is tested on a paused clock, and Android can reuse it.
- **The baseline**: the first value that settles is not a change, since the `Store` is already connecting. It is `Decision::Baseline`, which the app only logs and keeps. A baseline of `none`, when the app starts without a network, is `Offline`.
- **After the baseline**:
  - A settled `none` is `Offline`.
  - Any other settled value is `Changed`, even when the burst went through `none`. So wifi → `none` → cellular within a second is one reconnection, and "No network" never shows.
  - That includes a value equal to the one applied, since `{kind, expensive}` is not the whole path: Swift sends only paths that differ, so each one that settles is a new network (see Tailscale, below). Wifi → `none` → wifi within a second, from one wifi to another, is one reconnection too.
  - A change of `expensive` on the same interface counts as a change.
  - Only a `none` after a `none` gives nothing.
  - Rejected: comparing with the value applied and giving nothing when it is equal. Entering Tailscale on cellular while `Retrying` waited for the backoff, up to 15 s, and moving from one wifi to another kept the old streams until the 45 s of silence.
- **Applying a decision** (`src-tauri/src/network.rs`):
  - The network is kept in `AppState`, not in the `Store`, so a `Store` made later knows it too.
  - `Offline` calls `Store::offline()`.
  - `Changed` after a `none` calls `online()`; any other `Changed` calls `network_changed()`.
  - `Baseline` changes nothing in the `Store`.
  - Without an active `Store`, only `AppState` changes.
- **One line per decision** on stderr, `cornercase-mobile: network <kind>[ expensive]`. On iOS it goes to the unified log, next to the connection's changes, so the time from a change to `connected` can be measured on an iPhone.
- **Tests**: `network::follow` and `apply` run on Tauri's `MockRuntime` with `FakeTransport`, on a paused clock. Only the simulator check below covers what CI cannot run: `follow_in_background`, the wiring in `run`, the plugin's `ios.rs` and the Swift.

## Offline

- **"No network"** (`Connection::Offline`, `{ state: "offline" }` in `connection-changed`).
  - It drops the streams, or abandons the attempt in progress.
  - Like `Retrying`, it keeps the last session and app.
  - It starts no timer and does not move the backoff, so nothing is spent on a network that is not there.
  - `offline()` reaches it from any state, `Retrying` and the stopped ones (`Missing`, `Unauthorized`, `Incompatible`) included.
  - `online()` tries at once, with the backoff back at 1 s.
- **`connect` with the network down** makes the new `Store` and calls `offline()` before its first attempt. Calls queued before an attempt starts are read first, so switching machines without a network tries nothing.
- **The return to the foreground while offline tries once** (the task's P3).
  - Why: a false `none` with no path after it would leave the app stuck, and an attempt without a network fails in milliseconds.
  - If the attempt fails while the network is still `none`, the `Store` goes back to `Offline`, not to `Retrying`. If it connects, the `Store` forgets that the network was down.
  - Only `online()` brings the network up: `network_changed()` does nothing while it is down.
  - Rejected: leaving `Offline` only on a network event.

## What a network change interrupts

- **A network change interrupts an attempt; the return to the foreground does not.**
  - `network_changed()` while `Connecting` (picking a route, reading `contract`, waiting for the first snapshot) abandons the attempt and starts another at once. A route won over the old network just before it went would otherwise leave streams that stay dead for 45 s.
  - `resume_now()` still does not interrupt (Store › `resume_now`): the network did not change, and a burst of returns would ask for `contract` again and again.
  - The debounce keeps a burst of network changes from doing the same.
  - Precedent: upstream's `design/remote-windows.md`, "a poke that comes while a link connects remakes that connection".
  - Rejected: remembering the change and reconnecting once `Connected`, which is what upstream does. It needs more state for the same result.
- **In the other states both behave the same**: while `Connected` both drop the streams and pick a route again, and the queue moves to the new transport (`queue.reconnect`). In `Retrying` both skip the wait. In a stopped state both try once.
- **Every change reconnects**, even when a TCP connection over Tailscale could have survived it. The app cannot tell whether a socket still works; a reconnection costs about a second, while a dead socket costs 45 s of waiting.

## Tailscale and false `none`

- **Tailscale and the whole `NWPath`**: entering or leaving Tailscale adds or removes a `utun` interface without changing `kind`, which stays `wifi`.
  - Swift compares the whole `NWPath` (`Equatable`), not `{kind, expensive}`, so it does send that path.
  - The debounce counts every settled path after the baseline as a change, so that path reconnects like any other.
  - With Tailscale as the only network, `kind` is `other`, and the same holds.
- **False `none`**: `NWPathMonitor` sometimes reports `unsatisfied` for an instant, and in the simulator after the Mac sleeps.
  - The debounce keeps one that lasts less than a second from showing "No network". The path that follows it still settles as a change, so it costs a reconnection of about a second: the debounce cannot tell a false `none` from a move to another wifi, which looks the same.
  - A longer one shows "No network" until the next path, or until the return to the foreground (Offline, above).

## Checked live

- **In the iPhone 17e simulator** on 2026-11-09 (iOS 27.2, Xcode 27.2, macOS 26.6.2, Tauri 2.12.5), against the bridge on loopback and cornercase 0.13.19:
  - The plugin loads. On each of three launches, the unified log shows the baseline `network wifi` about a second after the identity line.
  - The app connects and counts the panes that the Mac's `status` has.
  - Killing the bridge sends the app to retrying, keeping the session. Starting the bridge again on the same port brings it back on its own.
  - On the return from the background, the three old commands ended on the Mac and three new ones were running within half a second, none left over. No `network` line appeared: the monitor sends nothing on a return.
  - After `xcrun simctl terminate`, the next launch connects to the saved machine on its own.
  - So none of 013's checks regressed (Store › Checked live).
- **Not checked: a real change of network.** The simulator shares the Mac's network and cannot move to cellular, and turning the Mac's wifi off would cut everything else running on it.
- **Pending, for the user on an iPhone** (CA9 of task 028, or 029 if nobody does it before), each time measured from the `network` line to `connected`:
  - wifi → cellular with Tailscale, and cellular → wifi, with the board and the terminal open;
  - `none` and "No network" in the UI;
  - a change while an attempt is running;
  - entering and leaving Tailscale.

## If the plugin breaks

- **Plan B**, the task's, for a native plugin that does not work: keep the retries on the return to the foreground and on a stream that fails, and lower the transport's silence to 20 s. Not done as well as the plugin, since the plugin works.
- **Plan C**: upstream's polling in Rust (`design/architecture/plugins.md`). It would replace the plugin if a later iOS or Tauri broke it, before falling back to plan B.
