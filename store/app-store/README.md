# The App Store listing

What "Cornercase remote" shows on the App Store and tells App Review, in a form that can be applied as it is through
the [App Store Connect API](https://developer.apple.com/documentation/appstoreconnectapi). Nothing here has been
applied yet. The decisions behind it are in [design/releases/app-store.md](../../design/releases/app-store.md).

## The format

- **One JSON file per group of API resources.** Each top-level key is a resource type of the API (`appInfos`,
  `appStoreVersionLocalizations`…), and under it `attributes`, `relationships` and `included` are what goes, as they
  are, into the request's `data.attributes`, `data.relationships` and `included`. The attribute names are those of the
  App Store Connect OpenAPI specification 4.6.
- **Localizations are keyed by locale** (`en-US`): the key is the `locale` attribute when the localization is created.
- **`{ "$file": "<path>" }`** stands for the whole content of that file, relative to this folder, as the attribute's
  string. The long texts live in plain `.txt` files so that what is pasted or sent is exactly the file: no markdown, no
  typographic quotes.
- **`null`** is a value still to fill in at upload time (an id, a URL, the reviewer's contact) or an attribute left
  empty on purpose. Ids of the app, its `appInfos`, its `appStoreVersions` and their localizations are looked up at
  upload time and never kept here.

## Files, resources and endpoints

| File | Resource | Endpoint |
| --- | --- | --- |
| `app-info.json` › `apps` | content rights: no third-party content | `PATCH /v1/apps/{id}` |
| `app-info.json` › `appInfos` | the app's info: categories | `GET /v1/apps/{id}/appInfos` (the one being edited), then `PATCH /v1/appInfos/{id}` |
| `app-info.json` › `appInfoLocalizations` | name, subtitle, privacy policy URL | `GET /v1/appInfos/{id}/appInfoLocalizations`, then `PATCH /v1/appInfoLocalizations/{id}`, or `POST /v1/appInfoLocalizations` with the `appInfo` relationship |
| `version.json` › `appStoreVersions` | version string, copyright, release type | `PATCH /v1/appStoreVersions/{id}` on the version being prepared, or `POST /v1/appStoreVersions` with `platform` `IOS` and the `app` relationship |
| `version.json` › `appStoreVersionLocalizations` | description, keywords, promotional text, what's new, URLs | `GET /v1/appStoreVersions/{id}/appStoreVersionLocalizations`, then `PATCH /v1/appStoreVersionLocalizations/{id}`, or `POST /v1/appStoreVersionLocalizations` |
| `age-rating.json` | the age rating questionnaire | `GET /v1/appInfos/{id}/ageRatingDeclaration`, then `PATCH /v1/ageRatingDeclarations/{id}` |
| `pricing.json` › `appPricePoints` | the price point of a price of 0 in the USA | `GET /v1/apps/{id}/appPricePoints?filter[territory]=USA`, the one whose `customerPrice` is `0.0` |
| `pricing.json` › `appPriceSchedules` | the price: free, with the USA as base territory | `POST /v1/appPriceSchedules`, with that price point's id in `included[0]` and the `app` relationship |
| `pricing.json` › `appAvailabilities`, `territoryAvailabilities` | every territory, and the new ones | `POST /v2/appAvailabilities`, with one `territoryAvailabilities` in `included` for each territory of `GET /v1/territories` |
| `review.json` | App Review Information: contact, sign-in, notes | `GET /v1/appStoreVersions/{id}/appStoreReviewDetail`, then `PATCH /v1/appStoreReviewDetails/{id}`, or `POST /v1/appStoreReviewDetails` with the `appStoreVersion` relationship |
| `screenshots/iphone-6.9/*.png` | the 6.9" iPhone screenshots, in their names' order | `POST /v1/appScreenshotSets` with `screenshotDisplayType` `APP_IPHONE_67` on the en-US version localization, then for each file `POST /v1/appScreenshots`, upload its parts, `PATCH /v1/appScreenshots/{id}` with `uploaded` and the checksum |
| `review/demo.mp4` | the screen recording for App Review, not an app preview | `POST /v1/appStoreReviewAttachments` on the review detail, upload its parts, `PATCH /v1/appStoreReviewAttachments/{id}` with `uploaded` and the checksum |

## Order

1. `app-info.json`: content rights, categories, then the en-US localization.
2. `version.json`: the version (create it or patch the one App Store Connect prepared), then its en-US localization.
3. `age-rating.json`.
4. `pricing.json`: the price point, the price schedule, then availability.
5. `review.json`, with the contact filled in at upload time, then the recording.
6. The screenshots.
7. What has no API (below), then `Add for Review` with the build, by hand.

## Still missing before submitting

- **The privacy policy's URL** (`privacyPolicyUrl`): `PRIVACY.md` at a public URL, once the repository is public.
- **The support URL** (`supportUrl`): for example the repository's issues, at an absolute URL.
- **The reviewer's contact** (`contactFirstName`, `contactLastName`, `contactPhone`, `contactEmail`): given at upload
  time only, never written into this repository.
- **The screenshots and the recording, taken again**: they still show the app's own words for the demo, "Demo Mac" and
  "No Mac nearby?", while the listing and the notes speak of a machine, since cornercase runs on Linux and macOS. Once
  task 070 renames them in the app, take them again with the steps below, before submitting.

## By hand, without an API

- **App Privacy**: "Data Not Collected" (`PRIVACY.md`). The API has no resource for it.

## What's new

`whatsNew` is not applied to the first version: App Store Connect does not take it there. `en-US/whats-new.txt` is
the base for the next one, whose text comes from that version's section of `CHANGELOG.md`, leaving out what is not
about the iPhone, such as Android.

## Screenshot sizes

The screenshots are 1320×2868, the 6.9" iPhone's display (`APP_IPHONE_67`), which App Store Connect scales down to the
smaller iPhones. App Store Connect may also ask for the "Dynamic Island (medium)" size: check it at upload time, and if
it does, that is one more set from an iPhone 17 Pro or later.

## Regenerating the screenshots and the recording

On a Mac with Xcode and the iOS simulator runtime (`CLAUDE.md`), only in the demo, never with a real machine paired.
With other simulators booted, give `simctl` the UDID, never `booted`.

1. **The simulator**, kept for the next version: "cornercase store 6.9", an iPhone 17 Pro Max on iOS 27.2, whose
   display is 1320×2868. If it is gone: `xcrun simctl create "cornercase store 6.9"
   com.apple.CoreSimulator.SimDeviceType.iPhone-17-Pro-Max com.apple.CoreSimulator.SimRuntime.iOS-27-2`, which prints
   the new UDID; `xcrun simctl list devices | grep "cornercase store"` finds it later.
2. **In English**: a new simulator takes the Mac's language, and the status bar then reads `09:41`, the keyboard and the
   spell checker another language. `xcrun simctl spawn <UDID> defaults write -g AppleLanguages -array en-US`, the same
   with `-g AppleLocale -string en_US` and `-g AppleKeyboards -array "en_US@sw=QWERTY;hw=Automatic"`, then `xcrun simctl
   shutdown <UDID>` and `xcrun simctl boot <UDID>`.
3. **The app** with `dist/` embedded and no Vite server: `pnpm build && LANG=en_US.UTF-8 pnpm tauri ios dev "cornercase
   store 6.9" --no-watch --features tauri/custom-protocol --config '{"build":{"beforeDevCommand":"","devUrl":null}}'`,
   on a clean install with no machines. It ends once the app is deployed. Afterwards `git checkout
   src-tauri/gen/apple`, which `ios dev` rewrites. The demo is not kept: each launch opens on `No machines yet`.
4. **The status bar**, again after every boot: `xcrun simctl status_bar <UDID> override --time 9:41 --batteryState
   charged --batteryLevel 100 --wifiBars 3 --cellularBars 4`.
5. **Moving through the app** with no Simulator.app: a throwaway XCUITest target, outside the repository, that
   attaches to `dev.usecornercase.mobile` and taps by accessibility label (`Try a demo`, `Show 2 subagents`,
   `Terminal`, `Changes`, `Files`, `Start`, `Leave demo`) or by point, built once with `xcodebuild build-for-testing`
   and run with `test-without-building`.
6. **Each screen**: `xcrun simctl io <UDID> screenshot --type=png <file>`, once the screen has settled.
   - `01-board`: the demo's board with `Show 2 subagents` open.
   - `02-chat`: "Fix rounding in the cart totals", whose question card is waiting.
   - `03-terminal`: its terminal.
   - `04-changes`: "Add Apple Pay to the checkout" › Changes › `src/checkout/PaymentForm.tsx`.
   - `05-start`: Start with the workspace `acme-store › main` and a prompt typed in, the keyboard closed.
   - `06-files`: Files of that agent, with `src/checkout` open.

   If the board comes out with the Dynamic Island drawn black, take it again after scrolling the board a little.
7. **No alpha**: simctl's PNGs are RGBA, every pixel opaque. Each one is rewritten as 8-bit RGB (`colorType` 2) with
   only `IHDR`, `sRGB`, `IDAT` and `IEND`: no `tRNS`, no EXIF. Dropping the alpha loses nothing, which `sips` cannot do
   without going through JPEG. A few lines with Python's `zlib` and `struct` do it.
8. **The recording**, `review/demo.mp4`:
   - Relaunch the app so that it opens on `No machines yet`.
   - `xcrun simctl io <UDID> recordVideo --codec=h264 raw.mp4` in the background, waiting for `Recording started` on
     its stderr.
   - The XCUITest goes through the script: `Try a demo`, the board, the waiting agent, the answer `Round the total`,
     `Terminal`, back, `Leave demo`.
   - Stop the recording with SIGINT.
   - `avconvert --source raw.mp4 --preset Preset1920x1080 --output demo.mp4 --start <s> --duration <s>`: `--start`
     cuts the seconds before the first tap. The output is 884×1920 H.264, with no audio and none of the source's
     metadata, which `avconvert` filters. It must stay under 60 seconds; the one here is 32.4 seconds and 2.8 MB.
