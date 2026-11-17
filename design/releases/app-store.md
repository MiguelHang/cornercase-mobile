# The App Store listing

A topic of [Releases](../releases.md): what "Cornercase remote" shows on the App Store and tells App Review, kept in the repository so that the API can apply it and the next version can start from it (065, decided on 2026-11-17).

Code: `store/app-store/` (the listing, its texts, the screenshots, the recording for App Review, and `README.md`: the endpoints, the order, what is still missing and how to take the screenshots again), `scripts/store.test.ts`.

## The format

- **One JSON file per group of App Store Connect API resources, in the API's own words**: each top-level key is a resource type, and under it `attributes`, `relationships` and `included` go as they are into the request's body. Whoever applies it maps keys to endpoints and nothing else: no field is renamed, merged or computed. The table of files, resources and endpoints, and the order, are in `store/app-store/README.md`.
- **The long texts in plain `.txt`**, referenced as `{ "$file": "<path>" }`, the one convention the API does not have: what is pasted or sent is exactly the file, a version's changes read as plain diffs, and the files hold ASCII only, with no markdown or typographic quotes for the store to show raw.
- **Not fastlane `deliver`**: its metadata folder would hold the same texts, but it is a Ruby toolchain for one step, the reason `scripts/ios-release.sh` already rejected it (Releases › iOS), and its folder only fastlane reads. These files go to the API from any client, by hand or by a script.
- **The specification**: App Store Connect's OpenAPI 4.6 (`openapi.oas.json` dated 2026-11-04, downloaded on 2026-11-17). Every attribute comes from the resource's create or update request there, and every endpoint in the README was found in it. Left out: `ageRatingOverride` and `usesIdfa`, both deprecated there (`ageRatingOverrideV2` replaces the first); `platform`, which only a version's create takes (the README says to add `IOS` then).
- **Content rights have an API** (`contentRightsDeclaration` on `apps`, `DOES_NOT_USE_THIRD_PARTY_CONTENT`), which the ticket thought was manual. App Privacy has none: "Data Not Collected" (`PRIVACY.md`) is set by hand.
- **Nothing applied** (the user's decision, 2026-11-17): no request has been sent to App Store Connect and the API key was not used. Another session applies it once the user approves, so "through the API where it allows it" is met by a format the API takes as it is.

## What is left `null`

- **The privacy policy's and the support URL** (`privacyPolicyUrl`, `supportUrl`): both are needed to submit, and neither exists until the repository is public (`PRIVACY.md` and the issues, at absolute URLs). `marketingUrl` is optional and stays empty.
- **The reviewer's contact** (`contactFirstName`, `contactLastName`, `contactPhone`, `contactEmail`): personal data, given at upload time and never written into git. `demoAccountRequired` is `false` and the demo account's fields `null`, since the app has no account.
- **Ids**: the app's, its `appInfos`', its version's and their localizations' are looked up at upload time, as is the id of the price point for a price of 0, so nothing here goes stale when App Store Connect makes new ones.
- **`secondaryCategory`**: the ticket names Developer Tools alone, and a second category the app does not fit would only blur where it shows.

## The listing

- **4+**: every question of the age rating is `NONE` or `false`, the 2025 ones included (`messagingAndChat`, `userGeneratedContent`, `advertising`, `parentalControls`, `ageAssurance`, `healthOrWellnessTopics`, `gunsOrOtherWeapons`, `lootBox`, `socialMedia`). The app sends no message to other people: it talks to the user's own cornercase, and what an agent writes is the user's own work. Nothing a user makes is shared with others, there is no web view, since a link in an agent's answer opens in the system's browser (Chat › Markdown), and there are no ads or purchases. `kidsAgeBand` stays `null`: it is not a Kids app.
- **The copyright is "2026 The cornercase-mobile authors"**, as `LICENSE-MIT` says it, with no person's name.
- **Free, in every territory and in new ones**: the price point of 0 with the USA as base territory, and `availableInNewTerritories`.
- **The computer is not a Mac** (the user's decision, 2026-11-17): cornercase runs on Linux and macOS. The listing, the review notes and the README speak of a "machine", the app's own word (`Add machine`, Machines), and of "your computer (Linux or macOS)" for someone new. The notes name the demo's link and what it adds ("the Try a demo link", "a simulated demo machine, marked DEMO") without quoting the app's texts, which still say "No Mac nearby?" and "Demo Mac" until 070 changes them, with `PRIVACY.md` and `README.md`.
- **Guideline 2.3.10**: the listing names no other platform, so no Android, and neither TestFlight nor "beta". The keywords name no other app (Claude, Codex, Tailscale) and no word of the app's name, which the store already searches. The description may name Tailscale as the network the phone reaches the computer through.
- **`whatsNew` is not set on the first version**, which App Store Connect does not take there. `en-US/whats-new.txt` is the base for the next one, written from that version's section of `CHANGELOG.md` without what is not about the iPhone, such as Android.

## The screenshots

- **From the demo, on a simulator made for them**: "cornercase store 6.9", an iPhone 17 Pro Max on iOS 27.2, whose display is the 6.9" size, 1320×2868. It is kept for the next version. A simulator of their own keeps the others, and the user's real machines and agents, out of the pictures; the demo's sample is fictional and in English (Demo › What it is), and each screenshot was looked over for names, emails or paths.
- **Six screens, 6.9" only** (`APP_IPHONE_67`): App Store Connect scales them down to the smaller iPhones. It may also ask for the "Dynamic Island (medium)" size: checked at upload time, and if it does, one more set comes from an iPhone 17 Pro or later.
- **No frames or captions**: the screens as the app shows them, which commands alone can take again after every change to the UI. Frames and text over them would need a design tool and someone to redo them each time.
- **In English**: a new simulator takes the language of the system it runs on, here Spanish, and the status bar then read `09:41`, with another keyboard and the spell checker's underlines. The simulator is set to en-US (`AppleLanguages`, `AppleLocale`, `AppleKeyboards`) and booted again. The status bar's override (`9:41`, full battery and bars) is lost on every boot and set again.
- **The Dynamic Island drawn black**: after a boot, the board's screenshot came out with the island painted in, also with `--mask=ignored`, and only on the board. Scrolling the board a little before taking it clears it.
- **No alpha**: App Store Connect refuses screenshots with an alpha channel, and simctl's PNGs are RGBA with every pixel opaque. Each one is rewritten as 8-bit RGB with only `IHDR`, `sRGB`, `IDAT` and `IEND`, which loses nothing. Rejected: `sips`, which drops the alpha only through JPEG, with loss.
- **Moving through the app with a throwaway XCUITest** outside the repository, which taps by accessibility label: the same screens each time, without Simulator.app. Rejected: a UI test target in `src-tauri/gen/apple`, kept up for screenshots taken once per version.
- **Taken again after 070**: they and the recording still show "Demo Mac" and "No Mac nearby?", while the listing speaks of a machine. `store/app-store/README.md` lists them as missing before submitting.

## The recording for App Review

- **A review attachment, not an app preview** (`appStoreReviewAttachments`): it shows App Review how to get in without a computer running cornercase (`Try a demo`, the board, answering a waiting agent, its terminal, `Leave demo`), and nobody else sees it. An app preview is marketing on the product page, with rules and a set per size of its own.
- **`recordVideo` in H.264, then `avconvert` with `Preset1920x1080`**: 884×1920, 32.4 seconds, 2.8 MB, no audio, and none of the source's metadata, which `avconvert` filters. `--start` cuts the seconds before the first tap. `PresetHighestQuality` kept 1320×2868 at 5.7 MB; the smaller one reads as well at half the weight.
- **In the repository**, since it is under 10 MB: at `store/app-store/review/demo.mp4`, next to the notes that mention it.

## The test

- **`scripts/store.test.ts` checks what App Store Connect would refuse, before an upload does**: the subtitle within 30 characters; the keywords within 100 bytes, each longer than 2 characters and none a word of the app's name; the promotional text within 170 characters; the description and `whatsNew` within 4,000; the review notes within 4,000 bytes; `versionString` equal to `package.json`'s version; every `$file` present; no Android in the listing; no "Mac" as a word in the listing or the notes (`macOS` passes); 4 to 6 screenshots, each 1320×2868, RGB, with no `tRNS`.
- **The PNGs are read by their header and chunk names**, without decoding them: the size, the colour type and `tRNS` are all it needs, so it adds no image library.
- **Bytes where the limit may be bytes** (the keywords and the notes), characters elsewhere: the texts are ASCII, so both agree today, and the stricter count holds if that changes.
- A `$file` that is missing among the description, `whatsNew` and the notes fails the test file as it loads, before any test runs, rather than a named test.

## Checked and not checked

- On 2026-11-17, with Xcode 27.2, the iOS 27.2 simulator and Tauri CLI 2.12.4:
  - the attributes and endpoints against the OpenAPI 4.6 file; every JSON file parses;
  - the screenshots' size, colour type and chunks, and each one by eye;
  - the recording's length, its one video track, its size and its lack of metadata, through AVFoundation, and its frames by eye;
  - `scripts/store.test.ts` failing with a longer subtitle, Android in the description, the word Mac in the notes and a `$file` naming a missing file.
- Not checked: anything in App Store Connect. No request was sent, so neither the API taking these attributes as they are, the price point's lookup, the upload of the screenshots and the recording, the size App Store Connect asks for, nor what App Review makes of the notes.
