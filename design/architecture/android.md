# Android

A topic of [Architecture](../architecture.md). The same app on Android, with nothing new to do (030): the Gradle project `tauri android init` writes, what was changed in it by hand and why, where the emulator finds Vite and the bridge, and what was checked live. Where Android keeps the route tokens until its Keystore is in `design/connection.md` (Identity › Android's route tokens, until 031).

## The project

- **`src-tauri/gen/android` as `android init` wrote it** (Tauri CLI 2.12.4), in git like `gen/apple` (Native projects in git): the Gradle project, `MainActivity.kt`, the resources and the launcher icons made from `src-tauri/icons`. Only three files are changed by hand: `app/src/main/AndroidManifest.xml` (permissions, `networkSecurityConfig`, `allowBackup`, `dataExtractionRules`), and the two new `res/xml/network_security_config.xml` and `res/xml/data_extraction_rules.xml`. `init` changed nothing outside `gen/android`.
- **What stays out of git**: what the two `.gitignore` files `init` writes leave out, which is what Gradle, the Tauri CLI and the Rust build write and what belongs to one Mac (`build`, `.gradle`, `local.properties`, `.idea` caches, `.cxx`, `key.properties`, `.tauri`, `tauri.settings.gradle`; in `app/`, `src/main/**/generated`, `src/main/jniLibs/**/*.so`, `src/main/assets/tauri.conf.json`, `tauri.build.gradle.kts`, `proguard-tauri.pro` and `tauri.properties`), plus the root's `/src-tauri/gen/**/build/`. After `pnpm tauri android dev`, `git status` is clean (checked on 2026-11-10).
- **`dev.usecornercase.mobile`** is the `applicationId` and the `namespace`, from `identifier` in `tauri.conf.json`; a debug build is `dev.usecornercase.mobile.debug` (`bundle.android.debugApplicationIdSuffix`), so a debug build and a release one can live on the same phone.
- **`minSdk` 24** (Android 7.0), pinned in `tauri.conf.json` › `bundle.android.minSdkVersion`, so a later `init` keeps it. It is Tauri's default and the floor of the plugins the app uses (barcode-scanner 2.5.3 and haptics 2.4.2 ask for 24); it covers almost every phone in use, the WebView is updated through the Play Store rather than with the system, and nothing 031 or 032 needs asks for more: `AndroidKeyStore` has AES-GCM since API 23. Rejected: 26 (fewer old phones to test, nothing the app uses) and 28 (cleartext off by default, which the network security config already says).
- **Permissions**: `INTERNET`, `CAMERA` (the QR scanner, which declares it too; twice does no harm) and `ACCESS_NETWORK_STATE` (032's `ConnectivityManager`). The scanner asks for the camera at run time, the first time it opens.
- **Targets**: `rust-toolchain.toml` lists only `aarch64-linux-android`, which runs on the arm64 emulator of an Apple Silicon Mac and on phones, so rustup brings it with the toolchain as it does the iOS ones, and CI downloads one target more, not four. `android init` also adds `armv7-linux-androideabi`, `i686-linux-android` and `x86_64-linux-android` to the local rustup, which only a release build for every ABI needs.

## Cleartext

- **`network_security_config.xml`**: `base-config cleartextTrafficPermitted="false"`, and cleartext only to `localhost` and `127.0.0.1` (Vite, below) and to `ts.net` with its subdomains (Tailscale's MagicDNS names). The `ts.net` entry only matters to the WebView, which reaches nothing there today (below); it is kept so a MagicDNS dev host would load. Nothing allows cleartext everywhere, in debug or in release. Rejected: `cleartextTrafficPermitted="true"` in `base-config`, the quick way to make `android dev` work.
- **No IP ranges**: a `<domain>` is a host name, and an IP address matches only whole, so "the private ranges and Tailscale's" cannot be written in that file. Listing the Macs' addresses one by one would break on the next DHCP lease.
- **The file does not govern `ws://`**: Android's `NetworkSecurityPolicy` is applied by the Java stacks and the WebView, not by native sockets, and the bridge's WebSocket is opened by tokio-tungstenite in Rust (All networking in Rust). What it guards is the WebView, whose CSP already lets it reach nothing but the app and Tauri's IPC (A strict CSP).
- **The boundary is the bridge**: it listens only on the Mac's private LAN and Tailscale addresses, never on a public one (`design/bridge.md` › Which addresses), and its token travels only there. A guard in Rust that refuses `ws://` routes outside those ranges would make the phone hold the line too; it is a feature of its own, left for a task apart.
- **`usesCleartextTraffic` is kept as the template wrote it** (`${usesCleartextTraffic}`, `true` in debug): from API 24 a `networkSecurityConfig` makes Android ignore it, so the file above decides in both builds. `debug-overrides` cannot help: it only takes trust anchors, not cleartext.

## Backups

- **`android:allowBackup="false"`**: `machines.json` and `route-secrets.json`, with the tokens in plain text, live in the app's data folder, and must not leave the phone in a cloud backup or through `adb backup`. From Android 12, with the template's `targetSdk` 37, `allowBackup="false"` no longer stops a device-to-device transfer, so `data_extraction_rules.xml` excludes every domain from both `cloud-backup` and `device-transfer`. Below Android 12, `allowBackup` alone is enough, so there is no `fullBackupContent`.

## Running it

- **Vite through `adb reverse`**: `pnpm tauri android dev` publishes port 1420 on the device with `adb reverse` and loads `http://localhost:1420/`, without `TAURI_DEV_HOST` (CLI 2.12.4), so `localhost` in the network security config is enough and hot reload works. The CLI leaves the `adb reverse` behind when it stops.
- **The emulator reaches the bridge** at the Mac's LAN address (through the emulator's outgoing NAT) or its Tailscale address, which are the ones the bridge prints. Not `10.0.2.2`, the Mac's loopback seen from the emulator: the bridge only listens there with `--loopback`, and then prints `127.0.0.1`, which in the emulator is the emulator itself. The code goes into the form by `adb shell input text` (cut into short pieces, below) or the emulator's clipboard, not scanned.

## Versions

Chosen on 2026-11-10.

- JDK 17 (Homebrew's `openjdk@17`): what AGP 9 asks for at least. The template still compiles Kotlin and Java for 1.8.
- What the template of Tauri CLI 2.12.4 brings, kept as it is: AGP 9.3.1, Kotlin 2.2.10, Gradle 9.6.1, `compileSdk` and `targetSdk` 37. Gradle downloads `platforms;android-36` and `build-tools;36.0.0` on the first build by itself, since Tauri's and the plugins' Android libraries compile against 36.
- NDK 29.0.14206865, the stable 29, installed with `sdkmanager` and pointed at by `NDK_HOME`. Rejected: the one the Tauri CLI offers to install, a release candidate (29.0.13846066).
- The SDK from Homebrew's `android-commandlinetools` cask, without Android Studio: `sdkmanager` (which warns that it is deprecated in favour of `android sdk`, and still works), `platforms;android-37.0`, `build-tools;37.0.0`, the emulator and `system-images;android-37.0;google_apis;arm64-v8a`.

## Checked live

On 2026-11-10, in an arm64 emulator (Pixel 8 profile, image `android-37.0;google_apis;arm64-v8a`, Android 17, API 37), emulator 37.2.13, Tauri 2.12.5 and its CLI 2.12.4, NDK 29.0.14206865, JDK 17:

- `pnpm tauri android dev` builds, installs `dev.usecornercase.mobile.debug`, loads the UI from Vite, and a change to a `.svelte` file shows without a reload.
- The bridge's code, typed into the form, adds the machine; it connects through the Mac's Tailscale address and shows the live board.
- `machines.json` and `route-secrets.json` are in the app's data folder, readable only by its user (`-rw-------`), with one key per route. After `am force-stop` and a new start the machine is there and connects again without the code.
- Machines › This device shows "no secret store on this platform yet" in place of the fingerprint, and the machines load all the same.
- The scanner asks for the camera and opens; Cancel goes back. Its viewfinder stays black in the emulator: CameraX binds, but the emulated camera never opens (the AVD has no front camera, and CameraX's check of the lenses fails), so no code was scanned there.
- Not checked: a real phone, over USB or Wi-Fi and through Tailscale (the ticket's own check, left to the user), scanning a real code, and the release build.
- Seen and left for later: the status bar's dark text on the dark background, and the WebView staying scrolled after the keyboard hides until the next tap (032's keyboard).
