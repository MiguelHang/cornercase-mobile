# Privacy policy

This policy covers Cornercase remote, the cornercase app for iPhone and Android.

Last updated: 2026-11-16.

## In short

The app collects nothing. It has no account and no server of its own. It has no analytics, no advertising and no third-party SDK. Everything it does happens between your phone and the Macs you pair it with.

## What the app does not do

- It does not collect, sell or share any data about you or about how you use it. Nobody, the developer included, receives any.
- It has no analytics, crash reporting, advertising or tracking. It contains no third-party SDK that could do these things.

## Who the app talks to

The app talks only to the Macs you pair it with, which run cornercase. It reaches them over your local network or your Tailscale network.

- **What it sends to them** is what you ask it to: your answers, keys, the work you start, the images you attach and the agents you close.
- **What it reads from them** is what you open: the state of your agents, their conversations and terminals, and your workspaces' changes and files.
- **When you pair**, the app tells the Mac its name, its version and the phone's platform, such as `cornercase-mobile/0.1.0 (iOS)`, so cornercase can list the phone among its paired devices.

Links you tap in an agent's message open in your phone's browser, outside the app.

## What the app keeps on your phone

The app keeps only these three things:

- **Your machines.** Each Mac's name, the addresses the app reaches it at, and which Mac you used last.
- **Secrets.** The phone's own key, and the secret each Mac gives the phone when you pair. They are kept in the iOS Keychain or the Android Keystore, for this phone only, and never move to another phone.
- **A cache of the conversations.** For each Mac, what you sent and the agents' messages, so that a conversation shows at once when you open it. The cache is capped in size and kept out of the phone's backups.

## Camera and photos

- **The camera** is used only to scan a Mac's pairing QR code. The code is read on the phone, and no picture is kept or sent.
- **Photos.** On iPhone, you can attach images from your photos. The system's picker gives the app only the images you choose, never the rest of your library. The app shrinks each image on the phone and encodes it again without its metadata, so its location is removed. It then sends the image only to the Mac of the agent you are answering. The app deletes its own copy as soon as it has read it, and does not keep the image in the conversation cache.

## Deleting your data

- **Removing a Mac** from the app deletes its addresses, its secrets and its cached conversations.
- **Uninstalling the app** deletes everything it kept, with one exception on iPhone. iOS keeps Keychain items after an app is deleted, so the phone's key and the pairing secrets stay in the Keychain, where nothing but the app can use them. If you install the app again, it gets the same key back. To cut a phone off from a Mac, forget the phone in cornercase on that Mac.
- **On Android**, uninstalling also deletes the Keystore's key and the secrets it protects.

## Changes to this policy

If this policy changes, the new version is published here, with a new date at the top.

## Contact

For questions or requests about privacy, open an [issue](../../issues) in the app's repository.
