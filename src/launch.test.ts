import { describe, expect, it } from 'vitest'
import html from '../index.html?raw'
import infoPlist from '../src-tauri/gen/apple/cornercase-mobile_iOS/Info.plist?raw'
import storyboard from '../src-tauri/gen/apple/LaunchScreen.storyboard?raw'
import projectYml from '../src-tauri/gen/apple/project.yml?raw'
import tauriConf from '../src-tauri/tauri.conf.json?raw'
import markSource from './lib/ui/Mark.svelte?raw'
import tokens from './lib/theme/tokens.css?raw'

const UNIT = 8

const declarations = (css: string, selector: string) =>
  Object.fromEntries(
    (css.match(new RegExp(`(?:^|\\s)${selector.replace('.', '\\.')} \\{([^}]*)\\}`))?.[1] ?? '')
      .split(';')
      .map((declaration) => declaration.trim().split(/:\s*/))
      .filter(([property]) => property !== ''),
  )

const layerStyle = html.match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? ''

const layer = (selector: string) => declarations(layerStyle, selector)

const atUnit = (value: string) => {
  const units = value.match(/^calc\((\d+) \* var\(--unit\)\)$/)?.[1]
  return units === undefined ? value : `${Number(units) * UNIT}px`
}

const markCell = (name: string) => {
  const { left, top, width, height } = declarations(markSource, `.${name}`)
  return { left: atUnit(left), top: atUnit(top), width: atUnit(width), height: atUnit(height) }
}

const token = (name: string) => tokens.match(new RegExp(`--${name}: (#[0-9a-f]+);`))?.[1]

const cellBox = (cell: Record<string, string>) => ({
  left: parseFloat(cell.left),
  top: parseFloat(cell.top),
  width: parseFloat(cell.width),
  height: parseFloat(cell.height),
})

const hex = (color: string) =>
  `#${['red', 'green', 'blue']
    .map((channel) => Number(color.match(new RegExp(` ${channel}="([\\d.]+)"`))?.[1]))
    .map((value) => Math.round(value * 255).toString(16).padStart(2, '0'))
    .join('')}`

const nativeView = (label: string) => {
  const open = storyboard.match(new RegExp(`<view [^>]*userLabel="${label}"[^>]*>`))
  const after = storyboard.slice((open?.index ?? 0) + (open?.[0].length ?? 0))
  return { id: open?.[0].match(/ id="([^"]+)"/)?.[1], body: after.slice(0, after.indexOf('</view>')), after }
}

const constant = (scope: string, attributes: string) => {
  const found = scope.match(new RegExp(`<constraint ${attributes}(?: constant="([\\d.]+)")? id="[^"]+"/>`))
  return found === null ? undefined : Number(found[1] ?? 0)
}

const nativeMark = nativeView('Mark')

const pin = (first: string | undefined, attribute: string, second: string | undefined) =>
  `firstItem="${first}" firstAttribute="${attribute}" secondItem="${second}" secondAttribute="${attribute}"`

const pinned = (id: string | undefined, attribute: string) => constant(storyboard, pin(id, attribute, nativeMark.id))

const nativeCell = (label: string) => {
  const { id, body } = nativeView(label)
  return {
    left: pinned(id, 'left'),
    top: pinned(id, 'top'),
    width: constant(body, 'firstAttribute="width"'),
    height: constant(body, 'firstAttribute="height"'),
  }
}

describe('launch', () => {
  it('lets the page draw under the notch, shrinks it above the keyboard and never zooms it', () => {
    expect(html.replace(/\s+/g, ' ')).toContain(
      '<meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no, viewport-fit=cover, interactive-widget=resizes-content" />',
    )
  })

  it('paints the window in the ground colour before the page loads', () => {
    const conf = JSON.parse(tauriConf) as { app: { windows: { backgroundColor?: string }[] } }
    expect(conf.app.windows[0].backgroundColor).toBe('#0b0c0e')
  })

  it('launches on the ground colour, not the system background', () => {
    expect(storyboard).not.toContain('systemBackgroundColor')
    expect(storyboard).toMatch(
      /<color key="backgroundColor" red="0\.0431\d*" green="0\.0470\d*" blue="0\.0549\d*" alpha="1" colorSpace="custom" customColorSpace="sRGB"\/>/,
    )
  })

  it('is dark in the project and in the generated Info.plist', () => {
    expect(projectYml).toContain('UIUserInterfaceStyle: Dark')
    expect(infoPlist).toMatch(/<key>UIUserInterfaceStyle<\/key>\s*<string>Dark<\/string>/)
  })

  it('covers the window with a layer of its own before the app mounts', () => {
    const body = html.slice(html.indexOf('<body>'))
    expect(body).toMatch(/<div id="launch" aria-hidden="true">[\s\S]*<\/div>\s*<div id="app"><\/div>/)
    expect(body.indexOf('id="launch"')).toBeLessThan(body.indexOf('id="app"'))
    expect(layer('#launch')).toMatchObject({ position: 'fixed', inset: '0', display: 'grid', 'place-items': 'center' })
    expect(Number(layer('#launch')['z-index'])).toBeGreaterThan(0)
  })

  it('paints the layer in the ground colour of the window', () => {
    const conf = JSON.parse(tauriConf) as { app: { windows: { backgroundColor?: string }[] } }
    expect(layer('#launch').background).toBe(conf.app.windows[0].backgroundColor)
  })

  it('draws the mark of Mark at a unit of eight, still and without a glow', () => {
    expect(layer('#launch .mark')).toMatchObject({ width: `${12 * UNIT}px`, height: `${12 * UNIT}px` })
    for (const name of ['top', 'side', 'cursor']) {
      expect(layer(`#launch .${name}`)).toMatchObject(markCell(name))
    }
    expect(layer('#launch .corner').background).toBe(token('brand'))
    expect(layer('#launch .cursor').background).toBe(token('text'))
    expect(layerStyle).not.toMatch(/box-shadow|animation|transition/)
  })

  it('launches on the same ground as the layer', () => {
    const ground = storyboard.match(/<viewLayoutGuide key="safeArea"[^>]*\/>\s*<color key="backgroundColor"[^>]*\/>/)
    expect(hex(ground?.[0] ?? '')).toBe(layer('#launch').background)
  })

  it('centres the native mark on the whole view, not on the safe area', () => {
    const root = storyboard.match(/<view key="view" [^>]*id="([^"]+)"/)?.[1]
    for (const axis of ['centerX', 'centerY']) {
      expect(constant(storyboard, pin(nativeMark.id, axis, root))).toBe(0)
    }
    const own = nativeMark.after.slice(nativeMark.after.indexOf('</subviews>'))
    expect(constant(own, 'firstAttribute="width"')).toBe(parseFloat(layer('#launch .mark').width))
    expect(constant(own, 'firstAttribute="height"')).toBe(parseFloat(layer('#launch .mark').height))
  })

  it('draws the native mark with the cells and colours of the layer, without an image', () => {
    for (const [label, name] of [['Top', 'top'], ['Side', 'side'], ['Cursor', 'cursor']]) {
      expect(nativeCell(label)).toEqual(cellBox(layer(`#launch .${name}`)))
    }
    const colour = (label: string) =>
      hex(nativeView(label).body.match(/<color key="backgroundColor"[^>]*\/>/)?.[0] ?? '')
    expect(colour('Top')).toBe(layer('#launch .corner').background)
    expect(colour('Side')).toBe(layer('#launch .corner').background)
    expect(colour('Cursor')).toBe(layer('#launch .cursor').background)
    expect(storyboard).not.toMatch(/<image|imageView/)
  })
})
