import { describe, expect, it } from 'vitest'
import html from '../index.html?raw'
import infoPlist from '../src-tauri/gen/apple/cornercase-mobile_iOS/Info.plist?raw'
import storyboard from '../src-tauri/gen/apple/LaunchScreen.storyboard?raw'
import projectYml from '../src-tauri/gen/apple/project.yml?raw'
import tauriConf from '../src-tauri/tauri.conf.json?raw'

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
})
