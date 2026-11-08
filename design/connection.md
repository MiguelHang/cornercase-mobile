# Connection

Follows cornercase-gpui's `design/connection.md`: the command line is the only contract, streams, an ordered queue, one `Store`.

- **Transport behind a trait**: `run(argv, stdin, stdout) → exit code`, with stdin and stdout as streams of lines, so `--follow` streams and commands that read stdin fit. The bridge is the first implementation; SSH, iroh and Cloudflare come later as new transports.
- **Machines with several routes**: each saved machine has routes (phase 0: bridge URLs, e.g. LAN and Tailscale). The first route that answers wins; routes are tried again on a network change and when the app returns to the foreground.
- **Adding a machine**: scan the bridge's QR code (the official `barcode-scanner` plugin) or paste its URL.
- **Identity**: an Ed25519 key pair from the start, stored in the Keychain (iOS) or the Keystore (Android), to be reused as the iroh identity. The bridge does not use it.
- **`contract --json` on connect**: the app keeps `boot`; a different `boot` means the server restarted and the `Store` is rebuilt. The app checks that the commands it uses are in the contract and says which are missing.
- **Typed commands** over the CLI's `--json` output, exposed to the WebView with generated TypeScript types.
- **Features, not versions**: the app checks the operations and `features` that `contract` announces, never a minimum cornercase version, so a build of upstream `main` works and a missing command is named.
