import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { inflateSync } from 'node:zlib'
import { describe, expect, it } from 'vitest'

const root = join(import.meta.dirname, '..', 'src-tauri')
const icons = join(root, 'icons')
const appiconset = join(root, 'gen/apple/Assets.xcassets/AppIcon.appiconset')
const res = join(root, 'gen/android/app/src/main/res')

type Png = { width: number; height: number; colorType: number; rgba: (x: number, y: number) => number[] }

function idatOf(data: Buffer) {
  const idat: Buffer[] = []
  let offset = 8
  while (offset < data.length) {
    const length = data.readUInt32BE(offset)
    if (data.toString('latin1', offset + 4, offset + 8) === 'IDAT') idat.push(data.subarray(offset + 8, offset + 8 + length))
    offset += 12 + length
  }
  return Buffer.concat(idat)
}

function paeth(a: number, b: number, c: number) {
  const p = a + b - c
  return [a, b, c].reduce((best, v) => (Math.abs(p - v) < Math.abs(p - best) ? v : best))
}

function unfilter(raw: Buffer, stride: number, height: number, channels: number) {
  const pixels = Buffer.alloc(stride * height)
  for (let i = 0; i < stride * height; i++) {
    const y = Math.floor(i / stride)
    const col = i % stride
    const a = col >= channels ? pixels[i - channels] : 0
    const b = y > 0 ? pixels[i - stride] : 0
    const c = col >= channels && y > 0 ? pixels[i - stride - channels] : 0
    const pick = [0, a, b, (a + b) >> 1, paeth(a, b, c)][raw[y * (stride + 1)]]
    pixels[i] = (raw[y * (stride + 1) + 1 + col] + pick) & 255
  }
  return pixels
}

function png(path: string): Png {
  const data = readFileSync(path)
  const width = data.readUInt32BE(16)
  const height = data.readUInt32BE(20)
  const colorType = data[25]
  const channels = { 0: 1, 2: 3, 4: 2, 6: 4 }[colorType]
  if (channels === undefined || data[24] !== 8 || data[28] !== 0) throw new Error(`unsupported png ${path}`)
  const stride = width * channels
  const pixels = unfilter(inflateSync(idatOf(data)), stride, height, channels)
  const rgba = (x: number, y: number) => {
    const at = y * stride + x * channels
    if (channels === 4) return [...pixels.subarray(at, at + 4)]
    if (channels === 3) return [...pixels.subarray(at, at + 3), 255]
    if (channels === 2) return [pixels[at], pixels[at], pixels[at], pixels[at + 1]]
    return [pixels[at], pixels[at], pixels[at], 255]
  }
  return { width, height, colorType, rgba }
}

function near(rgba: number[], rgb: number[]) {
  rgba.slice(0, 3).forEach((channel, i) => expect(Math.abs(channel - rgb[i])).toBeLessThanOrEqual(4))
}

function pixelsOf(image: Png) {
  return Array.from({ length: image.width * image.height }, (_, i) => ({ x: i % image.width, y: Math.floor(i / image.width) }))
}

const svg = (name: string) => readFileSync(join(icons, name), 'utf8')
const shapes = (text: string) =>
  [...text.matchAll(/<path fill="(#[0-9a-f]{6})" d="([^"]*)"\/>/g)].map(([, fill, d]) => ({ fill, d }))

