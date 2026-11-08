# Look

Follows cornercase-gpui's `design/widgets.md` and `src/theme.rs`: usecornercase.dev's colours, glass on one ground, rounded shapes with air, cobalt, the mark, motion. The tokens are ported from `theme.rs` into CSS custom properties; what changes for touch and small screens is recorded here. gpui's sources were read on `main` on 2026-11-08.

## Tokens

- **`src/lib/theme/tokens.css` keeps `theme.rs`'s names in kebab-case** (`brand_light` is `--brand-light`, `radius::CARD` is `--radius-card`), so a colour is found by the same name in both repositories. The colours are `mod colours`; `--glass` is gpui's opaque ground, the one it uses on Linux.
- **The whole palette is ported, not only what the first screen uses.** The states, `--veil`, `--field`, `--selection` and the rest are one piece of data copied as a whole, so the two repositories cannot drift apart; that is not dead code, which is about functions and components. A test checks that every `var(--x)` in `src/` is defined, in `tokens.css` or in the same file (a component's own property, like `Mark`'s `--unit`); it does not check that every token is used.
- **Spacing, fonts and durations are not in `theme.rs`, so their names are ours.** `--gap` (8 px) and the panel margins `--inset` (12 px) and `--inset-wide` (16 px) come from the literals in gpui's `ui/mod.rs`; `--enter`, `--leave` and `--pop` from `ui/motion.rs`'s `ENTER`, `LEAVE` and `POP`. The durations also live in `src/lib/ui/motion.ts`, which the transitions read, and a test keeps the two equal.
- **No Tokyo Night colour**: the terminal's palette comes with the terminal (task 020).
- Colours with an alpha are written `rgb(r g b / a)` from the token's channels, not `color-mix()`, which iOS 15 lacks (the deployment target is 15.0).

## Ground and glass

- **The ground is opaque.** In gpui the glass shows the desktop behind the window; behind the WebView there is nothing, so the page paints gpui's Linux ground, `--glass`, with `glass::wash` at the top: `--brand` at 12 % fading to nothing over the top 180 px. The wash is on a `position: fixed` layer, so it does not scroll away and the rubber band of iOS scrolling never shows a light gap.
- **Floating cards keep gpui's values**: the vertical gradient from `#1c1f24` at 88 % to `#101215` at 91 %, the 1 px border, the `lift` shadow and the highlight along the top. Unlike gpui, WebKit does blur the page's own content under a `backdrop-filter`, so `blur(24px)` is set with and without the `-webkit-` prefix (iOS 15–17 only read the prefixed one). On the empty screen only the ground and the wash sit behind the card, so the blur barely shows; it will matter once content scrolls under a card. Blur costs GPU on large scrolling surfaces, so it goes on floating cards only, never on lists or the ground.
- **Checked by eye in the iPhone 17 simulator, iOS 27.2, on 2026-11-08**: the card stands out from the ground at gpui's alphas, so none changed. The card's title is 20 px semibold, its lines 16 px with a line height of 1.4, the wordmark 22 px and the button's label 16 px medium. gpui sets 12–13 px text and a 17 px title for a mouse and a desktop; on a phone, body text goes to 15–17 px and titles to 20–22 px. The lines use `text-wrap: pretty`, which kept a single word from ending a paragraph; older WebKit ignores it. The mark and the card sit at the top of the screen rather than its middle, as the screens with machines will.

## Touch and motion

