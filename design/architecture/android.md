# Android

A topic of [Architecture](../architecture.md). The same app on Android (030): the Gradle project `tauri android init` writes, what was changed in it by hand and why, how the Keystore keeps the secrets (031), where the emulator finds Vite and the bridge, and what was checked live. What the secrets are, and how the route tokens left the file 030 kept them in, is in `design/connection/identity.md`.

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

- **`android:allowBackup="false"`**: `machines.json` and the preferences that hold the sealed secrets live in the app's data folder, and must not leave the phone in a cloud backup or through `adb backup`. Since 031 no token is in plain text there (030's `route-secrets.json` is moved and removed on the first start), and a sealed secret copied to another phone would not open without this one's Keystore key; the machines' names and addresses still would leave, so backups stay off. From Android 12, with the template's `targetSdk` 37, `allowBackup="false"` no longer stops a device-to-device transfer, so `data_extraction_rules.xml` excludes every domain from both `cloud-backup` and `device-transfer`. Below Android 12, `allowBackup` alone is enough, so there is no `fullBackupContent`.

## The Keystore

How the secrets plugin (`src-tauri/plugins/secrets/android/`, `design/architecture/plugins.md`) keeps a secret. Why a sealed seed and not a key in the Keystore: `design/connection/identity.md`.

- **One AES-256 key in `AndroidKeyStore`**, for encrypting and decrypting only, GCM without padding, made on the first `set` and found by its alias afterwards; a `get` never makes one. It cannot be exported: no call returns its bytes, and on a phone with a TEE they never leave it. The Keystore makes the 12-byte IV of every encryption (a key made this way refuses one from the caller), so an IV is never reused by mistake. GCM's tag also says whether a sealed secret was changed.
- **The secret's name as associated data**: the name goes into GCM as AAD, so a value moved to another name, by hand or by a bug, does not open (wrong key, rather than the other secret's bytes).
- **The app's own preferences** (`MODE_PRIVATE`), one entry per secret: its name, and the IV followed by the ciphertext in Base64 (Android's `android.util.Base64`). The names, `identity/ed25519` and `route/<machine>/<address>`, are in clear, as they are in `machines.json`: they are the AAD and say nothing secret. **`commit()`, not `apply()`**: when `set` or `delete` answers, the change is on disk, as the adapters of the other platforms promise; `commit()` blocks for a few milliseconds, which the caller does anyway, since `run_mobile_plugin` waits.
- **No `setUnlockedDeviceRequired`**: the app reconnects while the screen is locked (a network change, the return of a stream), and with it the key could not be used then. The preferences live in the credential-encrypted storage, which Android opens at the first unlock after a boot, and the key lives in a Keystore that never moves to another phone: together that is iOS's `AfterFirstUnlockThisDeviceOnly` (Identity and secrets › The Keychain). The app does not run before the first unlock (it is not `directBootAware`).
- **No user authentication** (`setUserAuthenticationRequired`, biometrics): a fingerprint at every start or reconnection, for a seed and development tokens. **No StrongBox** (`setIsStrongBoxBacked`, API 28+): slower per operation, missing on many phones and in the emulator, where it could not be tested, and the TEE is enough for what the key protects.
- **A lost key** (some makers' phones drop Keystore keys when the screen lock is changed or removed) is an error, never a quiet new identity: a `get` of a secret whose key is gone fails; a later `set` (a new machine) makes a new key for its own secret, and the old ones still do not open, now with the wrong key (`design/connection/identity.md` › A secret that no longer opens).
- **Threads**: `Secrets` is called from `setup` and from async commands on tokio's workers; each call goes to Kotlin, which keeps no state but the key it found, under `@Synchronized`, and two `commit()`s on the same preferences are serialized by Android.

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
- With 030, `machines.json` and `route-secrets.json` are in the app's data folder, readable only by its user (`-rw-------`), with one key per route. After `am force-stop` and a new start the machine is there and connects again without the code.
- With 030, Machines › This device shows "no secret store on this platform yet" in place of the fingerprint, and the machines load all the same.
- The scanner asks for the camera and opens; Cancel goes back. Its viewfinder stays black in the emulator: CameraX binds, but the emulated camera never opens (the AVD has no front camera, and CameraX's check of the lenses fails), so no code was scanned there.
- Not checked: a real phone, over USB or Wi-Fi and through Tailscale (the ticket's own check, left to the user), scanning a real code, and the release build.
- Seen and left for later: the status bar's dark text on the dark background, and the WebView staying scrolled after the keyboard hides until the next tap (032's keyboard).

The Keystore (031), on 2026-11-10 in the same emulator and versions (system build CE2A.260420.051.A3), with debug APKs built by `pnpm tauri android build --debug --apk`, after which `git status` is clean:

- **The update from 030**: 030 installed, a machine added from the bridge's code, with three routes (the LAN, and Tailscale's IPv4 and IPv6) and so three tokens in `route-secrets.json`. Once 031 is installed over it, the first start logs `moved 3 route secrets into the store` and then the identity's fingerprint; the machine connects without the code, `route-secrets.json` is gone and `machines.json` stays.
- **The identity**: its fingerprint is in logcat and in Machines › This device. It is the same after `am force-stop` and a new start (with no second move), and after a cold boot (`adb reboot`), after which the machine connects again. After `adb uninstall` and a new install it is another one.
- **The preferences** (`run-as … cat shared_prefs/cornercase-secrets.xml`, `-rw-rw----`, which is what `MODE_PRIVATE` gives) hold only the identity and the three tokens, each Base64 that decodes to binary; the bridge's token appears neither in clear nor in hex. Only the names are readable.
- **The cost**: reading a secret takes tens of milliseconds (the identity's line came 30 to 110 ms after its read began, after a reboot and after a force-stop; the first writes, which make the key, about 80 ms). Loading the machines reads one token per route, so a few routes add a fraction of a second at most to the start.
- Not checked: a real phone, the case the Keystore changes most between makers (StrongBox or not, a key lost when the screen lock changes), left to the user (`docs/tasks/031-android-keystore.md`); a move that fails, which the core's tests cover; and the release build.