describe('app icon sources', () => {
  it('draws the favicon mark on a full-bleed opaque ground, 48 px per unit', () => {
    const text = svg('icon.svg')
    expect(text).toContain('viewBox="0 0 1024 1024"')
    expect(text).toContain('shape-rendering="crispEdges"')
    expect(text).not.toContain('<!--')
    expect(shapes(text)).toEqual([
      { fill: '#0c0e11', d: 'M0 0h1024v1024H0z' },
      { fill: '#005fff', d: 'M368 224h432v144H368zM224 368h144v432H224z' },
      { fill: '#eceef1', d: 'M512 512h96v240H512z' },
    ])
  })

  it('draws the same mark on a transparent ground, 32 px per unit, for the adaptive foreground', () => {
    const text = svg('icon-foreground.svg')
    expect(text).toContain('viewBox="0 0 1024 1024"')
    expect(text).toContain('shape-rendering="crispEdges"')
    expect(text).not.toContain('<!--')
    expect(shapes(text)).toEqual([
      { fill: '#005fff', d: 'M416 320h288v96H416zM320 416h96v288H320z' },
      { fill: '#eceef1', d: 'M512 512h64v160H512z' },
    ])
  })

  it('is joined by a tauri icon manifest', () => {
    expect(JSON.parse(svg('icon.json'))).toEqual({
      default: 'icon.svg',
      android_fg: 'icon-foreground.svg',
      bg_color: '#0c0e11',
      android_fg_scale: 100,
    })
  })
})

describe('ios icon', () => {
  const files = readdirSync(appiconset).filter((name) => name.endsWith('.png'))

  it('lists every png of the set in Contents.json, 18 of them', () => {
    const contents = JSON.parse(readFileSync(join(appiconset, 'Contents.json'), 'utf8')) as { images: { filename: string }[] }
    expect(files).toHaveLength(18)
    expect(contents.images.map((image) => image.filename).sort()).toEqual([...files].sort())
  })

  it('has no transparency and a 1024 marketing icon', () => {
    for (const name of files) {
      const image = png(join(appiconset, name))
      expect([name, pixelsOf(image).filter(({ x, y }) => image.rgba(x, y)[3] !== 255)]).toEqual([name, []])
    }
    expect(png(join(appiconset, 'AppIcon-512@2x.png')).width).toBe(1024)
  })
})

describe('android icon', () => {
  const densities = ['mdpi', 'hdpi', 'xhdpi', 'xxhdpi', 'xxxhdpi']

  it('is adaptive: the ground colour behind the foreground mipmap', () => {
    const xml = readFileSync(join(res, 'mipmap-anydpi-v26/ic_launcher.xml'), 'utf8')
    expect(xml).toContain('<adaptive-icon')
    expect(xml).toContain('<foreground android:drawable="@mipmap/ic_launcher_foreground"/>')
    expect(xml).toContain('<background android:drawable="@color/ic_launcher_background"/>')
    expect(readFileSync(join(res, 'values/ic_launcher_background.xml'), 'utf8')).toMatch(/>#0c0e11</i)
  })

  it('leaves no template drawable behind', () => {
    const drawables = readdirSync(res).filter((name) => name.startsWith('drawable'))
    for (const dir of drawables) {
      const names = readdirSync(join(res, dir))
      expect(names).not.toContain('ic_launcher_background.xml')
      expect(names).not.toContain('ic_launcher_foreground.xml')
    }
  })

  it('keeps the foreground inside the safe zone: 132 px of 432 from the centre', () => {
    const image = png(join(res, 'mipmap-xxxhdpi/ic_launcher_foreground.png'))
    expect(image.width).toBe(432)
    const seen = pixelsOf(image).filter(({ x, y }) => image.rgba(x, y)[3] !== 0)
    expect(seen.length).toBeGreaterThan(0)
    expect(Math.max(...seen.map(({ x, y }) => Math.hypot(x + 0.5 - 216, y + 0.5 - 216)))).toBeLessThanOrEqual(132)
  })

  it.each(densities)('shows the mark on its ground in the %s classic icons', (density) => {
    for (const name of ['ic_launcher.png', 'ic_launcher_round.png']) {
      const image = png(join(res, `mipmap-${density}`, name))
      const middle = Math.floor(image.width / 2)
      near(image.rgba(middle, Math.floor(image.height * 0.3)), [0x00, 0x5f, 0xff])
      near(image.rgba(Math.floor(image.width * 0.54), Math.floor(image.height * 0.62)), [0xec, 0xee, 0xf1])
    }
  })
})