- **Every control is at least 44 px tall**, Apple's minimum touch target, and the pressed state is `:active`, never hover, which touch screens lack.
- **WebKit on iOS applies `:active` only to elements with a touch listener**, so `Button` carries an empty `ontouchstart`. The first screen's button is resting (inert), so the press could not be seen yet: check it with a real tap once task 014 turns the button on.
- **A resting button** keeps the primary fill at 40 % opacity and is `disabled`, `aria-disabled` and deaf to touches, like gpui's `resting_button`.
- **Motion follows `ui/motion.rs`**: entering rises 14 px and fades in over 180 ms with `cubicOut` (gpui's `ease_out`), leaving fades out over 120 ms with `quadIn` (its `ease_in`), both from `svelte/transition` and `svelte/easing`. With `prefers-reduced-motion: reduce`, every duration is 0. Checked on 2026-11-08 in the same simulator with Reduce Motion on and off, on a recording of the launch read frame by frame: with motion, the card passes through intermediate positions and brightness; with Reduce Motion, it appears in its final place in one frame. Nothing animates forever while idle.

## Fonts

- **System fonts only**: `--font-ui` is `system-ui` (SF Pro on iOS) and `--font-mono` is `ui-monospace` (SF Mono). The website's Archivo and JetBrains Mono were rejected for now: they add weight to the bundle, bring their licences into the app, and the CSP allows only `'self'`, so they would have to be bundled rather than fetched.

## Icons

- **Lucide is copied, not installed**, as gpui does in `assets/icons/`: version 0.460.0, with its ISC `LICENSE` next to the shapes in `src/lib/ui/icons/`. A package would bring in thousands of icons for the few the app draws.
- **The SVG's shapes are a table in `icons/shapes.ts`**, drawn by `Icon.svelte` with `stroke="currentColor"` and a stroke width of 1.75. Rejected: the `.svg` file pasted into a component (its `<!-- @license -->` line is a markup comment, which `scripts/check-rules.mjs` refuses), imported raw into `{@html}` (an error under `eslint-plugin-svelte`'s recommended rules), an `<img>` (it does not inherit `currentColor`) and a CSS `-webkit-mask-image` (it depends on the prefix on iOS 15 and goes through the CSP's `img-src`). An icon is added to the table, with its shapes copied from the same Lucide version, when a screen first needs it.

## Components

- **A component arrives with its first use**, so nothing sits unused. The first screen brought `Glass`, `Button` (primary and resting), `Emblem` (gpui's `emblem`, `emblem_page` and `emblem_line`), `Mark`, `Wordmark` and `Icon` (`server` only). The spinner, sheet, notice card, the dot and cursor that pulse while connecting and the normal and danger tones of `Button` come with task 014; `StatusDot` and the working orbit with 015; `Chip` with 014, 015 or 027. A gallery that only shows them was rejected for the same reason.
- **`src/routes/` holds the screens**, starting with `NoMachines.svelte`, which `App.svelte` mounts until task 014 brings navigation. `src/lib/theme/` holds the tokens and the page's base styles, which `main.ts` imports so that the tests' server rendering does not need them.

## Launch

- **No white flash and a light status bar.** `tauri.conf.json` gives the window `backgroundColor` `#0b0c0e`: wry 0.57.1 (`wkwebview/mod.rs`, read on 2026-11-08) paints it on iOS on the WKWebView and on its view with `setOpaque(false)`; without it the view uses `systemBackgroundColor`, which is pure black in dark mode and white in light mode. `LaunchScreen.storyboard` takes the same colour in sRGB instead of `systemBackgroundColor`, and `project.yml` sets `UIUserInterfaceStyle: Dark`, so the status bar is light whatever the phone's setting. Checked in the iPhone 17 simulator, iOS 27.2, on 2026-11-08: the launch screen is the dark ground with a light status bar, and on a recording of the launch read frame by frame no frame between the home screen and the card is lighter than the ground.
- **`pnpm tauri ios dev` does not run xcodegen** once the project exists, so a change to `project.yml` reaches `cornercase-mobile_iOS/Info.plist` only by running `xcodegen generate --spec project.yml` in `src-tauri/gen/apple`; commit what follows from the change and drop the rest of the `.pbxproj` it rewrites. Each `ios dev` then writes `Info.plist` again without its final newline: that change is noise, revert it.
- **The app icon is still Tauri's default.** Making it with `pnpm tauri icon` from the mark is a task of its own.
