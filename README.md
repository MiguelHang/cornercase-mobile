# Cornercase remote

The phone app for [cornercase](https://github.com/usecornercase/cornercase-terminal): follow, answer and start
cornercase's coding agents from your iPhone or Android phone, while they keep running on your Mac.

- See every agent of your projects on one board, with its status, the ones that need you first.
- Read an agent's conversation, answer it, tap its options, or open its live terminal.
- Start an agent or a flow with a prompt or on an issue, in an open workspace or a new worktree.
- Check which files a workspace changed, read the diffs, and browse its files.

What is new in each version is in the [changelog](CHANGELOG.md).

## What you need

- **cornercase 0.13.78 or later** on your Mac.
- A phone on the same network as the Mac, or both on your [Tailscale](https://tailscale.com) network.
- iOS 15 or later on an iPhone, or Android 7.0 or later.

## Install

- **iPhone**: through TestFlight, by invitation for now.
- **Android**: download the `.apk` of the [latest release](../../releases/latest) on your phone and open it. Android
  asks you to allow installing unknown apps from your browser or file manager the first time.

## Pair it with your Mac

1. In cornercase on your Mac, open Settings › Remote › Phone app and turn it on. It shows a QR code, with the code as
   text under it.
2. In the app, open **Machines** › **Add machine**, then scan the QR code, or paste the code.
3. Give the machine a name. Add one machine per Mac; the app opens the last one you used.

A code lasts ten minutes and pairs one phone. To cut a phone off, forget it in the same place in cornercase.

## Help

To ask a question or report a problem, see [support](SUPPORT.md): where to ask and what to include.

## Privacy

The app collects nothing and talks only to the Macs you pair it with. The [privacy policy](PRIVACY.md) says what it
keeps on your phone and how to delete it.

## Development

A Tauri 2 app, with a Rust core and a Svelte UI, that talks to the cornercase server only through its command line.
Setting up Xcode and the Android SDK, the commands, the checks that CI runs and the code rules are in
[CLAUDE.md](CLAUDE.md); the design decisions are in [DESIGN.md](DESIGN.md).

## Licence

Licensed under either of [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
