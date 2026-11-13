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
})
