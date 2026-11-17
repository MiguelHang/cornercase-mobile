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

On a Mac with Xcode and the iOS simulator runtime (`CLAUDE.md`), never with a real machine paired:

1. A simulator of its own: `xcrun simctl create "cornercase store 6.9" "iPhone 17 Pro Max"
   com.apple.CoreSimulator.SimRuntime.iOS-27-2`, then `xcrun simctl boot <UDID>`.
2. The app with `dist/` embedded and no Vite server: `pnpm build && LANG=en_US.UTF-8 pnpm tauri ios dev "cornercase
   store 6.9" --no-watch --features tauri/custom-protocol --config '{"build":{"beforeDevCommand":"","devUrl":null}}'`,
   on a clean install with no machines. Afterwards `git checkout src-tauri/gen/apple`, which `ios dev` rewrites.
3. The status bar: `xcrun simctl status_bar <UDID> override --time 9:41 --batteryState charged --batteryLevel 100
   --wifiBars 3 --cellularBars 4`.
4. `Try a demo`, then each screen: `xcrun simctl io <UDID> screenshot --type=png <file>`, and the alpha channel
   removed, so that each PNG is RGB (`colorType` 2) with no `tRNS`.
5. The recording: `xcrun simctl io <UDID> recordVideo --codec=h264 <file>` while going through the demo, stopped with
   SIGINT, then shrunk with `avconvert`, which also drops its metadata.
